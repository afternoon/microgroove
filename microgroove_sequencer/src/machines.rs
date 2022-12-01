pub mod unitmachine {
    extern crate alloc;

    use crate::{
        Machine, Sequence, SequenceProcessor,
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
