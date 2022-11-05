#![no_std]
#![no_main]

use panic_probe as _;

mod microgroove {
    /// Core data model for a MIDI sequencer. Provides types to represent a sequencer as a set of
    /// tracks, each with a Sequence of Steps. A Step consists of the basic information required to
    /// play a note.
    pub mod sequencer {
        use heapless::Vec;
        use midi_types::{Channel, Note, Value14, Value7};

        /// Represent a step in a musical sequence.
        #[derive(Clone, Debug)]
        pub struct Step {
            pub note: Note,
            pub velocity: Value7,
            pub pitch_bend: Value14,

            /// Note gate time as % of step time, e.g. 80 = 80%. Step time is defined by
            /// Track::time_division.
            pub length_step_cents: u8,

            /// Delay playing this step for % of track time division. Used for swing. Can be abused
            /// for general timing madness. Note that its not possible to play a step early. This
            /// is because Microgroove depends on an external clock.
            pub delay: u8
        }

        impl Step {
            pub fn new() -> Step {
                Step {
                    note: 60.into(),
                    velocity: 127.into(),
                    pitch_bend: 0u16.into(),
                    length_step_cents: 80,
                    delay: 0,
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
            Whole = 96,
        }

        pub type Sequence = Vec<Option<Step>, 32>;

        #[derive(Debug)]
        pub struct Track {
            pub time_division: TimeDivision,
            pub length: u8,
            pub midi_channel: Channel,
            pub steps: Sequence,
            pub current_step: u8,
        }

        impl Track {
            pub fn new() -> Track {
                Track {
                    time_division: TimeDivision::Sixteenth,
                    length: 16,
                    midi_channel: 0.into(),
                    steps: Track::generate_sequence(),
                    current_step: 0,
                }
            }

            fn generate_sequence() -> Sequence {
                Self::default_sequence()
            }

            fn default_sequence() -> Sequence {
                (0..16).map(|_x| Some(Step::new())).collect()
            }
        }
    }

    pub mod encoder {
        pub mod positional_encoder {
            use rp_pico::hal::gpio::DynPin;
            use rotary_encoder_hal::{Direction, Rotary};
            use defmt::error;

            pub struct PositionalEncoder {
                encoder: Rotary<DynPin, DynPin>,
                value: i32,
            }

            impl PositionalEncoder {
                pub fn new(mut pin_a: DynPin, mut pin_b: DynPin) -> PositionalEncoder {
                    pin_a.into_pull_up_input();
                    pin_b.into_pull_up_input();
                    PositionalEncoder {
                        encoder: Rotary::new(pin_a.into(), pin_b.into()),
                        value: 0,
                    }
                }

                /// Check the encoder state for changes. This should be called frequently, e.g.
                /// every 1ms. Returns a `Some` containing the encoder value if there have been
                /// changes, `None` otherwise.
                pub fn update(&mut self) -> Option<i32> {
                    match self.encoder.update() {
                        Ok(Direction::Clockwise) => {
                            self.value += 1;
                            Some(self.value)
                        }
                        Ok(Direction::CounterClockwise) => {
                            self.value += 1;
                            Some(self.value)
                        }
                        Ok(Direction::None) => {
                            None
                        }
                        Err(_error) => {
                            error!("could not update encoder");
                            None
                        }
                    }
                }

                /// Get the value of the encoder, and then reset that to zero. This has the
                /// semantics of "I would like to know your value, which I will use to update my
                /// state, so you can then discard it."
                pub fn take_value(&mut self) -> i32 {
                    let val = self.value;
                    self.value = 0;
                    val
                }
            }
        }

        pub mod encoder_array {
            use heapless::Vec;
            use super::positional_encoder::PositionalEncoder;

            pub const ENCODER_COUNT: usize = 6;

            /// An array of multiple `PositionalEncoders`.
            pub struct EncoderArray {
                encoders: Vec<PositionalEncoder, ENCODER_COUNT>,
            }

            impl EncoderArray {
                pub fn new(encoders: Vec<PositionalEncoder, ENCODER_COUNT>) -> EncoderArray {
                    EncoderArray { encoders }
                }    

                pub fn update(&mut self) -> Option<()> {
                    let any_changes = self.encoders.iter_mut().map(|enc| enc.update()).any(|opt| opt.is_some());
                    if any_changes { Some(()) } else { None }
                }

