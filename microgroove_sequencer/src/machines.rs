extern crate alloc;

use alloc::boxed::Box;
use core::fmt::Debug;
use heapless::String;

use crate::{params::ParamList, SequenceProcessor};

pub trait Machine: Debug + Send {
    fn name(&self) -> &str;
    fn sequence_processor(&self) -> Box<dyn SequenceProcessor>;
    fn params(&self) -> &ParamList;
    fn params_mut(&mut self) -> &mut ParamList;
}

pub const GROOVE_MACHINE_IDS: &str = "UNIT";
pub const MELODY_MACHINE_IDS: &str = "UNIT";

pub fn machine_from_id(id: &str) -> Option<impl Machine> {
    let mut id_upcase = String::<6>::from(id);
    id_upcase.make_ascii_uppercase();
    match id_upcase.as_str() {
        "UNIT" => Some(unitmachine::UnitMachine::new()),
        _ => None,
    }
}

pub mod unitmachine {
    extern crate alloc;

    use super::Machine;
    use crate::{
        params::{NumberParam, ParamList},
        Sequence, SequenceProcessor,
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
