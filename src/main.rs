#![no_std]
#![no_main]

// use probe_run to print panic messages
use panic_halt as _;

mod microgroove {
    mod sequencer {
        use heapless::Vec;
        use midi_types::{Channel, Note, Value14, Value7};

        pub const DEFAULT_NOTE_LENGTH_24PPQN_TICKS: u8 = 5;

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
                    velocity: 100.into(),
                    pitch_bend: 0u16.into(),
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
                    .map(|note_num| { Step::new(note_num.into()) });
                Track {
                    active: true,
                    speed: TrackSpeed::Sixteenth,
                    midi_channel: 1.into(),
                    steps: Vec::from_slice(steps.as_slice()).unwrap(),
                }
            }
        }
    }

    // RTIC app module
    #[rtic::app(device = rp_pico::hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0, PIO0_IRQ_1, PIO1_IRQ_0, PIO1_IRQ_1])]
    mod app {
        // hal aliases - turns out we have a big dependency on the hardware 😀
        use rp_pico::{
            hal::{
                Clock,
                Timer,
                Watchdog,
                clocks,
                gpio::{
                    Function,
                    FunctionUart,
                    Pin,
                    Uart,
                    pin::bank0::{Gpio0, Gpio1},
                },
                pac::UART0,
                sio::{
                    self,
                    Sio
                },
                timer::{
                    Alarm,
                    Alarm0,
                    Alarm3,
                    monotonic::Monotonic,
                },
                uart::{
                    DataBits,
                    Reader,
                    StopBits,
                    UartConfig,
                    UartPeripheral,
                    Writer,
                },
            },
            Pins,
            XOSC_CRYSTAL_FREQ,
        };

        // midi stuff
        use midi_types::MidiMessage;
        use embedded_midi::{MidiIn, MidiOut};

        // non-blocking io
        use nb::block;

        // defmt rtt logging (read the logs with cargo embed, etc)
        use defmt;
        use defmt::{info, debug, trace};
        use defmt_rtt as _;

        // alloc-free data structures
        use heapless::{Vec, HistoryBuffer};

        // time manipulation
        use fugit::{ExtU32, MicrosDurationU32, RateExtU32};

        // crate imports
        use super::sequencer::{
            Step,
            Track,
            DEFAULT_NOTE_LENGTH_24PPQN_TICKS
        };

        use rtic::rtic_monotonic::Monotonic as RticMonotonic;

        // monotonic clock for RTIC and defmt
        #[monotonic(binds = TIMER_IRQ_3, default = true)]
        type TimerMonotonic = Monotonic<Alarm3>;
        type TimerMonotonicInstant = <TimerMonotonic as RticMonotonic>::Instant;

        // alias the type for our UART pins
        type MidiInUartPin = Pin<Gpio0, Function<Uart>>;
        type MidiOutUartPin = Pin<Gpio1, Function<Uart>>;
        type MidiUartPins = (MidiInUartPin, MidiOutUartPin);

        // TODO move to UI/input module
        pub enum InputMode {
            Track,
            Groove,
            Melody
        }

        // display update time is the time between each render
        // this is the upper bound for the time to flush each frame to the oled
        // at 40ms, the frame rate will be 25 FPS
        // we want the lowest frame rate that looks acceptable, to provide the largest budget for
        // render times
        // TODO move to UI/input module
        const DISPLAY_UPDATE_INTERVAL_US: MicrosDurationU32 = MicrosDurationU32::millis(40);

        // RTIC shared resources
        #[shared]
        struct Shared {
            // tracks are where we store our sequence data
            tracks: Vec<Track, 16>,

            // are we playing, or not?
            playing: bool,

            input_mode: InputMode,
        }

        // RTIC local resources
        #[local]
        struct Local {
            // midi ports (2 halves of the split UART)
            midi_in: MidiIn<Reader<UART0, MidiUartPins>>,
            midi_out: MidiOut<Writer<UART0, MidiUartPins>>,

            // alarm for firing display updates
            display_update_alarm: Alarm0,

            // a buffer to track the intervals between MIDI ticks, which we can 
            // use to estimate the tempo, we can then use our tempo estimate to 
            // implement note lengths and swing
            tick_history: HistoryBuffer::<u64, 24>,

            // pins for buttons
            // pins for encoders
            // i2c/display interface
        }

        // RTIC init
        #[init]
        fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
            // release spinlocks to avoid a deadlock after soft-reset
            unsafe {
                sio::spinlock_reset();
            }

            // configure source of timestamps for defmt
            defmt::timestamp!("{=u64:us}", {
                monotonics::now().duration_since_epoch().to_micros()
            });

            let playing = false;

            // clock setup for timers and alarms
            let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);
            let clocks = clocks::init_clocks_and_plls(
                XOSC_CRYSTAL_FREQ,
                ctx.device.XOSC,
                ctx.device.CLOCKS,
                ctx.device.PLL_SYS,
                ctx.device.PLL_USB,
                &mut ctx.device.RESETS,
                &mut watchdog,
            )
            .ok()
            .expect("init_clocks_and_plls(...) should succeed");

            let mut timer = Timer::new(ctx.device.TIMER, &mut ctx.device.RESETS);
            
            // create an alarm to update the display regularly
            let mut display_update_alarm = timer.alarm_0().unwrap();
            let _ = display_update_alarm.schedule(DISPLAY_UPDATE_INTERVAL_US);
            display_update_alarm.enable_interrupt();

            // create a monotonic timer for RTIC
            let monotonic_alarm = timer.alarm_3().unwrap();
            let monotonic_timer = Monotonic::new(timer, monotonic_alarm);

            // the single-cycle i/o block controls our gpio pins
            let sio = Sio::new(ctx.device.SIO);

            // set the pins to their default state
            let pins = Pins::new(
                ctx.device.IO_BANK0,
                ctx.device.PADS_BANK0,
                sio.gpio_bank0,
                &mut ctx.device.RESETS,
            );

            // put pins for midi into uart mode
            let midi_uart_pins = (
                // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
                pins.gpio0.into_mode::<FunctionUart>(),
                // UART RX (characters received by RP2040) on pin 2 (GPIO1)
                pins.gpio1.into_mode::<FunctionUart>(),
            );

            // make a uart peripheral on the given pins
            let mut midi_uart = UartPeripheral::new(ctx.device.UART0, midi_uart_pins, &mut ctx.device.RESETS)
                .enable(
                    UartConfig::new(31_250.Hz(), DataBits::Eight, None, StopBits::One),
                    clocks.peripheral_clock.freq(),
                )
                .expect("midi_uart.enable(...) should succeed");

            // configure uart interrupt to fire on midi input
            midi_uart.enable_rx_interrupt();

            let (midi_reader, midi_writer) = midi_uart.split();

            let midi_in = MidiIn::new(midi_reader);
            let midi_out = MidiOut::new(midi_writer);
            
            // create buffer to collect MIDI tick intervals
            let tick_history = HistoryBuffer::<u64, 24>::new();

            // show track page of UI at startup
            let input_mode = InputMode::Track;

            // TODO configure interrupts on button and encoder GPIO pins
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

            (Shared { input_mode, playing, tracks }, Local { midi_in, midi_out, display_update_alarm, tick_history }, init::Monotonics(monotonic_timer))
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

        #[task(priority = 2, shared = [tracks], local = [ticks: u8 = 0, last_tick_instant: Option<TimerMonotonicInstant> = None, tick_history] )]
        fn sequencer_advance(mut ctx: sequencer_advance::Context) {
            debug!("sequencer_advance");

            let sequencer_advance::LocalResources { ticks, last_tick_instant, tick_history } = ctx.local;

            // calculate average interval between last K ticks
            // TODO should move to some impl
            let mut gate_time = 375.millis();
            if let Some(last_tick_instant) = *last_tick_instant {
                let tick_duration = monotonics::now().checked_duration_since(last_tick_instant).unwrap();
                tick_history.write(tick_duration.to_micros());
                let avg_tick_duration = tick_history.as_slice().iter().sum::<u64>() / tick_history.len() as u64;
                let gate_time: MicrosDurationU32 = ((avg_tick_duration * DEFAULT_NOTE_LENGTH_24PPQN_TICKS as u64) as u32).micros();
            }

            // TODO should all move out of task, e.g. Tracks::advance()?
            ctx.shared.tracks.lock(|tracks| {
                for track in tracks.as_slice() {
                    let step: &Step = &track.steps[*ticks as usize];
                    let note_on_message = MidiMessage::NoteOn(track.midi_channel, step.note, step.velocity);
                    midi_send::spawn(note_on_message).unwrap();

                    let note_off_message = MidiMessage::NoteOff(track.midi_channel, step.note, 0.into());
                    midi_send::spawn_after(gate_time.into(), note_off_message).unwrap();
                }
            });
        }

        // timer task to render UI at k FPS
        #[task(binds = TIMER_IRQ_0, priority = 1, local = [display_update_alarm, display_ticks: u32 = 0])]
        fn timer_irq_0(ctx: timer_irq_0::Context) {
            *ctx.local.display_ticks += 1;
            debug!("rendering ui, display_ticks={}", ctx.local.display_ticks);

            ctx.local.display_update_alarm.clear_interrupt();
            let _ = ctx.local.display_update_alarm.schedule(DISPLAY_UPDATE_INTERVAL_US);
        }

        // idle task needed in debug mode, default RTIC idle task calls wfi(), which breaks rtt
        #[idle]
        fn task_main(_: task_main::Context) -> ! {
            loop {
                cortex_m::asm::nop();
            }
        }
    }
}
