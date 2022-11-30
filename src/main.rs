#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use panic_probe as _;

mod microgroove {
    /// Model parameters as mutable values with metadata (name)
    pub mod params {
        extern crate alloc;
        use alloc::boxed::Box;
        use core::fmt::Debug;
        use defmt::debug;
        use heapless::Vec;

        pub trait Param: Debug + Send {
            fn name(&self) -> &str {
                "DISABLED"
            }
            fn value_str(&self) -> &str;
            fn increment(&mut self, n: u32);
        }

        pub trait ParamAdapter {
            fn apply(&mut self) {}
        }

        pub type ParamList = Vec<Box<dyn Param>, 6>;

        #[derive(Debug)]
        pub struct DummyParam {}

        impl DummyParam {
            pub fn new() -> DummyParam {
                DummyParam {}
            }
        }

        impl Param for DummyParam {
            fn name(&self) -> &str {
                "DUMMY"
            }
            fn value_str(&self) -> &str {
                "DUMMY"
            }
            fn increment(&mut self, _n: u32) {
                debug!("DummyParam::increment");
            }
        }
    }

    /// Core data model for a MIDI sequencer. Provides types to represent a sequencer as a set of
    /// tracks, each with a Sequence of Steps. A Step consists of the basic information required to
    /// play a note.
    pub mod sequence {
        extern crate alloc;
        use alloc::boxed::Box;
        use core::cmp::Ordering;
        use core::fmt::Debug;
        use heapless::Vec;
        use midi_types::{Channel, Note, Value14, Value7};

        use crate::microgroove::params::ParamList;

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
            pub delay: u8,
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

        impl PartialEq for Step {
            fn eq(&self, other: &Self) -> bool {
                let self_note_num: u8 = self.note.into();
                let other_note_num: u8 = other.note.into();
                self_note_num == other_note_num
            }
        }

        impl Eq for Step {}

        impl PartialOrd for Step {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for Step {
            fn cmp(&self, other: &Self) -> Ordering {
                let self_note_num: u8 = self.note.into();
                let other_note_num: u8 = other.note.into();
                self_note_num.cmp(&other_note_num)
            }
        }

        #[derive(Clone, Copy, Debug)]
        pub enum TimeDivision {
            NinetySixth = 1, // corresponds to midi standard of 24 clock pulses per quarter note
            ThirtySecond = 3,
            Sixteenth = 6,
            Eigth = 12,
            Quarter = 24,
            Whole = 96,
        }

        pub type Sequence = Vec<Option<Step>, 32>;

        pub trait SequenceProcessor {
            fn apply(&self, sequence: Sequence) -> Sequence;
        }

        pub trait Machine: Debug + Send {
            fn name(&self) -> &str;
            fn sequence_processor(&self) -> Box<dyn SequenceProcessor>;
            fn params(&self) -> &ParamList;
            fn params_mut(&mut self) -> &mut ParamList;
        }

        #[derive(Debug)]
        pub struct Track {
            pub time_division: TimeDivision,
            pub length: u8,
            pub midi_channel: Channel,
            pub steps: Sequence,
            pub rhythm_machine: Box<dyn Machine>,
            pub melody_machine: Box<dyn Machine>,
            params: ParamList,
        }

        impl Track {
            pub fn new(
                rhythm_machine: impl Machine + 'static,
                melody_machine: impl Machine + 'static,
            ) -> Track {
                Track {
                    time_division: TimeDivision::Sixteenth,
                    length: 16,
                    midi_channel: 0.into(),
                    steps: Track::generate_sequence(),
                    rhythm_machine: Box::new(rhythm_machine),
                    melody_machine: Box::new(melody_machine),
                    params: Vec::new(),
                }
            }

            pub fn params(&self) -> &ParamList {
                &self.params
            }

            pub fn params_mut(&mut self) -> &mut ParamList {
                &mut self.params
            }

            fn generate_sequence() -> Sequence {
                Self::initial_sequence()
            }

            fn initial_sequence() -> Sequence {
                (0..16).map(|_i| Some(Step::new())).collect()
            }

            pub fn should_play_on_tick(&self, tick: u32) -> bool {
                tick % (self.time_division as u32) == 0
            }

            pub fn step_num(&self, tick: u32) -> u32 {
                tick / (self.time_division as u32) % self.length as u32
            }

