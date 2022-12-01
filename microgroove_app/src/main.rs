#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use panic_probe as _;

pub mod encoder {
    pub mod positional_encoder {
        use core::fmt::Debug;
        use defmt::error;
        use rotary_encoder_hal::{Direction, Rotary};
        use rp_pico::hal::gpio::DynPin;

        pub struct PositionalEncoder {
            encoder: Rotary<DynPin, DynPin>,
            value: i8,
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
            pub fn update(&mut self) -> Option<i8> {
                match self.encoder.update() {
                    Ok(Direction::Clockwise) => {
                        self.value += 1;
                        Some(self.value)
                    }
                    Ok(Direction::CounterClockwise) => {
                        self.value += 1;
                        Some(self.value)
                    }
                    Ok(Direction::None) => None,
                    Err(_error) => {
                        error!("could not update encoder");
                        None
                    }
                }
            }

            /// Get the value of the encoder, and then reset that to zero. This has the
            /// semantics of "I would like to know your value, which I will use to update my
            /// state, so you can then discard it."
            pub fn take_value(&mut self) -> i8 {
                let val = self.value;
                self.value = 0;
                val
            }
        }

        impl Debug for PositionalEncoder {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "encoder")
            }
        }
    }

    pub mod encoder_array {
        use super::positional_encoder::PositionalEncoder;
        use heapless::Vec;

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
                let any_changes = self
                    .encoders
                    .iter_mut()
                    .map(|enc| enc.update())
                    .any(|opt| opt.is_some());
                if any_changes {
                    Some(())
                } else {
                    None
                }
            }

            pub fn take_values(&mut self) -> Vec<i8, ENCODER_COUNT> {
                self.encoders
                    .iter_mut()
                    .map(|enc| enc.take_value())
                    .collect()
            }
        }
    }
}

/// Rendering UI graphics to the display.
pub mod display {
    use core::iter::zip;

    use display_interface::DisplayError;
    use embedded_graphics::{
        mono_font::{
            ascii::{FONT_4X6, FONT_8X13_ITALIC},
            MonoTextStyle,
        },
        pixelcolor::BinaryColor,
        prelude::*,
        primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
        text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
    };

    use microgroove_sequencer::{core::Track, params::ParamList};
    use crate::{input::InputMode, peripherals::Display};

    type DisplayResult = Result<(), DisplayError>;

    const DISPLAY_WIDTH: i32 = 128;
    const DISPLAY_HEIGHT: i32 = 64;
    const DISPLAY_CENTER: i32 = DISPLAY_WIDTH / 2;

    const CHAR_HEIGHT: u32 = 7;

    const WARNING_Y_POS: i32 = 20;
    const WARNING_PADDING: i32 = 5;
    const WARNING_BORDER: u32 = 2;

    const HEADER_WIDTH: u32 = DISPLAY_WIDTH as u32;
    const HEADER_HEIGHT: u32 = 5;
    const HEADER_PLAYING_ICON_X_POS: i32 = 24;

    const SEQUENCE_X_POS: i32 = 0;
    const SEQUENCE_Y_POS: i32 = HEADER_HEIGHT as i32 + 1;
    const SEQUENCE_WIDTH: u32 = DISPLAY_WIDTH as u32;
    const SEQUENCE_HEIGHT: u32 = 45;
    const SEQUENCE_UNDERLINE_Y_POS: i32 = 44;

    const PARAM_Y_POS: u32 = 51;

    fn map_to_range(x: u32, in_min: u32, in_max: u32, out_min: u32, out_max: u32) -> u32 {
        (x - in_min) * (out_max - out_min + 1) / (in_max - in_min + 1) + out_min
    }

    /// Show snazzy splash screen.
    pub fn render_splash_screen_view(display: &mut Display) -> DisplayResult {
        display.clear();
        Text::with_text_style(
            "MICROGROOVE",
            Point::new(DISPLAY_CENTER, WARNING_Y_POS),
            big_character_style(),
            centered(),
        )
        .draw(display)?;
        Text::with_baseline(
            "I wanna go bang",
            Point::new(37, 42),
            default_character_style(),
            Baseline::Top,
        )
        .draw(display)?;
        display.flush()?;
        Ok(())
    }

    pub fn render_perform_view(
        display: &mut Display,
        track: &Option<Track>,
        input_mode: InputMode,
        playing: bool,
        active_step_num: Option<u32>,
    ) -> DisplayResult {
        draw_header(display, playing, input_mode)?;
        if let Some(track) = track {
            draw_sequence(display, track, active_step_num.unwrap())?;
            draw_params(display, input_mode, track)?;
        } else {
            draw_disabled_track_warning(display)?;
        }
        display.flush()?;
        Ok(())
    }