                pub fn take_values(&mut self) -> Vec<i32, ENCODER_COUNT> {
                    self.encoders.iter_mut().map(|enc| enc.take_value()).collect()
                }
            }
        }
    }

    /// Rendering UI graphics to the display.
    pub mod display {
        // graphics APIs
        use embedded_graphics::{
            mono_font::{
                ascii::FONT_8X13_ITALIC,
                MonoTextStyleBuilder,
            },
            pixelcolor::BinaryColor,
            prelude::*,
            text::{Baseline, Text},
        };

        use super::{
            sequencer::Track,
            input::InputMode,
            peripherals::Display
        };

        /// Show snazzy splash screen.
        fn render_splash_screen(display: &mut Display) {
            display.clear();

            let text_style = MonoTextStyleBuilder::new()
                .font(&FONT_8X13_ITALIC)
                .text_color(BinaryColor::On)
                .build();

            Text::with_baseline("MICROGROOVE", Point::new(20, 20), text_style, Baseline::Top)
                .draw(display)
                .unwrap();

            display.flush().unwrap();
        }

        pub fn render(display: &Display, track: &Option<Track>, input_mode: InputMode, playing: bool) {
            if track.is_none() {
                show_disabled_track_warning();
                return;
            }

            panic!("TODO");
        }

        fn show_disabled_track_warning() {
            panic!("TODO");
        }
    }
    
    pub mod midi {
        use defmt::{debug, trace};
        use midi_types::MidiMessage;

        pub fn log_message(message: &MidiMessage) {
            match message {
                MidiMessage::TimingClock => trace!("[midi_send] clock"),
                MidiMessage::Start => trace!("[midi_send] start"),
                MidiMessage::Stop => trace!("[midi_send] stop"),
                MidiMessage::Continue => trace!("[midi_send] continue"),
                MidiMessage::NoteOn(midi_channel, note, velocity) => {
                    let midi_channel: u8 = (*midi_channel).into();
                    let note: u8 = (*note).into();
                    let velocity: u8 = (*velocity).into();
                    debug!(
                        "[midi_send] note on midi_channel={} note={} velocity={}",
                        midi_channel, note, velocity
                    );
                }
                MidiMessage::NoteOff(midi_channel, note, _velocity) => {
                    let midi_channel: u8 = (*midi_channel).into();
                    let note: u8 = (*note).into();
                    debug!(
                        "[midi_send] note off midi_channel={} note={}",
                        midi_channel, note
                    );
                }
                _ => trace!("[midi_send] UNKNOWN"),
            }
        }
    }

    /// Handle user input (encoder turns, button presses).
    pub mod input {
        use heapless::Vec;
        use super::{
            encoder::encoder_array::ENCODER_COUNT,
            sequencer::Track
        };

        #[derive(Clone, Copy, Debug)]
        pub enum InputMode {
            Track,
            Rhythm,
            Melody,
        }

        /// Iterate over `encoder_values` and pass to either `Track`, `RhythmMachine` or
        /// `MelodyMachine`, determined by `input_mode`.
        pub fn map_encoder_input(track: &mut Option<Track>, input_mode: InputMode, encoder_values: Vec<i32, ENCODER_COUNT>) {
            if track.is_none() {
                track.replace(Track::new());
            }
        }
    }

    /// Device initialisation and interfacing.
    pub mod peripherals {
        use heapless::Vec;
        use rp2040_hal::clocks::PeripheralClock;
        use rp_pico::{
            hal::{
                clocks,
                gpio::{
                    pin::bank0::{Gpio0, Gpio1, Gpio16, Gpio17, Gpio2, Gpio26, Gpio27},
                    FunctionI2C, FunctionUart,
                    Interrupt::EdgeLow,
                    Pin, PullUpInput,
                },
                pac::{I2C1, UART0, Peripherals as PacPeripherals},
                sio::Sio,
                timer::{monotonic::Monotonic, Alarm0},
                uart::{DataBits, Reader, StopBits, UartConfig, UartPeripheral, Writer},
                Clock, Timer, Watchdog, I2C,
            },
            Pins, XOSC_CRYSTAL_FREQ,
        };
        use embedded_midi::{MidiIn as EmbeddedMidiIn, MidiOut as EmbeddedMidiOut};
        use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};
        use fugit::RateExtU32;
        use super::encoder::{
            encoder_array::EncoderArray,
            positional_encoder::PositionalEncoder,
        };

