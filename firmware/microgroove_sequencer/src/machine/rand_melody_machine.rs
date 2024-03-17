/// Machine which generates random note pitch values.
use super::Machine;
use crate::{
    machine_resources::MachineResources,
    map_to_range,
    midi::Note,
    param::{Param, ParamList},
    Sequence,
};

use alloc::boxed::Box;

#[derive(Debug)]
pub struct RandMelodyMachine {
    params: ParamList,
    seed: u64,
}

impl RandMelodyMachine {
    pub fn new() -> RandMelodyMachine {
        let params = ParamList::from_slice(&[
            Box::new(Param::new_note_param("ROOT")),
            Box::new(Param::new_number_param("RANGE", 1, 60, 12)),
        ])
        .expect("should create rand melody machine param list from slice");
        RandMelodyMachine { params, seed: 0 }
    }

    fn process(sequence: Sequence, root: Note, range: u8, seed: u64) -> Sequence {
        let min_note = Into::<u8>::into(root) as i32;
        let max_note: i32 = min_note + range as i32 - 1;
        let mut i = 0;
        sequence.map_notes(|_| {
            let rand_note_num = ((seed >> i) & 127) as i32;
            let note_num = map_to_range(rand_note_num, 0, 127, min_note, max_note) as u8;
            i += 1;
            note_num
                .try_into()
                .expect("note number should go into note")
        })
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

    fn generate(&mut self, machine_resources: &mut MachineResources) {
        self.seed = machine_resources.random_u64();
    }

    fn apply(&self, sequence: Sequence) -> Sequence {
        let root = self.params[0]
            .value()
            .try_into()
            .expect("unexpected root param for RandMelodyMachine");
        let range = self.params[1]
            .value()
            .try_into()
            .expect("unexpected range param for RandMelodyMachine");
        Self::process(sequence, root, range, self.seed)
    }
}

unsafe impl Send for RandMelodyMachine {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{machine_resources::MachineResources, sequence_generator::SequenceGenerator};

    #[test]
    fn rand_melody_machine_should_generate_stable_sequence() {
        let mut machine_resources = MachineResources::new();
        let mut machine = RandMelodyMachine::new();
        machine.generate(&mut machine_resources);
        let input_sequence = SequenceGenerator::initial_sequence(8);
        let output_sequence = machine.apply(SequenceGenerator::initial_sequence(8));
        let output_sequence2 = machine.apply(SequenceGenerator::initial_sequence(8));
        assert_ne!(input_sequence, output_sequence);
        assert_eq!(output_sequence, output_sequence2);
    }

    #[test]
    fn rand_melody_machine_should_generate_different_sequences_if_generate_called_twice() {
        let mut machine_resources = MachineResources::new();
        let mut machine = RandMelodyMachine::new();
        machine.generate(&mut machine_resources);
        let input_sequence = SequenceGenerator::initial_sequence(8);
        let output_sequence = machine.apply(SequenceGenerator::initial_sequence(8));
        machine.generate(&mut machine_resources);
        let output_sequence2 = machine.apply(SequenceGenerator::initial_sequence(8));
        assert_ne!(input_sequence, output_sequence);
        assert_ne!(output_sequence, output_sequence2);
    }

    #[test]
    fn rand_melody_machine_should_generate_notes_in_specified_range() {
        let mut machine_resources = MachineResources::new();
        let mut machine = RandMelodyMachine::new();
        machine.generate(&mut machine_resources);
        let root_note: u8 = Note::default().into();
        let max_note = root_note + 11;
        let output_sequence = machine.apply(SequenceGenerator::initial_sequence(8));
        assert!(output_sequence.iter().all(|step| {
            let note: u8 = step.as_ref().unwrap().note.into();
            note >= root_note && note <= max_note
        }));
    }
}