    fn draw_header(
        display: &mut Display,
        playing: bool,
        input_mode: InputMode,
    ) -> DisplayResult {
        Rectangle::new(Point::zero(), Size::new(HEADER_WIDTH, HEADER_HEIGHT))
            .into_styled(background_style())
            .draw(display)?;
        Text::with_text_style("TRK", Point::zero(), default_character_style(), centered())
            .draw(display)?;
        if playing {
            Text::with_baseline(
                ">",
                Point::new(HEADER_PLAYING_ICON_X_POS, 0),
                default_character_style(),
                Baseline::Top,
            )
            .draw(display)?;
        }
        let title = match input_mode {
            InputMode::Track => "TRACK",
            InputMode::Rhythm => "RHYTHM",
            InputMode::Melody => "MELODY",
        };
        Text::with_text_style(
            title,
            Point::new(DISPLAY_CENTER, 0),
            default_character_style(),
            centered(),
        )
        .draw(display)?;
        match input_mode {
            InputMode::Track => { /* don't do nuffink */ }
            InputMode::Rhythm | InputMode::Melody => {
                let machine_name = "MACHINE_NAME";
                Text::with_text_style(
                    machine_name,
                    Point::new(DISPLAY_WIDTH, 0),
                    default_character_style(),
                    right_align(),
                )
                .draw(display)?;
            }
        }
        Ok(())
    }

    fn draw_disabled_track_warning(display: &mut Display) -> DisplayResult {
        warning(display, "TRACK DISABLED")
    }

    fn draw_sequence(
        display: &mut Display,
        track: &Track,
        active_step_num: u32,
    ) -> DisplayResult {
        let step_width: u32 = if track.length < 17 { 6 } else { 3 };
        let step_height: u32 = step_width;
        let display_sequence_margin_left =
            (DISPLAY_WIDTH - (track.length as i32 * (step_width as i32 + 1))) / 2;
        let note_min: u8 = track
            .steps
            .iter()
            .min()
            .unwrap()
            .as_ref()
            .unwrap()
            .note
            .into();
        let note_max: u8 = track
            .steps
            .iter()
            .max()
            .unwrap()
            .as_ref()
            .unwrap()
            .note
            .into();
        let note_y_pos_min: u32 = 35;
        let note_y_pos_max: u32 = 9 + step_height as u32;
        let step_size = Size::new(step_width, step_height);
        let mut step_num: u32 = 0;

        // erase sequence region of display
        Rectangle::new(
            Point::new(SEQUENCE_X_POS, SEQUENCE_Y_POS),
            Size::new(SEQUENCE_WIDTH, SEQUENCE_HEIGHT),
        )
        .into_styled(background_style())
        .draw(display)?;

        for step in &track.steps {
            if let Some(step) = step {
                let x =
                    display_sequence_margin_left + (step_num as i32 * (step_width as i32 + 1));
                let x2 = x + step_width as i32;
                let note_num: u8 = step.note.into();
                let y = map_to_range(
                    note_num as u32,
                    note_min as u32,
                    note_max as u32,
                    note_y_pos_min,
                    note_y_pos_max,
                );

                // draw step
                let step_style = if step_num == active_step_num {
                    outline_style()
                } else {
                    filled_style()
                };
                Rectangle::new(Point::new(x as i32, y as i32), step_size)
                    .into_styled(step_style)
                    .draw(display)?;

                // draw step underline
                Line::new(
                    Point::new(x, SEQUENCE_UNDERLINE_Y_POS),
                    Point::new(x2, SEQUENCE_UNDERLINE_Y_POS),
                )
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display)?;
            }
            step_num += 1;
        }

