use crate::{
    machine::unit_machine::UnitMachine,
    machine::Machine,
    machine_resources::MachineResources,
    param::{Param, ParamList},
    quantizer::quantize,
    Sequence, Step,
};

use alloc::boxed::Box;

#[derive(Debug)]
pub struct SequenceGenerator {
    pub rhythm_machine: Box<dyn Machine>,
    pub melody_machine: Box<dyn Machine>,
    groove_params: ParamList,
    harmony_params: ParamList,
}

impl Default for SequenceGenerator {
    fn default() -> SequenceGenerator {
        SequenceGenerator {
            rhythm_machine: Box::new(UnitMachine::new()),
            melody_machine: Box::new(UnitMachine::new()),
            groove_params: ParamList::new(),
            harmony_params: ParamList::from_slice(&[
                Box::new(Param::new_scale_param("SCALE")),
                Box::new(Param::new_key_param("KEY")),
            ])
            .unwrap(),
        }
    }
}

impl SequenceGenerator {
    pub fn initial_sequence(length: u8) -> Sequence {
        (0..length).map(|_i| Step::new(60).ok()).collect()
    }

    /// Generate a sequence by piping the initial sequence through the set of configured machines.
    pub fn generate(&self, length: u8, machine_resources: &mut MachineResources) -> Sequence {
        self.apply_quantizer(
            self.melody_machine.apply(
                self.rhythm_machine
                    .apply(Self::initial_sequence(length), machine_resources),
                machine_resources,
            ),
        )
    }

    pub fn groove_params(&self) -> &ParamList {
        &self.groove_params
    }

    pub fn groove_params_mut(&mut self) -> &mut ParamList {
        &mut self.groove_params
    }

    pub fn harmony_params(&self) -> &ParamList {
        &self.harmony_params
    }

    pub fn harmony_params_mut(&mut self) -> &mut ParamList {
        &mut self.harmony_params
    }

    fn apply_quantizer(&self, sequence: Sequence) -> Sequence {
        let scale = self.harmony_params[0]
            .value()
            .try_into()
            .expect("unexpected scale value for quantizer");
        let key = self.harmony_params[1]
            .value()
            .try_into()
            .expect("unexpected key value for quantizer");
        sequence.map_notes(|note| quantize(note.into(), scale, key).into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        midi::Note,
        param::ParamValue,
        quantizer::{Key, Scale},
    };

    use super::*;

    #[test]
    fn sequence_generator_default_should_create_a_new_generator() {
        let generator = SequenceGenerator::default();
        assert_eq!("UNIT", generator.rhythm_machine.name());
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

    #[test]
    fn sequence_generator_should_quantize_melodies_if_configured_to_do_so() {
        let mut generator = SequenceGenerator::default();
        let params = generator.harmony_params_mut();
        params[0].set(ParamValue::Scale(Scale::Major));
        params[1].set(ParamValue::Key(Key::B));
        let mut machine_resources = MachineResources::new();
        let sequence = generator.generate(8, &mut machine_resources);
        assert!(sequence.steps[0].is_some());
        let step0 = sequence.steps[0].as_ref().unwrap();
        let step0_note_num: u8 = step0.note.into();
        let expected: u8 = Note::CSharp3.into();
        assert_eq!(expected, step0_note_num); // exp
    }
}
