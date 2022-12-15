extern crate alloc;

use crate::{
    machine::unit_machine::UnitMachine, machine::Machine, machine_resources::MachineResources,
    Sequence, Step,
};

use alloc::boxed::Box;

#[derive(Debug)]
pub struct SequenceGenerator {
    pub groove_machine: Box<dyn Machine>,
    pub melody_machine: Box<dyn Machine>,
}

impl Default for SequenceGenerator {
    fn default() -> SequenceGenerator {
        SequenceGenerator {
            groove_machine: Box::new(UnitMachine::new()),
            melody_machine: Box::new(UnitMachine::new()),
        }
    }
}

impl SequenceGenerator {
    pub fn initial_sequence(length: u8) -> Sequence {
        (0..length).map(|_i| Some(Step::new(60))).collect()
    }

    /// Generate a sequence by piping the initial sequence through the set of configured machines.
    pub fn generate(&self, length: u8, machine_resources: &mut MachineResources) -> Sequence {
        self.melody_machine.apply(
            self.groove_machine
                .apply(Self::initial_sequence(length), machine_resources),
            machine_resources,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_generator_default_should_create_a_new_generator() {
        let generator = SequenceGenerator::default();
        assert_eq!("UNIT", generator.groove_machine.name());
        assert_eq!("UNIT", generator.melody_machine.name());
    }

    #[test]
    fn sequence_generator_apply_should_generate_a_sequence() {
        let generator = SequenceGenerator::default();
        let mut machine_resources = MachineResources::new();
        let sequence = generator.generate(8, &mut machine_resources);
        assert_eq!(8, sequence.len());
        assert!(sequence.iter().all(|step| {
            match step {
                Some(step) => {
                    let note_num: u8 = step.note.into();
                    note_num == 60
                }
                _ => false,
            }
        }));
    }
}