        Ok(())
    }

    fn draw_params(
        display: &mut Display,
        input_mode: InputMode,
        track: &Track,
    ) -> DisplayResult {
        let params = match input_mode {
            InputMode::Track => track.params(),
            InputMode::Rhythm => track.rhythm_machine.params(),
            InputMode::Melody => track.melody_machine.params(),
        };
        draw_param_table(display, input_mode, params)
    }

    fn draw_param_table(
        display: &mut Display,
        input_mode: InputMode,
        params: &ParamList,
    ) -> DisplayResult {
        let is_track = match input_mode {
            InputMode::Track => true,
            _ => false,
        };

        let col_content_width = 40;
        let col_padding = 8;
        let col_width = col_content_width + col_padding;

        let name0_x: i32 = 0;
        let name1_x: i32 = if is_track { 60 } else { col_width };
        let name2_x: i32 = if is_track { 96 } else { col_width * 2 };

        let value0_x: i32 = if is_track {
            51
        } else {
            name0_x + col_content_width
        };
        let value1_x: i32 = if is_track {
            88
        } else {
            name1_x + col_content_width
        };
        let value2_x: i32 = DISPLAY_WIDTH;

        let row0_y = PARAM_Y_POS as i32;
        let row1_y = (PARAM_Y_POS + CHAR_HEIGHT) as i32;

        let param_name_points = [
            Point::new(name0_x, row0_y),
            Point::new(name1_x, row0_y),
            Point::new(name2_x, row0_y),
            Point::new(name0_x, row1_y),
            Point::new(name1_x, row1_y),
            Point::new(name2_x, row1_y),
        ];
        let param_value_points = [
            Point::new(value0_x, row0_y),
            Point::new(value1_x, row0_y),
            Point::new(value2_x, row0_y),
            Point::new(value0_x, row1_y),
            Point::new(value1_x, row1_y),
            Point::new(value2_x, row1_y),
        ];
        let params = zip(params, zip(param_name_points, param_value_points));

        Rectangle::new(
            Point::new(0, PARAM_Y_POS as i32),
            Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32 - PARAM_Y_POS),
        )
        .into_styled(background_style())
        .draw(display)?;

        for (param, (name_point, value_point)) in params {
            Text::with_baseline(
                param.name(),
                name_point,
                default_character_style(),
                Baseline::Top,
            )
            .draw(display)?;
            Text::with_text_style(
                param.value_str().as_str(),
                value_point,
                default_character_style(),
                right_align(),
            )
            .draw(display)?;
        }

        Ok(())
    }

    fn default_character_style<'a>() -> MonoTextStyle<'a, BinaryColor> {
        MonoTextStyle::new(&FONT_4X6, BinaryColor::On)
    }

    fn big_character_style<'a>() -> MonoTextStyle<'a, BinaryColor> {
        MonoTextStyle::new(&FONT_8X13_ITALIC, BinaryColor::On)
    }

    fn background_style() -> PrimitiveStyle<BinaryColor> {
        PrimitiveStyle::with_fill(BinaryColor::Off)
    }

    fn filled_style() -> PrimitiveStyle<BinaryColor> {
        PrimitiveStyle::with_fill(BinaryColor::On)
    }

    fn outline_style() -> PrimitiveStyle<BinaryColor> {
        PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(1)
            .fill_color(BinaryColor::Off)
            .build()
    }

    fn fat_outline_style() -> PrimitiveStyle<BinaryColor> {
        PrimitiveStyleBuilder::new()
            .stroke_color(BinaryColor::On)
            .stroke_width(WARNING_BORDER)
            .fill_color(BinaryColor::Off)
            .build()
    }

    fn centered() -> TextStyle {
        TextStyleBuilder::new()
            .alignment(Alignment::Center)
            .baseline(Baseline::Top)
            .build()
    }

    fn right_align() -> TextStyle {
        TextStyleBuilder::new()
            .alignment(Alignment::Right)
            .baseline(Baseline::Top)
            .build()
    }

    fn warning(display: &mut Display, text: &str) -> DisplayResult {
        let char_width = 8; // assumes FONT_8X13_ITALIC
        let char_height = 13; // assumes FONT_8X13_ITALIC
        let space_width = 1; // TODO check this
        let text_width = ((text.len() * char_width)
            + ((text.len() - 1) * space_width)
            + (WARNING_PADDING as usize * 2)) as i32;
        let text_margin_left = (DISPLAY_WIDTH - text_width) / 2;
        let warning_width = DISPLAY_WIDTH - (text_margin_left * 2);
        let warning_height = char_height + WARNING_PADDING * 2;
        let warning_text_y_pos = WARNING_Y_POS + WARNING_PADDING + WARNING_BORDER as i32;
        Rectangle::new(
            Point::new(text_margin_left, WARNING_Y_POS),
            Size::new(warning_width as u32, warning_height as u32),
        )
        .into_styled(fat_outline_style())
        .draw(display)?;
        Text::with_text_style(
            text,
            Point::new(DISPLAY_CENTER, warning_text_y_pos),
            big_character_style(),
            centered(),
        )
        .draw(display)?;
        Ok(())
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
    use microgroove_sequencer::sequencer::{self, Sequencer};
    use crate::encoder::encoder_array::ENCODER_COUNT;
    use heapless::Vec;
    use core::iter::zip;

    #[derive(Clone, Copy, Debug)]
    pub enum InputMode {
        Track,
        Rhythm,
        Melody,
    }

    /// Iterate over `encoder_values` and pass to either `Track`, `RhythmMachine` or
    /// `MelodyMachine`, determined by `input_mode`.
    pub fn map_encoder_input(
        input_mode: InputMode,
        sequencer: &mut Sequencer,
        encoder_values: Vec<i8, ENCODER_COUNT>,
    ) {
        let opt_track = sequencer.current_track_mut();
        opt_track.get_or_insert_with(|| sequencer::new_track_with_default_machines());
        let track = opt_track.as_mut().unwrap();
        let params_mut = match input_mode {
            InputMode::Track => track.params_mut(),
            InputMode::Rhythm => track.rhythm_machine.params_mut(),
            InputMode::Melody => track.melody_machine.params_mut(),
        };

        // update params
        let params_and_values = zip(params_mut, encoder_values);
        for (param, value) in params_and_values {
            param.increment(value);
        }

        // write param data back to track member variables and set the current track in the
        // sequencer
        if let InputMode::Track = input_mode {
            let track_num = (track.params()[2].value_i8().unwrap() - 1) as u8;
            track.apply_params();
            sequencer.set_current_track(track_num);
        }
    }
}

