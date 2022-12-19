/// Reference machine which passes sequence input through unmodified.
extern crate alloc;

use super::Machine;
use crate::{
    machine_resources::MachineResources,
    midi::Note,
    param::{Param, ParamList, ParamValue},
    map_to_range, Sequence,
};

use alloc::boxed::Box;

#[derive(Clone, Copy, Debug)]
struct RandMelodyProcessor;

impl RandMelodyProcessor {
    pub fn new() -> RandMelodyProcessor {
        RandMelodyProcessor {}
    }

    pub fn apply(
        &self,
        mut sequence: Sequence,
        machine_resources: &mut MachineResources,
        root: Note,
        range: u8,
    ) -> Sequence {
        let root_note: u8 = root.into();
        let max_note = root_note + range - 1;
        let rand = machine_resources.random_u64();
        let mut read_start_bit = 0;
        for step in sequence.iter_mut() {
            if let Some(step) = step {
                let note_num = ((rand >> read_start_bit) & 127) as u8;
                step.note = (map_to_range(note_num as i32, 0, 127, root_note as i32, max_note as i32) as u8).into();
                read_start_bit += 1;
            }
        }
        sequence
    }
}

#[derive(Debug)]
pub struct RandMelodyMachine {
    sequence_processor: RandMelodyProcessor,
    params: ParamList,
}

impl RandMelodyMachine {
    pub fn new() -> RandMelodyMachine {
        let sequence_processor = RandMelodyProcessor::new();
        let mut params = ParamList::new();
        params.push(Box::new(Param::new_note_param("ROOT"))).unwrap();
        params.push(Box::new(Param::new_number_param("RANGE", 1, 60, 12))).unwrap();
        RandMelodyMachine {
            sequence_processor,
            params,
        }
    }
}

impl Machine for RandMelodyMachine {
    fn name(&self) -> &str {
        "RAND"
    }

    fn params(&self) -> &ParamList {
        &self.params
    }

    fn params_mut(&mut self) -> &mut ParamList {
        &mut self.params
    }

    fn apply(&self, sequence: Sequence, machine_resources: &mut MachineResources) -> Sequence {
        let root = match self.params[0].value() {
            ParamValue::Note(note) => note,
            unexpected => panic!(
                "RandMelodyMachine got unexpected root param: {:?}",
                unexpected
            ),
        };
        let range = match self.params[1].value() {
            ParamValue::Number(i) => i,
            unexpected => panic!(
                "RandMelodyMachine got unexpected range param: {:?}",
                unexpected
            ),
        };
        self.sequence_processor
            .apply(sequence, machine_resources, root, range)
    }
}

unsafe impl Send for RandMelodyMachine {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{machine_resources::MachineResources, sequence_generator::SequenceGenerator};

    #[test]
    fn rand_melody_machine_should_generate_random_sequences() {
        let mut machine_resources = MachineResources::new();
        let machine = RandMelodyMachine::new();
        let input_sequence = SequenceGenerator::initial_sequence(8);
        let output_sequence = machine.apply(
            SequenceGenerator::initial_sequence(8),
            &mut machine_resources,
        );
        let output_sequence2 = machine.apply(
            SequenceGenerator::initial_sequence(8),
            &mut machine_resources,
        );
        assert_ne!(input_sequence, output_sequence);
        assert_ne!(output_sequence, output_sequence2);
    }

    #[test]
    fn rand_melody_machine_should_generate_notes_in_specified_range() {
        let mut machine_resources = MachineResources::new();
        let machine = RandMelodyMachine::new();
        let root_note: u8 = Note::default().into();
        let max_note = root_note + 11;
        let output_sequence = machine.apply(
            SequenceGenerator::initial_sequence(8),
            &mut machine_resources,
        );
        assert!(output_sequence.iter().all(|step| {
            let note: u8 = step.as_ref().unwrap().note.into();
            note >= root_note && note <= max_note
        }));
    }
}
