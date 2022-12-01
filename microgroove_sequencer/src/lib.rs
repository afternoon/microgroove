#![cfg_attr(not(test), no_std)]

/// Model parameters as mutable values with metadata (name)
pub mod params {
    extern crate alloc;

    use alloc::boxed::Box;
    use core::fmt::{Debug, Write};
    use heapless::{String, Vec};

    pub trait Param: Debug + Send {
        fn name(&self) -> &str {
            "DISABLED"
        }
        fn increment(&mut self, n: i8);
        fn value_str(&self) -> String<10>;
        fn value_i8(&self) -> Option<i8> {
            None
        }
    }

    pub trait ParamAdapter {
        fn apply(&mut self) {}
    }

    pub type ParamList = Vec<Box<dyn Param>, 6>;

    #[derive(Clone, Debug)]
    pub struct NumberParam {
        name: String<6>,
        val: i8,
        min: i8,
        max: i8,
    }

    impl NumberParam {
        pub fn new(name: &str, min: i8, max: i8, initial: i8) -> NumberParam {
            NumberParam { name: name.into(), val: initial, min, max }
        }
    }

    impl Param for NumberParam {
        fn name(&self) -> &str {
            self.name.as_str()
        }

        fn value_str(&self) -> String<10> {
            let mut val_string = String::<10>::new();
            let _ = write!(val_string, "{}", self.val);
            val_string
        }

        fn increment(&mut self, n: i8) {
            self.val += n;
            if self.val < self.min { self.val = self.min; }
            else if self.val > self.max { self.val = self.max; }
        }

        fn value_i8(&self) -> Option<i8> {
            Some(self.val)
        }
    }
}

/// Core data model for a MIDI sequencer. Provides types to represent a sequencer as a set of
/// tracks, each with a Sequence of Steps. A Step consists of the basic information required to
/// play a note.
pub mod core {
    extern crate alloc;

    use alloc::boxed::Box;
    use core::cmp::Ordering;
    use core::fmt::Debug;
    use heapless::Vec;
    use midi_types::{Channel, Note, Value14, Value7};

    use crate::params::{NumberParam, ParamList};

    pub const TRACK_COUNT: usize = 16;

    const TRACK_MIN_LENGTH: usize = 1; // because live performance effect of repeating a single step
    const TRACK_MAX_LENGTH: usize = 32;
    const TRACK_DEFAULT_LENGTH: usize = 8; // because techno

    const TRACK_MIN_NUM: i8 = 1;
    const TRACK_DEFAULT_NUM: i8 = 1;

    const MIDI_MIN_CHANNEL: i8 = 1;
    const MIDI_MAX_CHANNEL: i8 = 16;
    const MIDI_DEFAULT_CHANNEL: i8 = 1;

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
        pub fn new(note: u8) -> Step {
            Step {
                note: note.into(),
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

    pub type Sequence = Vec<Option<Step>, TRACK_MAX_LENGTH>;

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
            let mut params: ParamList = Vec::new();
            // params.push(Box::new(RhythmMachineParam::new())).unwrap();
            params.push(Box::new(NumberParam::new("LEN", TRACK_MIN_LENGTH as i8, TRACK_MAX_LENGTH as i8, TRACK_DEFAULT_LENGTH as i8))).unwrap();
            params.push(Box::new(NumberParam::new("TRACK", TRACK_MIN_NUM, TRACK_COUNT as i8, TRACK_DEFAULT_NUM))).unwrap();
            // params.push(Box::new(MelodyMachineParam::new())).unwrap();
            // params.push(Box::new(TrackSpeedParam::new())).unwrap();
            params.push(Box::new(NumberParam::new("CHAN", MIDI_MIN_CHANNEL, MIDI_MAX_CHANNEL, MIDI_DEFAULT_CHANNEL))).unwrap();

            Track {
                time_division: TimeDivision::Sixteenth,
                length: 16,
                midi_channel: 0.into(),
                steps: Track::generate_sequence(),
                rhythm_machine: Box::new(rhythm_machine),
                melody_machine: Box::new(melody_machine),
                params,
            }
        }

        pub fn params(&self) -> &ParamList {
            &self.params
        }

        pub fn params_mut(&mut self) -> &mut ParamList {
            &mut self.params
        }

        pub fn apply_params(&mut self) {
            /*
            0 -> rhythm_machine
            1 -> length
            2 -> track number -- ignore, handled by Sequencer::set_current_track
            3 -> melody_machine
            4 -> speed
            5 -> midi_channel
            */
            panic!("TODO");
        }

        fn generate_sequence() -> Sequence {
            Self::initial_sequence()
            // TODO pipe sequence through machines
        }

        fn initial_sequence() -> Sequence {
            (0..16).map(|_i| Some(Step::new(60))).collect()
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

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn steps_are_correctly_ordered() {
            let (s1, s2) = (
                Step::new(60),
                Step::new(61)
            );
            assert!(s1 < s2);
        }
    }
}

pub mod sequencer {
    extern crate alloc;

    use embedded_midi::MidiMessage;
    use fugit::{ExtU64, MicrosDurationU64};
    use heapless::{HistoryBuffer, Vec};

    use crate::{core::{Track, TRACK_COUNT}, machines::unitmachine::UnitMachine};

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

        pub fn set_current_track(&mut self, new_track_num: u8) {
            self.current_track = new_track_num as usize;
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

        use crate::{
            core::{Machine, Sequence, SequenceProcessor},
            params::{NumberParam, ParamList},
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
                params.push(Box::new(NumberParam::new("NUM", 1, 16, 1))).unwrap();
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