        // type alias for UART pins
        type MidiOutUartPin = Pin<Gpio16, FunctionUart>;
        type MidiInUartPin = Pin<Gpio17, FunctionUart>;
        type MidiUartPins = (MidiOutUartPin, MidiInUartPin);

        // microgroove-specific midi in/out channel types
        pub type MidiIn = EmbeddedMidiIn<Reader<UART0, MidiUartPins>>;
        pub type MidiOut = EmbeddedMidiOut<Writer<UART0, MidiUartPins>>;

        // type alias for display pins
        type DisplaySdaPin = Pin<Gpio26, FunctionI2C>;
        type DisplaySclPin = Pin<Gpio27, FunctionI2C>;
        pub type DisplayPins = (DisplaySdaPin, DisplaySclPin);

        // microgroove-specific display type
        pub type Display = Ssd1306<
            I2CInterface<I2C<I2C1, DisplayPins>>,
            DisplaySize128x64,
            BufferedGraphicsMode<DisplaySize128x64>,
        >;

        // type alias for button pins
        pub type ButtonTrackPin = Pin<Gpio0, PullUpInput>;
        pub type ButtonRhythmPin = Pin<Gpio1, PullUpInput>;
        pub type ButtonMelodyPin = Pin<Gpio2, PullUpInput>;
        type ButtonArray = (ButtonTrackPin, ButtonRhythmPin, ButtonMelodyPin);

        pub struct Peripherals {
            pub monotonic_timer: Monotonic<Alarm0>,
            pub midi_in: MidiIn,
            pub midi_out: MidiOut,
            pub display: Display,
            pub buttons: ButtonArray,
            pub encoders: EncoderArray,
        }

        impl Peripherals {
            pub fn new(device: &mut PacPeripherals) -> Peripherals {
                let pins = pins(device);
                let peripheral_clock = peripheral_clock(device);
                let (midi_in, midi_out) = midi_in_out(device, &pins, &peripheral_clock);
                Peripherals {
                    monotonic_timer: monotonic_timer(device),
                    midi_in,
                    midi_out,
                    display: display(device, &pins, &peripheral_clock),
                    buttons: buttons(device, &pins),
                    encoders: encoders(device, &pins),
                }
            }
        }

        fn pins(device: &mut PacPeripherals) -> Pins {
            let sio = Sio::new(device.SIO);
            Pins::new(
                device.IO_BANK0,
                device.PADS_BANK0,
                sio.gpio_bank0,
                &mut device.RESETS,
            )
        }

        fn peripheral_clock(device: &mut PacPeripherals) -> PeripheralClock {
            let mut watchdog = Watchdog::new(device.WATCHDOG);
            let clocks = clocks::init_clocks_and_plls(
                XOSC_CRYSTAL_FREQ,
                device.XOSC,
                device.CLOCKS,
                device.PLL_SYS,
                device.PLL_USB,
                &mut device.RESETS,
                &mut watchdog,
            )
            .ok()
            .expect("init: init_clocks_and_plls(...) should succeed");
            clocks.peripheral_clock
        }

        fn monotonic_timer(device: &mut PacPeripherals) -> Monotonic<Alarm0> {
            let mut timer = Timer::new(device.TIMER, &mut device.RESETS);
            let monotonic_alarm = timer.alarm_0().unwrap();
            Monotonic::new(timer, monotonic_alarm)
        }

        fn midi_in_out(device: &mut PacPeripherals, pins: &Pins, peripheral_clock: &PeripheralClock) -> (MidiIn, MidiOut) {
            // put pins for midi into uart mode
            let midi_uart_pins = (
                pins.gpio16.into_mode::<FunctionUart>(),
                pins.gpio17.into_mode::<FunctionUart>(),
            );

            // make a uart peripheral on the given pins
            let uart_config = UartConfig::new(31_250.Hz(), DataBits::Eight, None, StopBits::One);
            let mut midi_uart =
                UartPeripheral::new(device.UART0, midi_uart_pins, &mut device.RESETS)
                    .enable(uart_config, peripheral_clock.freq())
                    .expect("enabling uart for midi should succeed");

            // configure uart interrupt to fire on midi input
            midi_uart.enable_rx_interrupt();

            // split the uart into rx and tx channels 
            let (midi_reader, midi_writer) = midi_uart.split();
            //
            // create MidiIn/Out interfaces
            (EmbeddedMidiIn::new(midi_reader), EmbeddedMidiOut::new(midi_writer))
        }

