extern crate alloc;

use core::fmt::Debug;
use heapless::String;

use crate::{params::ParamList, Sequence};

pub trait Machine: Debug + Send {
    fn name(&self) -> &str;
    fn apply(&self, sequence: Sequence) -> Sequence;
    fn params(&self) -> &ParamList;
    fn params_mut(&mut self) -> &mut ParamList;
}

pub const GROOVE_MACHINE_IDS: &str = "UNIT";

pub const MELODY_MACHINE_IDS: &str = "UNIT";

pub fn machine_from_id(id: &str) -> Option<impl Machine> {
    let id_upcase = String::<6>::from(id).to_uppercase();
    match id_upcase.as_str() {
        "UNIT" => Some(unitmachine::UnitMachine::new()),
        _ => None,
    }
}

/// Reference machine which passes sequence input through unmodified.
pub mod unitmachine {
    extern crate alloc;

    use super::Machine;
    use crate::{
        params::{NumberParam, ParamList},
        Sequence,
    };
    use alloc::boxed::Box;

    #[derive(Clone, Copy, Debug)]
    struct UnitProcessor {}

    impl UnitProcessor {
        fn new() -> UnitProcessor {
            UnitProcessor {}
        }

        fn apply(&self, sequence: Sequence, _unused_argument: i8) -> Sequence {
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
            params
                .push(Box::new(NumberParam::new("NUM", 1, 16, 1)))
                .unwrap();
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

        fn apply(&self, sequence: Sequence) -> Sequence {
            let unused_argument = self.params[0].value_i8().unwrap();
            self.sequence_processor.apply(sequence, unused_argument)
        }

        fn params(&self) -> &ParamList {
            &self.params
        }

        fn params_mut(&mut self) -> &mut ParamList {
            &mut self.params
        }
    }

    unsafe impl Send for UnitMachine {}

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::initial_sequence;

        #[test]
        fn unitmachine_should_passthrough_sequence_unmodified() {
            let machine = UnitMachine::new();
            let input_sequence = initial_sequence(8);
            let output_sequence = machine.apply(initial_sequence(8));
            assert_eq!(output_sequence, input_sequence);
        }
    }
}
