/// Reference machine which passes sequence input through unmodified.
extern crate alloc;

use super::Machine;
use crate::{
    machine_resources::MachineResources,
    midi::Note,
    param::{Param, ParamList, ParamValue},
    Sequence,
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
        _root: Note,
        _range: u8,
    ) -> Sequence {
        let rand = machine_resources.random_u64();
        let mut read_start_bit = 0;
        for step in sequence.iter_mut() {
            if let Some(step) = step {
                let note_num: u8 = ((rand >> read_start_bit) & 0x80) as u8;
                step.note = note_num.min(127).into();
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
        params
            .push(Box::new(
                Param::new(
                    "ROOT".into(),
                    ParamValue::Note(Note::C3),
                    Note::all_variants()
                        .iter()
                        .map(|note| ParamValue::Note(note.clone()))
                        .collect(),
                )
                .unwrap(),
            ))
            .unwrap();
        params
            .push(Box::new(
                Param::new(
                    "RANGE".into(),
                    ParamValue::Number(12),
                    (1..=60).map(ParamValue::Number).collect(),
                )
                .unwrap(),
            ))
            .unwrap();
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
        assert_ne!(input_sequence, output_sequence);
    }
}