            pub fn step_at_tick(&self, tick: u32) -> Option<&Step> {
                if !self.should_play_on_tick(tick) {
                    return None;
                }
                self.steps
                    .get(self.step_num(tick) as usize)
                    .unwrap()
                    .as_ref()
            }
        }
    }

    pub mod sequencer {
        extern crate alloc;
        use defmt::trace;
        use embedded_midi::MidiMessage;
        use fugit::{ExtU64, MicrosDurationU64};
        use heapless::{HistoryBuffer, Vec};

        use crate::microgroove::{machines::unitmachine::UnitMachine, sequence::Track};

        /// Configure how many tracks are available.
        const TRACK_COUNT: usize = 16;

        // TODO will cause issues if polyphony
        const MAX_MESSAGES_PER_TICK: usize = TRACK_COUNT * 2;

        const MIDI_HISTORY_SAMPLE_COUNT: usize = 6;

        #[derive(Debug)]
        pub enum ScheduledMidiMessage {
            Immediate(MidiMessage),
            Delayed(MidiMessage, MicrosDurationU64),
        }

        const DEFAULT_BPM: u64 = 130;
        const DEFAULT_TICK_DURATION_US: u64 = (60 / DEFAULT_BPM) / 24;

        pub fn new_track_with_default_machines() -> Track {
            Track::new(UnitMachine::new(), UnitMachine::new())
        }

        pub struct Sequencer {
            pub tracks: Vec<Option<Track>, TRACK_COUNT>,
            current_track: usize,
            playing: bool,
            tick: u32,
            last_tick_instant_us: Option<u64>,
            midi_tick_history: HistoryBuffer<u64, MIDI_HISTORY_SAMPLE_COUNT>,
        }

        impl Sequencer {
            pub fn new() -> Sequencer {
                // create a set of empty tracks
                let mut tracks = Vec::new();
                tracks
                    .push(Some(new_track_with_default_machines()))
                    .expect("inserting track into tracks vector should succeed");
                for _ in 1..TRACK_COUNT {
                    tracks
                        .push(None)
                        .expect("inserting track into tracks vector should succeed");
                }
                Sequencer {
                    tracks,
                    current_track: 0,
                    playing: false,
                    tick: 0,
                    last_tick_instant_us: None,
                    midi_tick_history: HistoryBuffer::<u64, MIDI_HISTORY_SAMPLE_COUNT>::new(),
                }
            }

            pub fn is_playing(&self) -> bool {
                self.playing
            }

            pub fn start_playing(&mut self) {
                self.tick = 0;
                self.playing = true
            }

            pub fn stop_playing(&mut self) {
                self.playing = false;
            }

            pub fn continue_playing(&mut self) {
                self.playing = true
            }

            pub fn current_track(&self) -> &Option<Track> {
                &self.tracks.get(self.current_track).unwrap()
            }

            pub fn current_track_mut(&mut self) -> &mut Option<Track> {
                self.tracks.get_mut(self.current_track).unwrap()
            }

            pub fn current_track_active_step_num(&self) -> Option<u32> {
                self.current_track()
                    .as_ref()
                    .map(|track| track.step_num(self.tick))
            }

            pub fn advance(
                &mut self,
                now_us: u64,
            ) -> Vec<ScheduledMidiMessage, MAX_MESSAGES_PER_TICK> {
                let mut output_messages = Vec::new();
                let tick_duration = self.average_tick_duration(now_us);

                for track in &self.tracks {
                    if let Some(track) = track {
                        if let Some(step) = track.step_at_tick(self.tick) {
                            let note_on_message =
                                MidiMessage::NoteOn(track.midi_channel, step.note, step.velocity);
                            output_messages
                                .push(ScheduledMidiMessage::Immediate(note_on_message))
                                .unwrap();

                            let midi_channel: u8 = track.midi_channel.into();
                            let note: u8 = step.note.into();
                            let velocity: u8 = step.velocity.into();
                            trace!(
                                "Sequencer::advance: note_on channel={} note={} velocity={}",
                                midi_channel,
                                note,
                                velocity
                            );

                            let note_off_message =
                                MidiMessage::NoteOff(track.midi_channel, step.note, 0.into());
                            let note_off_time = ((tick_duration.to_micros()
                                * (track.time_division as u64)
                                * step.length_step_cents as u64)
                                / 100)
                                .micros();
                            output_messages
                                .push(ScheduledMidiMessage::Delayed(
                                    note_off_message,
                                    note_off_time,
                                ))
                                .unwrap();

                            trace!(
                                "Sequencer::advance: scheduling note off message for {}us",
                                note_off_time.to_micros()
                            );
                        }
                    }
                }

                self.tick += 1;

                output_messages
            }