        fn display(device: &mut PacPeripherals, pins: &Pins, peripheral_clock: &PeripheralClock) -> Display {
            // configure i2c pins
            let sda_pin = pins.gpio26.into_mode::<FunctionI2C>();
            let scl_pin = pins.gpio27.into_mode::<FunctionI2C>();

            // create i2c driver
            let i2c = I2C::i2c1(
                device.I2C1,
                sda_pin,
                scl_pin,
                1.MHz(),
                &mut device.RESETS,
                peripheral_clock,
            );

            // configure pins
            let sda_pin: DisplaySdaPin = sda_pin.try_into().expect("should be able to convert SDA pin into a DisplaySdaPin");
            let scl_pin: DisplaySclPin = scl_pin.try_into().expect("should be able to convert SCL pin into a DisplaySclPin");

            // create i2c display interface
            let mut display = Ssd1306::new(
                I2CDisplayInterface::new_alternate_address(i2c),
                DisplaySize128x64,
                DisplayRotation::Rotate0,
            )
            .into_buffered_graphics_mode();

            // intialise display
            display.init().expect("init: display initialisation failed");

            display
        }
    
        pub fn buttons(device: &mut PacPeripherals, pins: &Pins) -> ButtonArray {
            let button_track_pin = pins.gpio0.into_pull_up_input();
            let button_rhythm_pin = pins.gpio1.into_pull_up_input();
            let button_melody_pin = pins.gpio2.into_pull_up_input();
            button_track_pin.set_interrupt_enabled(EdgeLow, true);
            button_rhythm_pin.set_interrupt_enabled(EdgeLow, true);
            button_melody_pin.set_interrupt_enabled(EdgeLow, true);
            (button_track_pin, button_rhythm_pin, button_melody_pin)
        }
        
        pub fn encoders(device: &mut PacPeripherals, pins: &Pins) -> EncoderArray {
            let mut encoders = Vec::new();
            encoders.push(PositionalEncoder::new(pins.gpio9.into(), pins.gpio10.into()));
            encoders.push(PositionalEncoder::new(pins.gpio11.into(), pins.gpio12.into()));
            encoders.push(PositionalEncoder::new(pins.gpio13.into(), pins.gpio14.into()));
            encoders.push(PositionalEncoder::new(pins.gpio3.into(), pins.gpio4.into()));
            encoders.push(PositionalEncoder::new(pins.gpio5.into(), pins.gpio6.into()));
            encoders.push(PositionalEncoder::new(pins.gpio7.into(), pins.gpio8.into()));
            EncoderArray::new(encoders)
        }
    }

