#![no_std]
#![no_main]

// use probe_run to print panic messages
use panic_halt as _;

mod microgroove {
    mod sequencer {
        use heapless::Vec;
        use midi_types::{Channel, Note, Value14, Value7};

        #[derive(Clone, Debug)]
        pub struct Step {
            pub active: bool,
            pub note: Note,
            pub velocity: Value7,
            pub pitch_bend: Value14,
        }

        impl Step {
            pub fn new(note: Note) -> Step {
                Step {
                    active: true,
                    note,
                    velocity: Value7::from(100),
                    pitch_bend: Value14::from(0u16),
                }
            }
        }

        #[derive(Debug)]
        pub enum TrackSpeed {
            ThirtySecond = 3,
            Sixteenth = 6,
            Eigth = 12,
            Quarter = 24,
            Whole = 96
        }

        pub type Sequence = Vec<Step, 32>;

        #[derive(Debug)]
        pub struct Track {
            pub active: bool,
            pub speed: TrackSpeed,
            pub midi_channel: Channel,
            pub steps: Sequence,
        }

        impl Track {
            pub fn new() -> Track {
                let steps = [57, 59, 60, 62, 64, 65, 67, 69, 57, 59, 60, 62, 64, 65, 67, 69]
                    .map(|note_num| { Step::new(Note::from(note_num)) });
                Track {
                    active: true,
                    speed: TrackSpeed::Sixteenth,
                    midi_channel: Channel::from(1),
                    steps: Vec::from_slice(steps.as_slice()).unwrap(),
                }
            }
        }
    }

    // RTIC app module
    #[rtic::app(device = rp_pico::hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0, PIO0_IRQ_1, PIO1_IRQ_0, PIO1_IRQ_1])]
    mod app {
        // hal aliases
        use rp_pico::hal::{
            self,
            Clock,
            gpio::{
                self,
                pin::bank0::{Gpio0, Gpio1},
            },
            timer::{
                self,
                monotonic,
                Alarm,
            },
            uart,
        };

        // midi type traits
        use midi_types::MidiMessage;

        // non-blocking io
        use nb::block;

        // defmt rtt logging (read the logs with cargo embed, etc)
        use defmt;
        use defmt::{info, debug, trace};
        use defmt_rtt as _;

        // alloc-free data structures
        use heapless::Vec;

        // crate imports
        use crate::microgroove::sequencer::Track;

        // monotonic clock for RTIC and defmt
        #[monotonic(binds = TIMER_IRQ_3, default = true)]
        type DatMonotonicTimerYo = monotonic::Monotonic<timer::Alarm3>;

        // alias the type for our UART pins
        type MidiInUartPin = gpio::Pin<Gpio0, gpio::Function<gpio::Uart>>;
        type MidiOutUartPin = gpio::Pin<Gpio1, gpio::Function<gpio::Uart>>;
        type MidiUartPins = (MidiInUartPin, MidiOutUartPin);

        // display update time is the time between each render
        // this is the upper bound for the time to flush each frame to the oled
        // at 40ms, the frame rate will be 25 FPS
        // we want the lowest frame rate that looks acceptable, to provide the largest budget for
        // render times
        const DISPLAY_UPDATE_INTERVAL_US: fugit::MicrosDurationU32 = fugit::MicrosDurationU32::millis(40);

        // RTIC shared resources
        #[shared]
        struct Shared {
            // tracks are where we store our sequence data
            tracks: Vec<Track, 16>,

            // are we playing, or not?
            playing: bool,

            // pins for buttons
            // pins for encoders
            // i2c/display interface
        }

        // RTIC local resources
        #[local]
        struct Local {
            // midi ports (2 halves of the split UART)
            midi_in: embedded_midi::MidiIn<uart::Reader<hal::pac::UART0, MidiUartPins>>,
            midi_out: embedded_midi::MidiOut<uart::Writer<hal::pac::UART0, MidiUartPins>>,

            // alarm for firing display updates
            display_update_alarm: timer::Alarm0,
        }

        // RTIC init
        #[init]
        fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
            // release spinlocks to avoid a deadlock after soft-reset
            unsafe {
                hal::sio::spinlock_reset();
            }

            // configure source of timestamps for defmt
            defmt::timestamp!("{=u64:us}", {
                monotonics::now().duration_since_epoch().to_micros()
            });

            let playing = false;

            // clock setup for timers and alarms
            let mut watchdog = hal::Watchdog::new(ctx.device.WATCHDOG);
            let clocks = hal::clocks::init_clocks_and_plls(
                rp_pico::XOSC_CRYSTAL_FREQ,
                ctx.device.XOSC,
                ctx.device.CLOCKS,
                ctx.device.PLL_SYS,
                ctx.device.PLL_USB,
                &mut ctx.device.RESETS,
                &mut watchdog,
            )
            .ok()
            .expect("init_clocks_and_plls(...) should succeed");

            let mut timer = hal::Timer::new(ctx.device.TIMER, &mut ctx.device.RESETS);
            
            // create an alarm to update the display regularly
            let mut display_update_alarm = timer.alarm_0().unwrap();
            let _ = display_update_alarm.schedule(DISPLAY_UPDATE_INTERVAL_US);
            display_update_alarm.enable_interrupt();

            // create a monotonic timer for RTIC
            let monotonic_alarm = timer.alarm_3().unwrap();
            let monotonic_timer = timer::monotonic::Monotonic::new(timer, monotonic_alarm);

            // the single-cycle i/o block controls our gpio pins
            let sio = hal::Sio::new(ctx.device.SIO);

