/// Reference machine which passes sequence input through unmodified.
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

    fn generate(&mut self, _machine_resources: &mut MachineResources) {}

    fn apply(&self, sequence: Sequence) -> Sequence {
        sequence
    }
}

unsafe impl Send for UnitMachine {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence_generator::SequenceGenerator;

    #[test]
    fn unitmachine_should_passthrough_sequence_unmodified() {
        let machine = UnitMachine::new();
        let input_sequence = SequenceGenerator::initial_sequence(8);
        let output_sequence = machine.apply(SequenceGenerator::initial_sequence(8));
        assert_eq!(output_sequence, input_sequence);
    }
}