    // RTIC app module runs the app as a set of concurrent tasks modifying shared state
    // this module is responsible for interfacing with the hardware
    #[rtic::app(
        device = rp_pico::hal::pac,
        peripherals = true,
        dispatchers = [USBCTRL_IRQ, DMA_IRQ_0, DMA_IRQ_1, PWM_IRQ_WRAP]
    )]
    mod app {
        use defmt::{self, error, info, trace};
        use defmt_rtt as _;
        use heapless::Vec;
        use midi_types::MidiMessage;
        use nb::block;
        use rp_pico::hal::{gpio::Interrupt::EdgeLow, timer::{monotonic::Monotonic, Alarm0}};

        use super::{
            display,
            encoder::encoder_array::EncoderArray,
            input::{self, InputMode},
            midi,
            peripherals::{ButtonTrackPin, ButtonRhythmPin, ButtonMelodyPin, Display, MidiIn, MidiOut, Peripherals},
            sequencer::Track,
        };

        /// Configure how many tracks are available.
        const TRACK_COUNT: usize = 16;

        /// Define RTIC monotonic timer. Also used for defmt.
        #[monotonic(binds = TIMER_IRQ_0, default = true)]
        type TimerMonotonic = Monotonic<Alarm0>;
        type TimerMonotonicInstant = <TimerMonotonic as rtic::rtic_monotonic::Monotonic>::Instant;
        
        /// RTIC shared resources.
        #[shared]
        struct Shared {
            /// Are we playing, or not?
            playing: bool,

            /// Current page of the UI.
            input_mode: InputMode,

            /// Set of tracks for the sequencer to play.
            tracks: Vec<Option<Track>, TRACK_COUNT>,

            /// The currently selected track.  
            current_track: & 'static Option<Track>,
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
        }

        /// RTIC init method sets up the hardware and initialises shared and local resources.
        #[init]
        fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
            info!("[init] hello world!");

            // configure RTIC monotonic as source of timestamps for defmt
            defmt::timestamp!("{=u64:us}", {
                monotonics::now().duration_since_epoch().to_micros()
            });

            // initialise our tracks
            let mut tracks = Vec::new();
            for _ in 0..TRACK_COUNT {
                tracks.push(Some(Track::new())).expect("inserting track into tracks vector should succeed");
            }

            // create a device wrapper instance and grab some of the peripherals we need
            let periph = Peripherals::new(&mut ctx.device);
            let (button_track_pin, button_rhythm_pin, button_melody_pin) = periph.buttons;

            info!("[init] spawning tasks");

            // start scheduled task to read encoders
            read_encoders::spawn().expect("read_encoders::spawn should succeed");

            // start scheduled task to update display
            render_display::spawn().expect("render_display::spawn should succeed");

            info!("[init] complete 🤘");

            (
                Shared {
                    playing: false,
                    input_mode: InputMode::Track,
                    tracks,
                    current_track: &mut tracks[0],
                },
                Local {
                    midi_in: periph.midi_in,
                    midi_out: periph.midi_out,
                    display: periph.display,
                    button_track_pin,
                    button_rhythm_pin,
                    button_melody_pin,
                    encoders: periph.encoders,
                },
                init::Monotonics(periph.monotonic_timer),
            )
        }

        /// Handle MIDI input. Triggered by a byte being received on UART0.
        #[task(
            binds = UART0_IRQ,
            priority = 4,
            shared = [playing],
            local = [midi_in]
        )]
        fn uart0_irq(mut ctx: uart0_irq::Context) {
            // read those sweet sweet midi bytes!
            // TODO do we need the block! here?
            if let Ok(message) = block!(ctx.local.midi_in.read()) { 
                match message {
                    MidiMessage::TimingClock => {
                        trace!("[midi] clock");

                        // spawn task to advance tracks and potentially generate midi output
                        ctx.shared.playing.lock(|playing| {
                            if *playing {
                                sequencer_advance::spawn()
                                    .expect("sequencer_advance::spawn() should succeed");
                            }
                        });
                    }
                    MidiMessage::Start => {
                        info!("[midi] start");
                        ctx.shared.playing.lock(|playing| {
                            *playing = true;
                        });
                    }
                    MidiMessage::Stop => {
                        info!("[midi] stop");
                        ctx.shared.playing.lock(|playing| {
                            *playing = false;
                        });
                    }
                    MidiMessage::Continue => {
                        info!("[midi] continue");
                        ctx.shared.playing.lock(|playing| {
                            *playing = true;
                        });
                    }
                    _ => trace!("[midi] UNKNOWN"),
                }

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

        #[task(
            priority = 2,
            shared = [tracks],
            local = []
        )]
        fn sequencer_advance(ctx: sequencer_advance::Context) {
            panic!("TODO");
        }

        /// Handle interrupts caused by button presses and update the `input_mode` shared resource.
        #[task(
            binds = IO_IRQ_BANK0,
            priority = 4,
            shared = [input_mode],
            local = [button_track_pin, button_rhythm_pin, button_melody_pin]
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
            if ctx.local.button_rhythm_pin.interrupt_status(EdgeLow) {
                info!("[RHYTHM] pressed");
                ctx.shared.input_mode.lock(|input_mode| {
                    *input_mode = InputMode::Rhythm;
                });
                ctx.local.button_rhythm_pin.clear_interrupt(EdgeLow);
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
            shared = [input_mode, current_track],
            local = [encoders],
        )]
        fn read_encoders(ctx: read_encoders::Context) {
            if let Some(_changes) = ctx.local.encoders.update() {
                (ctx.shared.current_track, ctx.shared.input_mode).lock(|current_track, input_mode| {
                    input::map_encoder_input(*current_track, *input_mode, ctx.local.encoders.take_values());
                })
            }
        }

        #[task(
            priority = 1,
            shared = [playing, input_mode, current_track],
            local = [display]
        )]
        fn render_display(mut ctx: render_display::Context) {
            (ctx.shared.playing, ctx.shared.input_mode, ctx.shared.current_track).lock(|playing, input_mode, current_track| {
                display::render(ctx.local.display, current_track, *input_mode, *playing);
            });
        }

        // idle task needed because default RTIC idle task calls wfi(), which breaks rtt
        // TODO disable in release mode
        #[idle]
        fn task_main(_: task_main::Context) -> ! {
            loop {
                cortex_m::asm::nop();
            }
        }
    }
}
