#![no_std]
#![no_main]

use panic_probe as _;

mod microgroove {
    mod sequencer {
        use heapless::Vec;
        use midi_types::{Channel, Note, Value14, Value7};

        // represent a step in a musical sequence
        // TODO polyphonic steps - note: Note -> notes: Vec<Note, _>
        #[derive(Clone, Debug)]
        pub struct Step {
            pub note: Note,
            pub velocity: Value7,
            pub pitch_bend: Value14,
            pub length_step_cents: u8,
        }

        impl Step {
            pub fn new(note: Note) -> Step {
                Step {
                    note,
                    velocity: 127.into(),
                    pitch_bend: 0u16.into(),
                    length_step_cents: 85, // == gate time is 85% of step length
                }
            }
        }

        #[derive(Debug)]
        pub enum TimeDivision {
            NinetySixth = 1, // corresponds to midi standard of 24 clock pulses per quarter note
            ThirtySecond = 3,
            Sixteenth = 6,
            Eigth = 12,
            Quarter = 24,
            Whole = 96
        }

        pub type Sequence = Vec<Option<Step>, 32>;

        #[derive(Debug)]
        pub struct Track {
            pub time_division: TimeDivision,
            pub length: u8,
            pub midi_channel: Channel,
            pub steps: Sequence,
        }

        impl Track {
            pub fn new() -> Track {
                let steps = [57, 59, 60, 62, 64, 65, 67, 69, 57, 59, 60, 62, 64, 65, 67, 69]
                // let steps = [60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60, 60]
                    .map(|note_num| { Some(Step::new(note_num.into())) });
                Track {
                    time_division: TimeDivision::Sixteenth,
                    length: 16,
                    midi_channel: 0.into(),
                    steps: Vec::from_slice(steps.as_slice()).unwrap(),
                }
            }
        }
    }

