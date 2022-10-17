#![no_std]
#![no_main]

// use probe_run to print panic messages
use panic_halt as _;

mod microgroove {
    mod sequencer {
        use heapless::Vec;
        use midi_types::{Channel, Note, Value14, Value7};

        pub const DEFAULT_NOTE_LENGTH_24PPQN_TICKS: u8 = 5;

        // type for note lengths, 256ths of a whole note
        type NoteLength = u32;

        // represent a step in a musical sequence
        // TODO polyphonic steps - note: Note -> notes: Vec<Note, _>
        #[derive(Clone, Debug)]
        pub struct Step {
            pub note: Note,
            pub velocity: Value7,
            pub pitch_bend: Value14,
            pub length: NoteLength,
        }

        impl Step {
            pub fn new(note: Note) -> Step {
                Step {
                    note,
                    velocity: 100.into(),
                    pitch_bend: 0u16.into(),
                    length: 13,
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

        pub type Sequence = Vec<Option<Step>, 32>;

        #[derive(Debug)]
        pub struct Track {
            pub speed: TrackSpeed,
            pub midi_channel: Channel,
            pub steps: Sequence,
        }

        impl Track {
            pub fn new() -> Track {
                let steps = [57, 59, 60, 62, 64, 65, 67, 69, 57, 59, 60, 62, 64, 65, 67, 69]
                    .map(|note_num| { Some(Step::new(note_num.into())) });
                Track {
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
                I2C,
                Timer,
                Watchdog,
                clocks,
                gpio::{
                    Function,
                    FunctionI2C,
                    FunctionUart,
                    Pin,
                    Uart,
                    pin::bank0::{
                        Gpio0,
                        Gpio1,
                        Gpio26,
                        Gpio27,
                    },
                },
                pac::{
                    I2C1,
                    UART0,
                },
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

        // ssd1306 oled display driver
        use ssd1306::{
            I2CDisplayInterface,
            Ssd1306,
            prelude::*,
            mode::BufferedGraphicsMode,
        };

        // graphics
        use embedded_graphics::{
            mono_font::{
                MonoTextStyle,
                MonoTextStyleBuilder,
                ascii::FONT_4X6,
            },
            pixelcolor::BinaryColor,
            prelude::*,
            text::{Baseline, Text},
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
        use heapless::{Vec, HistoryBuffer, String};

        // Write trait to allow formatting heapless Strings
        use core::fmt::Write;

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

        // type alias for our UART pins
        type MidiInUartPin = Pin<Gpio0, FunctionUart>;
        type MidiOutUartPin = Pin<Gpio1, FunctionUart>;
        type MidiUartPins = (MidiInUartPin, MidiOutUartPin);

        // type alias for our display pins
        type DisplaySdaPin = Pin<Gpio26, FunctionI2C>;
        type DisplaySclPin = Pin<Gpio27, FunctionI2C>;
        type DisplayPins = (DisplaySdaPin, DisplaySclPin);

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
            tracks: Vec<Option<Track>, 16>,

            // are we playing, or not?
            playing: bool,

            // current page of the UI
            input_mode: InputMode,
        }

        // RTIC local resources
        #[local]
        struct Local {
            // midi ports (2 halves of the split UART)
            midi_in: MidiIn<Reader<UART0, MidiUartPins>>,
            midi_out: MidiOut<Writer<UART0, MidiUartPins>>,

            // display interface
            display: Ssd1306<
                I2CInterface<I2C<I2C1, DisplayPins>>,
                DisplaySize128x64,
                BufferedGraphicsMode<DisplaySize128x64>
            >,

            // alarm for firing display updates
            display_update_alarm: Alarm0,

            // a buffer to track the intervals between MIDI ticks, which we can 
            // use to estimate the tempo, we can then use our tempo estimate to 
            // implement note lengths and swing
            tick_history: HistoryBuffer::<u64, 24>,

            // pins for buttons
            // pins for encoders
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

            // timer for, well, timing
            let mut timer = Timer::new(ctx.device.TIMER, &mut ctx.device.RESETS);

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

            // DISPLAY

            // configure i2c pins
            let sda_pin = pins.gpio26.into_mode::<FunctionI2C>();
            let scl_pin = pins.gpio27.into_mode::<FunctionI2C>();

            // create i2c driver
            let i2c = I2C::i2c1(
                ctx.device.I2C1,
                sda_pin,
                scl_pin,
                1.MHz(),
                &mut ctx.device.RESETS,
                &clocks.peripheral_clock,
            );

            // create i2c display interface
            let mut display = Ssd1306::new(
                I2CDisplayInterface::new_alternate_address(i2c),
                DisplaySize128x64,
                DisplayRotation::Rotate0
            ).into_buffered_graphics_mode();

            // intialise display
            display.init().unwrap();

            debug!("init: display initialised");

            // create an alarm to update the display regularly
            let mut display_update_alarm = timer.alarm_0().unwrap();
            let _ = display_update_alarm.schedule(DISPLAY_UPDATE_INTERVAL_US);
            display_update_alarm.enable_interrupt();

            // RTIC Monotonic

            // create a monotonic timer for RTIC
            let monotonic_alarm = timer.alarm_3().unwrap();
            let monotonic_timer = Monotonic::new(timer, monotonic_alarm);

            // APP STATE
            
            // create buffer to collect MIDI tick intervals
            let tick_history = HistoryBuffer::<u64, 24>::new();

            // show track page of UI at startup
            let input_mode = InputMode::Track;

            // TODO configure interrupts on button and encoder GPIO pins
            // let mut buttons = Vec::<InputPin, 4>::new();
            // for button in buttons {
            //     button.set_interrupt_enabled(EdgeLow, true);
            // }

            // create some tracks!
            let mut tracks = Vec::new();
            for _ in 0..16 {
                tracks.push(Some(Track::new())).unwrap();
            }

            info!("init complete");

            (Shared { input_mode, playing, tracks }, Local { midi_in, midi_out, display, display_update_alarm, tick_history }, init::Monotonics(monotonic_timer))
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
            let mut gate_time = 100_000.micros(); // 80% of 1/16th note at 120bpm
            if let Some(last_tick_instant) = *last_tick_instant {
                let tick_duration = monotonics::now().checked_duration_since(last_tick_instant).unwrap();
                tick_history.write(tick_duration.to_micros());
                let avg_tick_duration = tick_history.as_slice().iter().sum::<u64>() / tick_history.len() as u64;
                debug!("sequencer_advance: avg_tick_duration={}", avg_tick_duration);

                // TODO use step.length
                let gate_time: MicrosDurationU32 = ((avg_tick_duration * DEFAULT_NOTE_LENGTH_24PPQN_TICKS as u64) as u32).micros();
            }

            // TODO should all move out of task, e.g. Tracks::advance()?
            ctx.shared.tracks.lock(|tracks| {
                for track in tracks.as_slice() {
                    if let Some(track) = track {
                        if let Some(step) = &track.steps[*ticks as usize] {
                            let note_on_message = MidiMessage::NoteOn(track.midi_channel, step.note, step.velocity);
                            midi_send::spawn(note_on_message).unwrap();

                            let note_off_message = MidiMessage::NoteOff(track.midi_channel, step.note, 0.into());
                            debug!("sequencer_advance: scheduling note off message for {}us", gate_time.to_micros());
                            midi_send::spawn_after(gate_time.into(), note_off_message).unwrap();
                        }
                    }
                }
            });
        }

        // timer task to render UI at k FPS
        #[task(binds = TIMER_IRQ_0, priority = 1, local = [display, display_update_alarm, display_ticks: u32 = 0])]
        fn timer_irq_0(ctx: timer_irq_0::Context) {
            *ctx.local.display_ticks += 1;
            debug!("rendering ui, display_ticks={}", ctx.local.display_ticks);

            let mut ticks_string: String<8> = String::new();

            ctx.local.display.clear();

            // define style for text
            let text_style = MonoTextStyleBuilder::new()
                .font(&FONT_4X6)
                .text_color(BinaryColor::On)
                .build();

            Text::with_baseline("MICROGROOVE", Point::zero(), text_style, Baseline::Top)
                .draw(&mut *ctx.local.display)
                .unwrap();

            let _ = write!(ticks_string, "TICKS: {}", *ctx.local.display_ticks);
            Text::with_baseline(ticks_string.as_str(), Point::new(0, 6), text_style, Baseline::Top)
                .draw(&mut *ctx.local.display)
                .unwrap();

            ctx.local.display.flush().unwrap();

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