            // set the pins to their default state
            let pins = rp_pico::Pins::new(
                ctx.device.IO_BANK0,
                ctx.device.PADS_BANK0,
                sio.gpio_bank0,
                &mut ctx.device.RESETS,
            );

            // put pins for midi into uart mode
            let midi_uart_pins = (
                // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
                pins.gpio0.into_mode::<gpio::FunctionUart>(),
                // UART RX (characters received by RP2040) on pin 2 (GPIO1)
                pins.gpio1.into_mode::<gpio::FunctionUart>(),
            );

            // configure uart comms settings
            let mut uart_config_31250_8_N_1 = uart::common_configs::_38400_8_N_1;
            uart_config_31250_8_N_1.baudrate = fugit::HertzU32::from_raw(31250);

            // make a uart peripheral on the given pins
            let mut midi_uart = uart::UartPeripheral::new(ctx.device.UART0, midi_uart_pins, &mut ctx.device.RESETS)
                .enable(
                    uart_config_31250_8_N_1,
                    clocks.peripheral_clock.freq(),
                )
                .expect("midi_uart.enable(...) should succeed");

            // configure uart interrupt to fire on midi input
            midi_uart.enable_rx_interrupt();

            let (midi_reader, midi_writer) = midi_uart.split();

            let midi_in = embedded_midi::MidiIn::new(midi_reader);
            let midi_out = embedded_midi::MidiOut::new(midi_writer);

            // TODO configure interrupts on button and encoder gpio pins
            // let mut buttons = Vec::<InputPin, 4>::new();
            // for button in buttons {
            //     button.set_interrupt_enabled(EdgeLow, true);
            // }

            // TODO init display

            // create some tracks!
            let mut tracks = Vec::new();
            for _ in 0..16 {
                tracks.push(Track::new()).unwrap();
            }

            info!("init complete");

            (Shared { playing, tracks }, Local { midi_in, midi_out, display_update_alarm }, init::Monotonics(monotonic_timer))
        }

        #[task(binds = IO_IRQ_BANK0, priority = 4)]
        fn io_irq_bank0(_ctx: io_irq_bank0::Context) {
            trace!("a wild gpio_bank0 interrupt has fired!");

            // TODO for each pin, check pin.interrupt_status(EdgeLow) to see if we fired
            // TODO for button presses, trigger an input mode change
            // TODO for encoders, trigger a param change
            // TODO clear_interrupt(EdgeLow)
        }

        #[task(binds = UART0_IRQ, priority = 4, shared = [playing], local = [midi_in])]
        fn uart0_irq(mut ctx: uart0_irq::Context) {
            // check midi input for messages
            trace!("a wild uart0 interrupt has fired!");

            // read those sweet sweet midi bytes!
            if let Some(message) = block!(ctx.local.midi_in.read()).ok() {
                // log the message
                match message {
                    MidiMessage::TimingClock => {
                        debug!("got midi message: TimingClock");

                        // if clock, spawn task to tick tracks and potentially generate midi output
                        ctx.shared.playing.lock(|playing| {
                            if *playing {
                                sequencer_advance::spawn().expect("sequencer_advance::spawn() should succeed");
                            }
                        });
                    }
                    MidiMessage::Start => {
                        debug!("got midi message: start");
                        ctx.shared.playing.lock(|playing| {
                            *playing = true;
                        });
                    }
                    MidiMessage::Stop => {
                        debug!("got midi message: stop");
                        ctx.shared.playing.lock(|playing| {
                            *playing = false;
                        });
                    }
                    MidiMessage::Continue => {
                        debug!("got midi message: continue");
                        ctx.shared.playing.lock(|playing| {
                            *playing = true;
                        });
                    }
                    _ => debug!("got midi message: UNKNOWN"),
                }

                // pass received message to midi out ("soft thru")
                midi_send::spawn(message).expect("midi_send::spawn(message) should succeed");
            }

            // TODO clear interrupt??
        }

        #[task(priority = 3, capacity = 50, local = [midi_out])]
        fn midi_send(ctx: midi_send::Context, message: MidiMessage) {
            debug!("midi_send");
            ctx.local.midi_out.write(&message)
                .ok()
                .expect("midi_out.write(message) should succeed");
        }

        #[task(priority = 2, shared = [tracks], local = [ticks: u8 = 0])]
        fn sequencer_advance(mut ctx: sequencer_advance::Context) {
            debug!("sequencer_advance");

            // TODO should all move to Tracks::advance()?
            ctx.shared.tracks.lock(|tracks| {
                for track in tracks.as_slice() {
                    let step = &track.steps[*ctx.local.ticks as usize];
                    let message = MidiMessage::NoteOn(track.midi_channel, step.note, step.velocity);
                    midi_send::spawn(message).unwrap();
                }
            });

            *ctx.local.ticks += 1;
            if *ctx.local.ticks > 95 { *ctx.local.ticks = 0}
        }

        // timer task to render UI at k FPS
        #[task(binds = TIMER_IRQ_0, priority = 1, local = [display_update_alarm, ticks: u32 = 0])]
        fn timer_irq_0(ctx: timer_irq_0::Context) {
            *ctx.local.ticks += 1;
            debug!("rendering ui, ticks={}", ctx.local.ticks);

            ctx.local.display_update_alarm.clear_interrupt();
            let _ = ctx.local.display_update_alarm.schedule(DISPLAY_UPDATE_INTERVAL_US);
        }

        // idle task needed in debug mode, default RTIC idle task calls wfi(), which breaks rtt
        #[idle]
        fn taskmain(_: taskmain::Context) -> ! {
            loop {
                cortex_m::asm::nop();
            }
        }
    }
}