    // RTIC app module
    #[rtic::app(
        device = rp_pico::hal::pac,
        peripherals = true,
        dispatchers = [I2C0_IRQ, DMA_IRQ_0, DMA_IRQ_1, SPI0_IRQ, SPI1_IRQ, UART1_IRQ]
    )]
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
                    FunctionI2C,
                    FunctionUart,
                    Interrupt::{EdgeLow, EdgeHigh},
                    Pin,
                    PullUpInput,
                    pin::bank0::{
                        Gpio0,
                        Gpio1,
                        Gpio2,
                        Gpio3,
                        Gpio4,
                        Gpio5,
                        Gpio6,
                        Gpio7,
                        Gpio8,
                        Gpio9,
                        Gpio10,
                        Gpio11,
                        Gpio12,
                        Gpio13,
                        Gpio14,
                        Gpio16,
                        Gpio17,
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
                    Alarm1,
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

        // driver for rotary encoders
        use rotary_encoder_hal::{Direction, Rotary};

        // midi stuff
        use midi_types::MidiMessage;
        use embedded_midi::{MidiIn, MidiOut};

        // ssd1306 oled display driver
        use ssd1306::{
            I2CDisplayInterface,
            Ssd1306,
            prelude::*,
            mode::BufferedGraphicsMode,
        };

        // graphics APIs
        use embedded_graphics::{
            mono_font::{
                MonoTextStyleBuilder,
                ascii::{FONT_4X6, FONT_8X13_ITALIC},
            },
            pixelcolor::BinaryColor,
            prelude::*,
            text::{Baseline, Text},
        };
        
        // non-blocking io
        use nb::block;

        // defmt rtt logging (read the logs with cargo embed, etc)
        use defmt;
        use defmt::{error, warn, info, debug, trace};
        use defmt_rtt as _;

        // alloc-free data structures
        use heapless::{Vec, HistoryBuffer, String};

        // Write trait to allow formatting heapless Strings
        use core::fmt::Write;

        // time manipulation
        use fugit::{ExtU64, MicrosDurationU64, RateExtU32};

        // crate imports
        use super::sequencer::{
            Step,
            Track,
        };

        // display update time is the time between each render
        // this is the upper bound for the time to flush each frame to the oled
        // at 40ms, the frame rate will be 25 FPS
        // we want the lowest frame rate that looks acceptable, to provide the largest budget for
        // render times
        // TODO move to UI/input module
        // TODO actually use this
        const DISPLAY_UPDATE_INTERVAL: MicrosDurationU64 = MicrosDurationU64::millis(40);

        // monotonic clock for RTIC and defmt
        #[monotonic(binds = TIMER_IRQ_0, default = true)]
        type TimerMonotonic = Monotonic<Alarm0>;
        type TimerMonotonicInstant = <TimerMonotonic as rtic::rtic_monotonic::Monotonic>::Instant;

        // type alias for UART pins
        type MidiOutUartPin = Pin<Gpio16, FunctionUart>;
        type MidiInUartPin = Pin<Gpio17, FunctionUart>;
        type MidiUartPins = (MidiOutUartPin, MidiInUartPin);

        // type alias for display pins
        type DisplaySdaPin = Pin<Gpio26, FunctionI2C>;
        type DisplaySclPin = Pin<Gpio27, FunctionI2C>;
        type DisplayPins = (DisplaySdaPin, DisplaySclPin);

        // type alias for button pins
        type ButtonTrackPin = Pin<Gpio0, PullUpInput>;
        type ButtonGroovePin = Pin<Gpio1, PullUpInput>;
        type ButtonMelodyPin = Pin<Gpio2, PullUpInput>;

        // type alias for encoder pins
        type Encoder0PinA = Pin<Gpio9, PullUpInput>;
        type Encoder0PinB = Pin<Gpio10, PullUpInput>;
        type Encoder0 = Rotary<Encoder0PinA, Encoder0PinB>;
        type Encoder1PinA = Pin<Gpio11, PullUpInput>;
        type Encoder1PinB = Pin<Gpio12, PullUpInput>;
        type Encoder2PinA = Pin<Gpio13, PullUpInput>;
        type Encoder2PinB = Pin<Gpio14, PullUpInput>;

        type Encoder3PinA = Pin<Gpio3, PullUpInput>;
        type Encoder3PinB = Pin<Gpio4, PullUpInput>;
        type Encoder4PinA = Pin<Gpio5, PullUpInput>;
        type Encoder4PinB = Pin<Gpio6, PullUpInput>;
        type Encoder5PinA = Pin<Gpio7, PullUpInput>;
        type Encoder5PinB = Pin<Gpio8, PullUpInput>;

        // TODO move to UI/input module
        pub enum InputMode {
            Track,
            Groove,
            Melody
        }

        // RTIC shared resources
        #[shared]
        struct Shared {
            // are we playing, or not?
            playing: bool,

            // current page of the UI
            input_mode: InputMode,

            // encoder positions
            encoder0_pos: i32,

            // tracks are where we store our sequence data
            tracks: Vec<Option<Track>, 16>,
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

            // pins for buttons
            button_track_pin: ButtonTrackPin,
            button_groove_pin: ButtonGroovePin,
            button_melody_pin: ButtonMelodyPin,

            // encoders
            encoder0: Encoder0,

            // a buffer to track the intervals between MIDI ticks, which we can 
            // use to estimate the tempo, we can then use our tempo estimate to 
            // implement note lengths and swing
            tick_history: HistoryBuffer::<u64, 24>,
        }

        // RTIC init
        #[init]
        fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
            info!("hello world!");

            // release spinlocks to avoid a deadlock after soft-reset
            unsafe {
                sio::spinlock_reset();
            }

            // DEVICE SETUP

            // I don't know what these are, but they are REQUIRED MAGIC
            let mut resets = ctx.device.RESETS;

            // the single-cycle i/o block controls our gpio pins
            let sio = Sio::new(ctx.device.SIO);

            // set the pins to their default state
            let pins = Pins::new(
                ctx.device.IO_BANK0,
                ctx.device.PADS_BANK0,
                sio.gpio_bank0,
                &mut resets,
            );

            // clock setup for timers and alarms
            let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);
            let clocks = clocks::init_clocks_and_plls(
                XOSC_CRYSTAL_FREQ,
                ctx.device.XOSC,
                ctx.device.CLOCKS,
                ctx.device.PLL_SYS,
                ctx.device.PLL_USB,
                &mut resets,
                &mut watchdog,
            )
            .ok()
            .expect("init_clocks_and_plls(...) should succeed");

            // timer for, well, timing
            let mut timer = Timer::new(ctx.device.TIMER, &mut resets);

            // BUTTONS

            // configure interrupts on button and encoder GPIO pins
            let button_track_pin = pins.gpio0.into_pull_up_input();
            button_track_pin.set_interrupt_enabled(EdgeLow, true);
            let button_groove_pin = pins.gpio1.into_pull_up_input();
            button_groove_pin.set_interrupt_enabled(EdgeLow, true);
            let button_melody_pin = pins.gpio2.into_pull_up_input();
            button_melody_pin.set_interrupt_enabled(EdgeLow, true);

            // // ENCODERS

            let encoder0_pin_a = pins.gpio9.into_pull_up_input();
            let encoder0_pin_b = pins.gpio10.into_pull_up_input();
            // encoder0_pin_a.set_interrupt_enabled(EdgeLow, true);
            // encoder0_pin_b.set_interrupt_enabled(EdgeLow, true);
            // encoder0_pin_a.set_interrupt_enabled(EdgeHigh, true);
            // encoder0_pin_b.set_interrupt_enabled(EdgeHigh, true);
            let mut encoder0 = Rotary::new(encoder0_pin_a, encoder0_pin_b);

            // let encoder1_pin_a = pins.gpio11.into_pull_up_input();
            // let encoder1_pin_b = pins.gpio12.into_pull_up_input();
            // encoder1_pin_a.set_interrupt_enabled(EdgeLow, true);
            // encoder1_pin_b.set_interrupt_enabled(EdgeLow, true);
            // encoder1_pin_a.set_interrupt_enabled(EdgeHigh, true);
            // encoder1_pin_b.set_interrupt_enabled(EdgeHigh, true);
            // let mut encoder1 = Rotary::new(encoder1_pin_a, encoder1_pin_b);

            // let encoder2_pin_a = pins.gpio13.into_pull_up_input();
            // let encoder2_pin_b = pins.gpio14.into_pull_up_input();
            // encoder2_pin_a.set_interrupt_enabled(EdgeLow, true);
            // encoder2_pin_b.set_interrupt_enabled(EdgeLow, true);
            // encoder2_pin_a.set_interrupt_enabled(EdgeHigh, true);
            // encoder2_pin_b.set_interrupt_enabled(EdgeHigh, true);
            // let mut encoder2 = Rotary::new(encoder2_pin_a, encoder2_pin_b);

            // let encoder3_pin_a = pins.gpio3.into_pull_up_input();
            // let encoder3_pin_b = pins.gpio4.into_pull_up_input();
            // encoder3_pin_a.set_interrupt_enabled(EdgeLow, true);
            // encoder3_pin_b.set_interrupt_enabled(EdgeLow, true);
            // encoder3_pin_a.set_interrupt_enabled(EdgeHigh, true);
            // encoder3_pin_b.set_interrupt_enabled(EdgeHigh, true);
            // let mut encoder3 = Rotary::new(encoder3_pin_a, encoder3_pin_b);

            // let encoder4_pin_a = pins.gpio5.into_pull_up_input();
            // let encoder4_pin_b = pins.gpio6.into_pull_up_input();
            // encoder4_pin_a.set_interrupt_enabled(EdgeLow, true);
            // encoder4_pin_b.set_interrupt_enabled(EdgeLow, true);
            // encoder4_pin_a.set_interrupt_enabled(EdgeHigh, true);
            // encoder4_pin_b.set_interrupt_enabled(EdgeHigh, true);
            // let mut encoder4 = Rotary::new(encoder4_pin_a, encoder4_pin_b);

            // let encoder5_pin_a = pins.gpio7.into_pull_up_input();
            // let encoder5_pin_b = pins.gpio8.into_pull_up_input();
            // encoder5_pin_a.set_interrupt_enabled(EdgeLow, true);
            // encoder5_pin_b.set_interrupt_enabled(EdgeLow, true);
            // encoder5_pin_a.set_interrupt_enabled(EdgeHigh, true);
            // encoder5_pin_b.set_interrupt_enabled(EdgeHigh, true);
            // let mut encoder5 = Rotary::new(encoder5_pin_a, encoder5_pin_b);

            // MIDI

            // put pins for midi into uart mode
            let midi_uart_pins = (
                pins.gpio16.into_mode::<FunctionUart>(),
                pins.gpio17.into_mode::<FunctionUart>(),
            );

            // make a uart peripheral on the given pins
            let uart_config = UartConfig::new(31_250.Hz(), DataBits::Eight, None, StopBits::One);
            let mut midi_uart = UartPeripheral::new(ctx.device.UART0, midi_uart_pins, &mut resets)
                .enable(uart_config, clocks.peripheral_clock.freq())
                .expect("midi_uart.enable(...) should succeed");

            // configure uart interrupt to fire on midi input
            midi_uart.enable_rx_interrupt();

            // split the uart into rx and tx channels and create MidiIn/Out interfaces
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
                &mut resets,
                &clocks.peripheral_clock,
            );

            // create i2c display interface
            let mut display = Ssd1306::new(
                I2CDisplayInterface::new_alternate_address(i2c),
                DisplaySize128x64,
                DisplayRotation::Rotate0
            ).into_buffered_graphics_mode();

            // intialise display
            display.init().expect("init: display initialisation failed");

            // show splash screen
            display.clear();

            let text_style = MonoTextStyleBuilder::new()
                .font(&FONT_8X13_ITALIC)
                .text_color(BinaryColor::On)
                .build();

            Text::with_baseline("MICROGROOVE", Point::new(20, 20), text_style, Baseline::Top)
                .draw(&mut display)
                .unwrap();

            display.flush().unwrap();

            info!("init: display initialised");

            // RTIC MONOTONIC

            // create a monotonic timer for RTIC (1us resolution!)
            let monotonic_alarm = timer.alarm_0().unwrap();
            let monotonic_timer = Monotonic::new(timer, monotonic_alarm);

            // configure source of timestamps for defmt
            defmt::timestamp!("{=u64:us}", {
                monotonics::now().duration_since_epoch().to_micros()
            });

            // APP STATE
            
            let playing = false;

            // show track page of UI at startup
            let input_mode = InputMode::Track;

            // initial encoder positions
            let encoder0_pos = 0;

            // create a track
            let mut tracks = Vec::new();
            tracks.push(Some(Track::new())).unwrap();

            // buffer to collect MIDI tick intervals
            let tick_history = HistoryBuffer::<u64, 24>::new();

            // LET'S GOOOO!!

            // start reading encoders
            read_encoders::spawn_after(1.millis()).unwrap();

            // start scheduled display updates
            display_update::spawn_after(40.millis()).unwrap();

            info!("init: complete 🤘");

            (
                Shared {
                    input_mode,
                    playing,
                    encoder0_pos,
                    tracks,
                },
                Local {
                    midi_in,
                    midi_out,
                    display,
                    button_track_pin,
                    button_groove_pin,
                    button_melody_pin,
                    encoder0,
                    tick_history,
                },
                init::Monotonics(monotonic_timer)
            )
        }

        // handles UART0 interrupts, which is MIDI input
        #[task(
            binds = UART0_IRQ,
            priority = 4,
            shared = [playing],
            local = [midi_in]
        )]
        fn uart0_irq(mut ctx: uart0_irq::Context) {
            // check midi input for messages
            trace!("a wild uart0 interrupt has fired!");

            // read those sweet sweet midi bytes!
            if let Ok(message) = block!(ctx.local.midi_in.read()) {
                // log the message
                match message {
                    MidiMessage::TimingClock => {
                        trace!("midi: clock");

                        // if clock, spawn task to tick tracks and potentially generate midi output
                        ctx.shared.playing.lock(|playing| {
                            if *playing {
                                sequencer_advance::spawn().expect("sequencer_advance::spawn() should succeed");
                            }
                        });
                    }
                    MidiMessage::Start => {
                        info!("midi: start");
                        ctx.shared.playing.lock(|playing| {
                            *playing = true;
                        });
                    }
                    MidiMessage::Stop => {
                        info!("midi: stop");
                        ctx.shared.playing.lock(|playing| {
                            *playing = false;
                        });
                    }
                    MidiMessage::Continue => {
                        info!("midi: continue");
                        ctx.shared.playing.lock(|playing| {
                            *playing = true;
                        });
                    }
                    _ => trace!("midi: UNKNOWN"),
                }

                // pass received message to midi out ("soft thru")
                match midi_send::spawn(message) {
                    Ok(_) => (),
                    Err(_) => error!("could not spawn midi_send to pass through message"),
                }
            }
        }

        #[task(
            priority = 3,
            capacity = 64,
            local = [midi_out]
        )]
        fn midi_send(ctx: midi_send::Context, message: MidiMessage) {
            trace!("midi_send");
            match message {
                MidiMessage::TimingClock => trace!("midi_send: clock"),
                MidiMessage::Start => trace!("midi_send: start"),
                MidiMessage::Stop => trace!("midi_send: stop"),
                MidiMessage::Continue => trace!("midi_send: continue"),
                MidiMessage::NoteOn(midi_channel, note, velocity) => {
                    let midi_channel: u8 = midi_channel.into();
                    let note: u8 = note.into();
                    let velocity: u8 = velocity.into();
                    debug!("midi_send: note on midi_channel={} note={} velocity={}", midi_channel, note, velocity);
                }
                MidiMessage::NoteOff(midi_channel, note, velocity) => {
                    let midi_channel: u8 = midi_channel.into();
                    let note: u8 = note.into();
                    let velocity: u8 = velocity.into();
                    debug!("midi_send: note off midi_channel={} note={}", midi_channel, note);
                }
                _ => trace!("midi: UNKNOWN"),
            }
            ctx.local.midi_out.write(&message).expect("midi_out.write(message) should succeed");
        }

        #[task(
            priority = 2,
            shared = [tracks],
            local = [ticks: u32 = 0, last_tick_instant: Option<TimerMonotonicInstant> = None, tick_history]
        )]
        fn sequencer_advance(mut ctx: sequencer_advance::Context) {
            trace!("sequencer_advance");

            let sequencer_advance::LocalResources { ticks, last_tick_instant, tick_history } = ctx.local;

            // calculate average interval between last K ticks
            // TODO should move to some impl
            let mut tick_duration: MicrosDurationU64 = 20_830.micros(); // time between ticks at 120bpm
            if let Some(last_tick_instant) = *last_tick_instant {
                let last_tick_duration = monotonics::now().checked_duration_since(last_tick_instant).unwrap().to_micros();
                tick_history.write(last_tick_duration);
                tick_duration = (tick_history.as_slice().iter().sum::<u64>() / tick_history.len() as u64).micros();
            }

            *last_tick_instant = Some(monotonics::now());

            trace!("sequencer_advance: tick_duration={}", tick_duration.to_micros());

            // TODO should all move out of task, e.g. Tracks::advance()?
            ctx.shared.tracks.lock(|tracks| {
                for track in tracks.as_slice() {
                    if let Some(track) = track {
                        if *ticks % (track.time_division as u32) == 0 {
                            let step_num = (*ticks % track.length as u32) as usize;
                            if let Some(step) = &track.steps.get(step_num).unwrap() {
                                let note_on_message = MidiMessage::NoteOn(track.midi_channel, step.note, step.velocity);
                                let midi_channel: u8 = track.midi_channel.into();
                                let note: u8 = step.note.into();
                                let velocity: u8 = step.velocity.into();
                                trace!("sequencer_advance: note_on channel={} note={} velocity={}", midi_channel, note, velocity);
                                match midi_send::spawn(note_on_message) {
                                    Ok(_) => (),
                                    Err(_error) => error!("could not spawn midi_send for note on message"),
                                }

                                let note_off_message = MidiMessage::NoteOff(track.midi_channel, step.note, 0.into());
                                let note_off_time = ((tick_duration.to_micros() * (track.time_division as u64) * step.length_step_cents as u64) / 100).micros();
                                trace!("sequencer_advance: scheduling note off message for {}us", note_off_time.to_micros());
                                match midi_send::spawn_after(note_off_time, note_off_message) {
                                    Ok(_) => (),
                                    Err(_error) => error!("could not spawn midi_send for note off message"),
                                }
                            }
                        }
                    }
                }
            });

            *ticks += 1; // will overflow after a few years of continuous play
        }

        // handles GPIO input, which is either a button press or an encoder turn
        #[task(
            binds = IO_IRQ_BANK0,
            priority = 3,
            shared = [input_mode],
            local = [button_track_pin, button_groove_pin, button_melody_pin]
        )]
        fn io_irq_bank0(mut ctx: io_irq_bank0::Context) {
            trace!("a wild gpio_bank0 interrupt has fired!");

            // TODO WTF aren't these working any more??
            // for each button, check interrupt status to see if we fired
            if ctx.local.button_track_pin.interrupt_status(EdgeLow) {
                info!("track button pressed");
                ctx.shared.input_mode.lock(|input_mode| {
                    *input_mode = InputMode::Track;
                });
                ctx.local.button_track_pin.clear_interrupt(EdgeLow);
            }
            if ctx.local.button_groove_pin.interrupt_status(EdgeLow) {
                info!("groove button pressed");
                ctx.shared.input_mode.lock(|input_mode| {
                    *input_mode = InputMode::Groove;
                });
                ctx.local.button_groove_pin.clear_interrupt(EdgeLow);
            }
            if ctx.local.button_melody_pin.interrupt_status(EdgeLow) {
                info!("melody button pressed");
                ctx.shared.input_mode.lock(|input_mode| {
                    *input_mode = InputMode::Melody;
                });
                ctx.local.button_melody_pin.clear_interrupt(EdgeLow);
            }
        }

        #[task(
            priority = 4,
            shared = [encoder0_pos],
            local = [encoder0]
        )]
        fn read_encoders(mut ctx: read_encoders::Context) {
            // check encoders every 1ms to remove some of the noise vs checking on interrupt
            // TODO trigger param changes
            ctx.shared.encoder0_pos.lock(|encoder0_pos| {
                match ctx.local.encoder0.update().unwrap() {
                    Direction::Clockwise => {
                        *encoder0_pos += 1;
                    }
                    Direction::CounterClockwise => {
                        *encoder0_pos -= 1;
                    }
                    Direction::None => {}
                }
            });

            // TODO repeat for encoder[1-5]

            // read again in 1ms
            read_encoders::spawn_after(1.millis()).unwrap();
        }

        #[task(
            priority = 1,
            shared = [playing, input_mode, encoder0_pos],
            local = [display, display_ticks: u32 = 0]
        )]
        fn display_update(mut ctx: display_update::Context) {
            *ctx.local.display_ticks += 1;
            trace!("rendering ui, display_ticks={}", *ctx.local.display_ticks);

            ctx.local.display.clear();

            let text_style = MonoTextStyleBuilder::new()
                .font(&FONT_4X6)
                .text_color(BinaryColor::On)
                .build();

            Text::with_baseline("MICROGROOVE", Point::zero(), text_style, Baseline::Top)
                .draw(&mut *ctx.local.display)
                .unwrap();

            ctx.shared.playing.lock(|playing| {
                let text: String<30> = String::from(if *playing { "PLAYING" } else { "STOPPED" });
                Text::with_baseline(text.as_str(), Point::new(0, 6), text_style, Baseline::Top)
                    .draw(&mut *ctx.local.display)
                    .unwrap();
            });

            ctx.shared.input_mode.lock(|input_mode| {
                let mut text: String<30> = String::new();
                let mode_text = match *input_mode {
                    InputMode::Track => "TRACK",
                    InputMode::Groove => "GROOVE",
                    InputMode::Melody => "MELODY",
                };
                let _ = write!(text, "MODE: {}", mode_text);
                Text::with_baseline(text.as_str(), Point::new(0, 12), text_style, Baseline::Top)
                    .draw(&mut *ctx.local.display)
                    .unwrap();
            });

            ctx.shared.encoder0_pos.lock(|encoder0_pos| {
                let mut text: String<30> = String::new();
                let _ = write!(text, "ENC0: {}", *encoder0_pos);
                Text::with_baseline(text.as_str(), Point::new(0, 18), text_style, Baseline::Top)
                    .draw(&mut *ctx.local.display)
                    .unwrap();
            });

            ctx.local.display.flush().unwrap();

            display_update::spawn_after(40.millis()).expect("should be able to spawn_after display_update");
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
