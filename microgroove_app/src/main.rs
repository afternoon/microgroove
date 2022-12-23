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
    use core::fmt::Write;
    use defmt::{self, debug, error, info, trace};
    use defmt_rtt as _;
    use fugit::MicrosDurationU64;
    use heapless::{String, Vec};
    use midi_types::MidiMessage;
    use nb::block;
    use rp_pico::hal::{
        gpio::Interrupt::EdgeLow,
        timer::{monotonic::Monotonic, Alarm0},
    };

    use crate::{
        display::{self, PerformView},
        encoder::encoder_array::EncoderArray,
        input::{self, InputMode},
        midi,
        peripherals::{
            setup, ButtonRhythmPin, ButtonMelodyPin, ButtonTrackPin, Display, MidiIn, MidiOut,
        },
    };
    use microgroove_sequencer::{
        Track, TRACK_COUNT,
        machine_resources::MachineResources,
        sequence_generator::SequenceGenerator,
        sequencer::{ScheduledMidiMessage, Sequencer}, param::ParamList,
    };

    #[global_allocator]
    static ALLOCATOR: CortexMHeap = CortexMHeap::empty();
    const HEAP_SIZE_BYTES: usize = 8 * 1024;

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
        current_track: u8,

        /// Sequencer big-ball-of-state
        sequencer: Sequencer,

        /// Current page of the UI.
        input_mode: InputMode,

        // set of SequenceGenerators, one for each `Track` in `Sequencer`
        sequence_generators: Vec<SequenceGenerator, TRACK_COUNT>,
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

        /// Pin for button the [RHYTHM] button
        button_rhythm_pin: ButtonRhythmPin,

        /// Pin for button the [MELODY] button
        button_melody_pin: ButtonMelodyPin,

        // encoders
        encoders: EncoderArray,

        // context object for machines to use in sequence generation
        machine_resources: MachineResources,
    }

    /// RTIC init method sets up the hardware and initialises shared and local resources.
    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        info!("[init] hello world!");

        // initialise allocator for dynamic structures (machines, params, etc)
        unsafe {
            ALLOCATOR.init(cortex_m_rt::heap_start() as usize, HEAP_SIZE_BYTES);
                debug!("[init] heap_start={} heap_size_bytes={}", cortex_m_rt::heap_start() as usize, HEAP_SIZE_BYTES);
        }

        // configure RTIC monotonic as source of timestamps for defmt
        defmt::timestamp!("{=u64:us}", {
            monotonics::now().duration_since_epoch().to_micros()
        });

        // create a device wrapper instance and grab some of the peripherals we need
        let (midi_in, midi_out, mut display, buttons, encoders, rosc, monotonic_timer) =
            setup(ctx.device);
        let (button_track_pin, button_rhythm_pin, button_melody_pin) = buttons;

        // create a vec of `SequenceGenerator`s, we'll use these to generate sequences for our
        // tracks.
        let mut sequence_generators: Vec<SequenceGenerator, TRACK_COUNT> = Vec::new();
        for _i in 0..TRACK_COUNT {
            sequence_generators.push(SequenceGenerator::default()).unwrap();
        }

        // create a new sequencer and build the first track
        let mut sequencer = Sequencer::default();
        let generator = SequenceGenerator::default();
        let mut machine_resources = MachineResources::new(rosc);
        let mut new_track = Track::default();
        new_track.sequence = generator.generate(new_track.length, &mut machine_resources);
        sequencer.enable_track(0, new_track);

        // show a splash screen for a bit
        display::render_splash_screen_view(&mut display).unwrap();

        // start scheduled task to read encoders
        read_encoders::spawn().expect("read_encoders::spawn should succeed");

        // start scheduled task to update display
        update_display::spawn().expect("update_display::spawn should succeed");

        info!("[init] complete 🤘");

        (
            Shared {
                input_mode: Default::default(),
                current_track: 0,
                sequencer,
                sequence_generators,
            },
            Local {
                midi_in,
                midi_out,
                display,
                button_track_pin,
                button_rhythm_pin,
                button_melody_pin,
                encoders,
                machine_resources,
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
        let start = monotonics::now();
        trace!("[uart0_irq] start");

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

        trace!("[uart0_irq] elapsed_time={}", (monotonics::now() - start).to_micros());
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
        local = [button_track_pin, button_rhythm_pin, button_melody_pin]
    )]
    fn io_irq_bank0(mut ctx: io_irq_bank0::Context) {
        let start = monotonics::now();
        trace!("[io_irq_bank0] start");

        // for each button, check interrupt status to see if we fired
        if ctx.local.button_track_pin.interrupt_status(EdgeLow) {
            info!("[TRACK] pressed");
            ctx.shared.input_mode.lock(|input_mode| {
                *input_mode = match *input_mode {
                    InputMode::Track => InputMode::Global,
                    _ => InputMode::Track
                }
            });
            ctx.local.button_track_pin.clear_interrupt(EdgeLow);
        }
        if ctx.local.button_rhythm_pin.interrupt_status(EdgeLow) {
            info!("[RHYTHM] pressed");
            ctx.shared.input_mode.lock(|input_mode| {
                *input_mode = match *input_mode {
                    InputMode::Rhythm => InputMode::Groove,
                    _ => InputMode::Rhythm
                }
            });
            ctx.local.button_rhythm_pin.clear_interrupt(EdgeLow);
        }
        if ctx.local.button_melody_pin.interrupt_status(EdgeLow) {
            info!("[MELODY] pressed");
            ctx.shared.input_mode.lock(|input_mode| {
                *input_mode = match *input_mode {
                    InputMode::Melody => InputMode::Harmony,
                    _ => InputMode::Melody
                }
            });
            ctx.local.button_melody_pin.clear_interrupt(EdgeLow);
        }

        trace!("[io_irq_bank0] elapsed_time={}", (monotonics::now() - start).to_micros());
    }

    /// Check encoders for position changes.
    /// Reading every 1ms removes some of the noise vs reading on each interrupt.
    #[task(
        priority = 4,
        shared = [input_mode, current_track, sequencer, sequence_generators],
        local = [encoders, machine_resources],
    )]
    fn read_encoders(ctx: read_encoders::Context) {
        let start = monotonics::now();
        trace!("[read_encoders] start");

        if let Some(_changes) = ctx.local.encoders.update() {
            (ctx.shared.input_mode, ctx.shared.current_track, ctx.shared.sequencer, ctx.shared.sequence_generators).lock(|input_mode, current_track, sequencer, sequence_generators| {
                input::apply_encoder_values(
                    ctx.local.encoders.take_values(),
                    *input_mode,
                    current_track,
                    sequencer,
                    sequence_generators,
                    ctx.local.machine_resources,
                );
            })
        }

        // read again in 1ms
        read_encoders::spawn_after(ENCODER_READ_INTERVAL).unwrap();

        trace!("[read_encoders] elapsed_time={}", (monotonics::now() - start).to_micros());
    }

    /// Update the display by rendering a view object. This method creates an instance of a view,
    /// passing in relevant data (which is copied). The view then takes care of rendering to the
    /// display. Rendering is time-consuming, because writing data across I2C is slow. Hence the
    /// work is offloaded to the `render_view` task, unlocking shared resources and allowing other
    /// tasks to interrupt the rendering.
    #[task(
        priority = 1,
        shared = [input_mode, current_track, sequencer, sequence_generators],
    )]
    fn update_display(ctx: update_display::Context) {
        let start = monotonics::now();
        trace!("[update_display] start");

        (ctx.shared.input_mode, ctx.shared.current_track, ctx.shared.sequencer, ctx.shared.sequence_generators).lock(|input_mode, current_track, sequencer, sequence_generators| {
            let maybe_track = sequencer.tracks.get_mut(*current_track as usize).unwrap().as_mut();
            let view = match maybe_track {
                Some(track) => {
                    let sequence = Some(track.sequence.clone());
                    let active_step_num = Some(track.step_num(sequencer.tick));
                    let generator = sequence_generators.get(*current_track as usize).unwrap();
                    let machine_name = match input_mode {
                        InputMode::Rhythm => Some(String::<10>::from(generator.rhythm_machine.name())),
                        InputMode::Melody => Some(String::<10>::from(generator.melody_machine.name())),
                        _ => None,
                    };
                    let global_params = ParamList::new();
                    let params = match input_mode {
                        InputMode::Track => track.params(),
                        InputMode::Global => &global_params, // TODO
                        InputMode::Rhythm => generator.rhythm_machine.params(),
                        InputMode::Groove => generator.groove_params(),
                        InputMode::Melody => generator.melody_machine.params(),
                        InputMode::Harmony => generator.harmony_params(),
                    };
                    let param_data = Some(params.iter().map(|param| {
                        let mut value_string = String::new();
                        write!(value_string, "{}", param.value()).unwrap();
                        (String::<6>::from(param.name()), value_string)
                    }).collect());
                    PerformView {
                        input_mode: *input_mode,
                        playing: sequencer.is_playing(),
                        sequence,
                        track_num: *current_track,
                        active_step_num,
                        machine_name,
                        param_data,
                    }
                }
                None => PerformView {
                    input_mode: *input_mode,
                    playing: sequencer.is_playing(),
                    sequence: None,
                    track_num: *current_track,
                    active_step_num: None,
                    machine_name: None,
                    param_data: None,
                }
            };

            render_view::spawn(view)
                .expect("should be able to spawn_after display_update");

        });

        update_display::spawn_after(DISPLAY_UPDATE_INTERVAL)
            .expect("should be able to spawn_after update_display");

        trace!("[update_display] elapsed_time={}", (monotonics::now() - start).to_micros());
    }

    #[task(
        priority = 1,
        local = [display]
    )]
    fn render_view(ctx: render_view::Context, view: PerformView) {
        let start = monotonics::now();
        trace!("[render_view] start");

        if let Err(_) = view.render(ctx.local.display) {
            error!("PerformView::render error");
        }

        trace!("[render_view] elapsed_time={}", (monotonics::now() - start).to_micros());
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
        panic!("TICK TICK TICK TICK OOM!");
    }
}