/// Device initialisation and interfacing.
pub mod peripherals {
    use super::encoder::{encoder_array::EncoderArray, positional_encoder::PositionalEncoder};
    use embedded_midi;
    use fugit::{HertzU32, RateExtU32};
    use heapless::Vec;
    use rp2040_hal::clocks::PeripheralClock;
    use rp_pico::{
        hal::{
            clocks::{self, Clock},
            gpio::{
                pin::bank0::{Gpio0, Gpio1, Gpio16, Gpio17, Gpio2, Gpio26, Gpio27},
                FunctionI2C, FunctionUart,
                Interrupt::EdgeLow,
                Pin, PullUpInput,
            },
            pac::{self, I2C1, RESETS, TIMER, UART0},
            sio::Sio,
            timer::{monotonic::Monotonic, Alarm0},
            uart::{DataBits, Reader, StopBits, UartConfig, UartPeripheral, Writer},
            Timer, Watchdog, I2C,
        },
        Pins, XOSC_CRYSTAL_FREQ,
    };
    use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

    // type alias for UART pins
    type MidiOutUartPin = Pin<Gpio16, FunctionUart>;
    type MidiInUartPin = Pin<Gpio17, FunctionUart>;
    type MidiUartPins = (MidiOutUartPin, MidiInUartPin);

    // microgroove-specific midi in/out channel types
    pub type MidiIn = embedded_midi::MidiIn<Reader<UART0, MidiUartPins>>;
    pub type MidiOut = embedded_midi::MidiOut<Writer<UART0, MidiUartPins>>;

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

    pub fn setup(
        mut pac: pac::Peripherals,
    ) -> (
        MidiIn,
        MidiOut,
        Display,
        ButtonArray,
        EncoderArray,
        Monotonic<Alarm0>,
    ) {
        // setup gpio pins
        let sio = Sio::new(pac.SIO);
        let pins = Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        // setup clocks
        let mut watchdog = Watchdog::new(pac.WATCHDOG);
        let clocks = clocks::init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        )
        .ok()
        .expect("init: init_clocks_and_plls(...) should succeed");

        let (midi_in, midi_out) = new_midi_uart(
            pac.UART0,
            pins.gpio16.into_mode::<FunctionUart>(),
            pins.gpio17.into_mode::<FunctionUart>(),
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
        );

        let display = new_display(
            pac.I2C1,
            pins.gpio26.into_mode::<FunctionI2C>(),
            pins.gpio27.into_mode::<FunctionI2C>(),
            &mut pac.RESETS,
            &clocks.peripheral_clock,
        );

        // setup buttons
        let button_track_pin = pins.gpio0.into_pull_up_input();
        let button_rhythm_pin = pins.gpio1.into_pull_up_input();
        let button_melody_pin = pins.gpio2.into_pull_up_input();
        button_track_pin.set_interrupt_enabled(EdgeLow, true);
        button_rhythm_pin.set_interrupt_enabled(EdgeLow, true);
        button_melody_pin.set_interrupt_enabled(EdgeLow, true);
        let buttons = (button_track_pin, button_rhythm_pin, button_melody_pin);

