/// Machine which generates random note pitch values.
use super::Machine;
use crate::{
    machine_resources::MachineResources,
    map_to_range,
    midi::Note,
    param::{Param, ParamList, ParamValue},
    Sequence,
};

use alloc::boxed::Box;

#[derive(Debug)]
pub struct RandMelodyMachine {
    params: ParamList,
}

impl RandMelodyMachine {
    pub fn new() -> RandMelodyMachine {
        RandMelodyMachine {
            params: ParamList::from_slice(&[
                Box::new(Param::new_note_param("ROOT")),
                Box::new(Param::new_number_param("RANGE", 1, 60, 12)),
            ])
            .unwrap(),
        }
    }

    fn process(
        sequence: Sequence,
        machine_resources: &mut MachineResources,
        root: Note,
        range: u8,
    ) -> Sequence {
        let min_note = Into::<u8>::into(root) as i32;
        let max_note: i32 = min_note + range as i32 - 1;
        let rand = machine_resources.random_u64();
        let notes = (0..8)
            .map(|i| ((rand >> i) & 127) as i32)
            .map(|rand_note_num| map_to_range(rand_note_num, 0, 127, min_note, max_note) as u8)
            .map(|note_num| note_num.try_into().expect("note number should go into note"));
        sequence.set_notes(notes)
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
        Self::process(sequence, machine_resources, root, range)
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
