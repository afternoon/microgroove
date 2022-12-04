#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

mod display;
mod encoder;
mod input;
mod midi;
mod peripherals;

use panic_probe as _;

// RTIC app module runs the app as a set of concurrent tasks modifying shared state
// this module is responsible for interfacing with the hardware
#[rtic::app(
    device = rp_pico::hal::pac,
    peripherals = true,
    dispatchers = [USBCTRL_IRQ, DMA_IRQ_0, DMA_IRQ_1, PWM_IRQ_WRAP]
)]
mod app {
    use alloc_cortex_m::CortexMHeap;
    use defmt::{self, error, info, trace};
    use defmt_rtt as _;
    use fugit::MicrosDurationU64;
    use midi_types::MidiMessage;
    use nb::block;
    use rp_pico::hal::{
        gpio::Interrupt::EdgeLow,
        timer::{monotonic::Monotonic, Alarm0},
    };

    use crate::{
        display,
        encoder::encoder_array::EncoderArray,
        input::{self, InputMode},
        midi,
        peripherals::{
            setup, ButtonGroovePin, ButtonMelodyPin, ButtonTrackPin, Display, MidiIn, MidiOut,
        },
    };
    use microgroove_sequencer::sequencer::{ScheduledMidiMessage, Sequencer};

    #[global_allocator]
    static ALLOCATOR: CortexMHeap = CortexMHeap::empty();
    const HEAP_SIZE_BYTES: usize = 16 * 1024; // 16KB!

    // time between each display render
    // this is the practical upper bound for drawing and flushing a frame to the oled
    // at 40ms, the frame rate will be 25 FPS
    // we want the lowest frame rate that looks acceptable, to provide the largest budget for
    // render times
    const DISPLAY_UPDATE_INTERVAL: MicrosDurationU64 = MicrosDurationU64::millis(40);

    // how often to poll encoders for position updates
    const ENCODER_READ_INTERVAL: MicrosDurationU64 = MicrosDurationU64::millis(2);

    /// Define RTIC monotonic timer. Also used for defmt.
    #[monotonic(binds = TIMER_IRQ_0, default = true)]
    type TimerMonotonic = Monotonic<Alarm0>;

    /// RTIC shared resources.
    #[shared]
    struct Shared {
        /// Sequencer big-ball-of-state
        sequencer: Sequencer,

        /// Current page of the UI.
        input_mode: InputMode,
    }

    /// RTIC local resources.
    #[local]
    struct Local {
        /// MIDI input port (1 half of the split UART).
        midi_in: MidiIn,

        /// MIDI output port (1 half of the split UART).
        midi_out: MidiOut,

        /// Interface to the display.
        display: Display,

        /// Pin for button the [TRACK] button
        button_track_pin: ButtonTrackPin,

        /// Pin for button the [GROOVE] button
        button_groove_pin: ButtonGroovePin,

        /// Pin for button the [MELODY] button
        button_melody_pin: ButtonMelodyPin,

        // encoders
        encoders: EncoderArray,
    }

    /// RTIC init method sets up the hardware and initialises shared and local resources.
    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        info!("[init] hello world!");