        let mut encoder_vec = Vec::new();
        encoder_vec
            .push(PositionalEncoder::new(
                pins.gpio9.into(),
                pins.gpio10.into(),
            ))
            .expect("failed to create encoder");
        encoder_vec
            .push(PositionalEncoder::new(
                pins.gpio11.into(),
                pins.gpio12.into(),
            ))
            .unwrap();
        encoder_vec
            .push(PositionalEncoder::new(
                pins.gpio13.into(),
                pins.gpio14.into(),
            ))
            .unwrap();
        encoder_vec
            .push(PositionalEncoder::new(pins.gpio3.into(), pins.gpio4.into()))
            .unwrap();
        encoder_vec
            .push(PositionalEncoder::new(pins.gpio5.into(), pins.gpio6.into()))
            .unwrap();
        encoder_vec
            .push(PositionalEncoder::new(pins.gpio7.into(), pins.gpio8.into()))
            .unwrap();
        let encoders = EncoderArray::new(encoder_vec);

        (
            midi_in,
            midi_out,
            display,
            buttons,
            encoders,
            new_monotonic_timer(pac.TIMER, &mut pac.RESETS),
        )
    }

    fn new_monotonic_timer(timer: TIMER, resets: &mut RESETS) -> Monotonic<Alarm0> {
        // setup monotonic timer for rtic
        let mut timer = Timer::new(timer, resets);
        let monotonic_alarm = timer.alarm_0().unwrap();
        Monotonic::new(timer, monotonic_alarm)
    }

    fn new_midi_uart(
        uart: UART0,
        out_pin: MidiOutUartPin,
        in_pin: MidiInUartPin,
        resets: &mut RESETS,
        peripheral_clock_freq: HertzU32,
    ) -> (MidiIn, MidiOut) {
        let midi_uart_pins = (out_pin, in_pin);
        let uart_config = UartConfig::new(31_250.Hz(), DataBits::Eight, None, StopBits::One);
        let mut midi_uart = UartPeripheral::new(uart, midi_uart_pins, resets)
            .enable(uart_config, peripheral_clock_freq)
            .expect("enabling uart for midi should succeed");
        midi_uart.enable_rx_interrupt();
        let (midi_reader, midi_writer) = midi_uart.split();
        (
            embedded_midi::MidiIn::new(midi_reader),
            embedded_midi::MidiOut::new(midi_writer),
        )
    }

    fn new_display(
        i2c: I2C1,
        sda_pin: DisplaySdaPin,
        scl_pin: DisplaySclPin,
        resets: &mut RESETS,
        peripheral_clock: &PeripheralClock,
    ) -> Display {
        let i2c_bus = I2C::i2c1(i2c, sda_pin, scl_pin, 1.MHz(), resets, peripheral_clock);

        let mut display = Ssd1306::new(
            I2CDisplayInterface::new_alternate_address(i2c_bus),
            DisplaySize128x64,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics_mode();

        display.init().expect("init: display initialisation failed");

        display
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
    use alloc_cortex_m::CortexMHeap;
    use defmt::{self, error, info, trace};
    use defmt_rtt as _;
    use midi_types::MidiMessage;
    use nb::block;
    use rp_pico::hal::{
        gpio::Interrupt::EdgeLow,
        timer::{monotonic::Monotonic, Alarm0},
    };

    use microgroove_sequencer::sequencer::{ScheduledMidiMessage, Sequencer};
    use crate::{
        display,
        encoder::encoder_array::EncoderArray,
        input::{self, InputMode},
        midi,
        peripherals::{
            setup, ButtonMelodyPin, ButtonRhythmPin, ButtonTrackPin, Display, MidiIn, MidiOut,
        },
    };

    #[global_allocator]
    static ALLOCATOR: CortexMHeap = CortexMHeap::empty();
    const HEAP_SIZE_BYTES: usize = 16 * 1024; // 16KB!

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

        /// Pin for button the [RHYTHM] button
        button_rhythm_pin: ButtonRhythmPin,

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
        let (button_track_pin, button_rhythm_pin, button_melody_pin) = buttons;

        // show a splash screen for a bit
        display::render_splash_screen_view(&mut display).unwrap();

        info!("[init] spawning tasks");

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
                button_rhythm_pin,
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
        shared = [input_mode, sequencer],
        local = [encoders],
    )]
    fn read_encoders(ctx: read_encoders::Context) {
        if let Some(_changes) = ctx.local.encoders.update() {
            (ctx.shared.input_mode, ctx.shared.sequencer).lock(|input_mode, sequencer| {
                input::map_encoder_input(
                    *input_mode,
                    sequencer,
                    ctx.local.encoders.take_values(),
                );
            })
        }
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
                track,
                *input_mode,
                sequencer.is_playing(),
                sequencer.current_track_active_step_num(),
            )
            .unwrap();
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

    // OOM handler
    #[alloc_error_handler]
    fn alloc_error(_layout: core::alloc::Layout) -> ! {
        error!("TICK TICK TICK TICK OOM!");
        cortex_m::asm::bkpt();
        loop {}
    }
}