            /// Calculate average time between last k MIDI ticks. Defaults to tick frequency of
            /// 19,230ms, which is equivalent to 130BPM.
            fn average_tick_duration(&mut self, now_us: u64) -> MicrosDurationU64 {
                let mut tick_duration = DEFAULT_TICK_DURATION_US.micros();

                if let Some(last_tick_instant_us) = self.last_tick_instant_us {
                    let last_tick_duration = last_tick_instant_us - now_us;
                    self.midi_tick_history.write(last_tick_duration);
                    tick_duration = (self.midi_tick_history.as_slice().iter().sum::<u64>()
                        / self.midi_tick_history.len() as u64)
                        .micros();
                }

                self.last_tick_instant_us = Some(now_us);

                tick_duration
            }
        }
    }

    pub mod machines {
        pub mod unitmachine {
            extern crate alloc;
            use crate::microgroove::{
                params::{DummyParam, ParamList},
                sequence::{Machine, Sequence, SequenceProcessor},
            };
            use alloc::boxed::Box;

            #[derive(Clone, Copy, Debug)]
            struct UnitProcessor {}

            impl UnitProcessor {
                fn new() -> UnitProcessor {
                    UnitProcessor {}
                }
            }

            impl SequenceProcessor for UnitProcessor {
                fn apply(&self, sequence: Sequence) -> Sequence {
                    sequence
                }
            }

            #[derive(Debug)]
            pub struct UnitMachine {
                sequence_processor: UnitProcessor,
                params: ParamList,
            }

            impl UnitMachine {
                pub fn new() -> UnitMachine {
                    let sequence_processor = UnitProcessor::new();
                    let mut params = ParamList::new();
                    params.push(Box::new(DummyParam::new())).unwrap();
                    UnitMachine {
                        sequence_processor,
                        params,
                    }
                }
            }

            impl Machine for UnitMachine {
                fn name(&self) -> &str {
                    "UNIT"
                }

                fn sequence_processor(&self) -> Box<dyn SequenceProcessor> {
                    Box::new(self.sequence_processor)
                }

                fn params(&self) -> &ParamList {
                    &self.params
                }

                fn params_mut(&mut self) -> &mut ParamList {
                    &mut self.params
                }
            }

            unsafe impl Send for UnitMachine {}
        }
    }

    pub mod encoder {
        pub mod positional_encoder {
            use core::fmt::Debug;
            use defmt::error;
            use rotary_encoder_hal::{Direction, Rotary};
            use rp_pico::hal::gpio::DynPin;

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
                pub fn take_value(&mut self) -> i32 {
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

                pub fn take_values(&mut self) -> Vec<i32, ENCODER_COUNT> {
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

        use crate::microgroove::{
            input::InputMode, params::ParamList, peripherals::Display, sequence::Track,
        };

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
                    param.value_str(),
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
        use crate::microgroove::{
            encoder::encoder_array::ENCODER_COUNT,
            sequencer::{self, Sequencer},
        };
        use heapless::Vec;

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
            _encoder_values: Vec<i32, ENCODER_COUNT>,
        ) {
            let opt_track = sequencer.current_track_mut();
            opt_track.get_or_insert_with(|| sequencer::new_track_with_default_machines());
            let track = opt_track.as_mut().unwrap();
            match input_mode {
                InputMode::Track => {
                    let _params = track.params_mut();
                    // TODO update params
                    // TODO write param data back to track member variables
                }
                InputMode::Rhythm | InputMode::Melody => {
                    // TODO update params
                }
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

        use crate::microgroove::{
            display,
            encoder::encoder_array::EncoderArray,
            input::{self, InputMode},
            midi,
            peripherals::{
                setup, ButtonMelodyPin, ButtonRhythmPin, ButtonTrackPin, Display, MidiIn, MidiOut,
            },
            sequencer::{ScheduledMidiMessage, Sequencer},
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
}
