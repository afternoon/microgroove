/// Reference machine which passes sequence input through unmodified.
extern crate alloc;

use super::Machine;
use crate::{machine_resources::MachineResources, param::ParamList, Sequence};

#[derive(Debug)]
pub struct UnitMachine {
    params: ParamList,
}

impl UnitMachine {
    pub fn new() -> UnitMachine {
        UnitMachine {
            params: ParamList::new(),
        }
    }
}

impl Machine for UnitMachine {
    fn name(&self) -> &str {
        "UNIT"
    }

    fn params(&self) -> &ParamList {
        &self.params
    }

    fn params_mut(&mut self) -> &mut ParamList {
        &mut self.params
    }

    fn apply(&self, sequence: Sequence, _machine_resources: &mut MachineResources) -> Sequence {
        sequence
    }
}

unsafe impl Send for UnitMachine {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{machine_resources::MachineResources, sequence_generator::SequenceGenerator};

    #[test]
    fn unitmachine_should_passthrough_sequence_unmodified() {
        let mut machine_resources = MachineResources::new();
        let machine = UnitMachine::new();
        let input_sequence = SequenceGenerator::initial_sequence(8);
        let output_sequence = machine.apply(
            SequenceGenerator::initial_sequence(8),
            &mut machine_resources,
        );
        assert_eq!(output_sequence, input_sequence);
    }
}