        // initialise allocator for dynamic structures (machines, params, etc)
        unsafe { ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE_BYTES) }

        // configure RTIC monotonic as source of timestamps for defmt
        defmt::timestamp!("{=u64:us}", {
            monotonics::now().duration_since_epoch().to_micros()
        });

        // create a device wrapper instance and grab some of the peripherals we need
        let (midi_in, midi_out, mut display, buttons, encoders, monotonic_timer) =
            setup(ctx.device);
        let (button_track_pin, button_groove_pin, button_melody_pin) = buttons;

        // show a splash screen for a bit
        display::render_splash_screen_view(&mut display).unwrap();

        // start scheduled task to read encoders
        read_encoders::spawn().expect("read_encoders::spawn should succeed");

        // start scheduled task to update display
        render_perform_view::spawn().expect("render_perform_view::spawn should succeed");

        info!("[init] complete 🤘");

        (
            Shared {
                input_mode: InputMode::Track,
                sequencer: Sequencer::new(),
            },
            Local {
                midi_in,
                midi_out,
                display,
                button_track_pin,
                button_groove_pin,
                button_melody_pin,
                encoders,
            },
            init::Monotonics(monotonic_timer),
        )
    }

    /// Handle MIDI input. Triggered by a byte being received on UART0.
    #[task(
        binds = UART0_IRQ,
        priority = 4,
        shared = [sequencer],
        local = [midi_in]
    )]
    fn uart0_irq(mut ctx: uart0_irq::Context) {
        // read those sweet sweet midi bytes!
        // TODO do we need the block! here?
        if let Ok(message) = block!(ctx.local.midi_in.read()) {
            ctx.shared.sequencer.lock(|sequencer| match message {
                MidiMessage::TimingClock => {
                    trace!("[midi] clock");
                    let now_us = monotonics::now().duration_since_epoch().to_micros();
                    let messages = sequencer.advance(now_us);
                    for message in messages {
                        match message {
                            ScheduledMidiMessage::Immediate(message) => {
                                if let Err(_err) = midi_send::spawn(message) {
                                    error!("could not spawn midi_send for immediate message")
                                }
                            }
                            ScheduledMidiMessage::Delayed(message, delay) => {
                                if let Err(_err) = midi_send::spawn_after(delay, message) {
                                    error!("could not spawn midi_send for delayed message")
                                }
                            }
                        }
                    }
                }
                MidiMessage::Start => {
                    info!("[midi] start");
                    sequencer.start_playing();
                }
                MidiMessage::Stop => {
                    info!("[midi] stop");
                    sequencer.stop_playing();
                }
                MidiMessage::Continue => {
                    info!("[midi] continue");
                    sequencer.continue_playing();
                }
                _ => trace!("[midi] UNKNOWN"),
            });

            // pass received message to midi out ("soft thru")
            match midi_send::spawn(message) {
                Ok(_) => (),
                Err(_) => error!("could not spawn midi_send to pass through message"),
            }
        }
    }

    /// Send a MIDI message. Implemented as a task to allow cooperative multitasking with
    /// higher-pri tasks.
    #[task(
        priority = 3,
        capacity = 64,
        local = [midi_out]
    )]
    fn midi_send(ctx: midi_send::Context, message: MidiMessage) {
        trace!("midi_send");
        midi::log_message(&message);
        ctx.local
            .midi_out
            .write(&message)
            .expect("midi_out.write(message) should succeed");
    }

    /// Handle interrupts caused by button presses and update the `input_mode` shared resource.
    #[task(
        binds = IO_IRQ_BANK0,
        priority = 4,
        shared = [input_mode],
        local = [button_track_pin, button_groove_pin, button_melody_pin]
    )]
    fn io_irq_bank0(mut ctx: io_irq_bank0::Context) {
        trace!("a wild gpio_bank0 interrupt has fired!");

        // for each button, check interrupt status to see if we fired
        if ctx.local.button_track_pin.interrupt_status(EdgeLow) {
            info!("[TRACK] pressed");
            ctx.shared.input_mode.lock(|input_mode| {
                *input_mode = InputMode::Track;
            });
            ctx.local.button_track_pin.clear_interrupt(EdgeLow);
        }
        if ctx.local.button_groove_pin.interrupt_status(EdgeLow) {
            info!("[GROOVE] pressed");
            ctx.shared.input_mode.lock(|input_mode| {
                *input_mode = InputMode::Groove;
            });
            ctx.local.button_groove_pin.clear_interrupt(EdgeLow);
        }
        if ctx.local.button_melody_pin.interrupt_status(EdgeLow) {
            info!("[MELODY] pressed");
            ctx.shared.input_mode.lock(|input_mode| {
                *input_mode = InputMode::Melody;
            });
            ctx.local.button_melody_pin.clear_interrupt(EdgeLow);
        }
    }

    /// Check encoders for position changes.
    /// Reading every 1ms removes some of the noise vs reading on each interrupt.
    #[task(
        priority = 4,
        shared = [input_mode, sequencer],
        local = [encoders],
    )]
    fn read_encoders(ctx: read_encoders::Context) {
        if let Some(_changes) = ctx.local.encoders.update() {
            (ctx.shared.input_mode, ctx.shared.sequencer).lock(|input_mode, sequencer| {
                input::map_encoder_input(*input_mode, sequencer, ctx.local.encoders.take_values());
            })
        }

        // read again in 1ms
        read_encoders::spawn_after(ENCODER_READ_INTERVAL).unwrap();
    }

    // TODO we're locking all the shared state here, which blocks other tasks using that
    // state from running. Does this create a performance issue?
    #[task(
        priority = 1,
        shared = [input_mode, sequencer],
        local = [display]
    )]
    fn render_perform_view(ctx: render_perform_view::Context) {
        (ctx.shared.input_mode, ctx.shared.sequencer).lock(|input_mode, sequencer| {
            let track = sequencer.current_track();
            display::render_perform_view(
                ctx.local.display,
                *input_mode,
                sequencer.is_playing(),
                track,
                sequencer.current_track_num(),
                sequencer.current_track_active_step_num(),
            )
            .unwrap();
        });

        render_perform_view::spawn_after(DISPLAY_UPDATE_INTERVAL)
            .expect("should be able to spawn_after display_update");
    }

    // idle task needed because default RTIC idle task calls wfi(), which breaks rtt
    // TODO disable in release mode
    #[idle]
    fn task_main(_: task_main::Context) -> ! {
        loop {
            cortex_m::asm::nop();
        }
    }

    // OOM handler
    #[alloc_error_handler]
    fn alloc_error(_layout: core::alloc::Layout) -> ! {
        error!("TICK TICK TICK TICK OOM!");
        cortex_m::asm::bkpt();
        loop {}
    }
}
