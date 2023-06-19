use crate::{
    machine::unit_machine::UnitMachine,
    machine::Machine,
    machine_resources::MachineResources,
    param::{Param, ParamList, ParamValue},
    part::Part,
    quantizer::quantize,
    Sequence, Step, SEQUENCE_MAX_STEPS,
};

use alloc::boxed::Box;
use heapless::Vec;

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
            groove_params: ParamList::from_slice(&[Box::new(Param::new_part_param("PART"))])
                .unwrap(),
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

    pub fn part(&self) -> Part {
        self.groove_params[0].value().try_into().unwrap()
    }

    pub fn set_part(&mut self, part: Part) {
        self.groove_params[0].set(ParamValue::Part(part));
    }

    /// Generate a sequence by piping the initial sequence through the set of configured machines.
    pub fn generate(&self, length: u8, machine_resources: &mut MachineResources) -> Sequence {
        // a pipe operator would be nice to have here
        self.apply_part(
            self.apply_quantizer(
                self.melody_machine.apply(
                    self.rhythm_machine
                        .apply(Self::initial_sequence(length), machine_resources),
                    machine_resources,
                ),
            ),
        )
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

    fn apply_part(&self, sequence: Sequence) -> Sequence {
        let part = self.part();
        let step_mask = Part::new_mask(part, sequence.len());
        match part {
            Part::A => {
                let sequence = sequence.mask_steps(step_mask);
                let prefix_len = sequence.len() / 2;
                let suffix_len = sequence.len() - prefix_len;
                let steps_clone = sequence.steps.clone();
                let suffix = steps_clone.iter().take(suffix_len);
                let prefix = suffix.clone().take(prefix_len);
                sequence.set_steps(Vec::<Option<Step>, SEQUENCE_MAX_STEPS>::from_iter(
                    prefix.chain(suffix).cloned(),
                ))
            }
            _ => sequence.mask_steps(step_mask),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        machine::rand_melody_machine::RandMelodyMachine,
        midi::Note,
        param::ParamValue,
        quantizer::{Key, Scale},
    };

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
        generator.set_part(Part::Sequence);
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

    #[test]
    fn sequence_generator_with_part_equal_call_should_only_have_active_steps_in_first_half_of_sequence(
    ) {
        let mut generator = SequenceGenerator::default();
        generator.set_part(Part::Call);
        let mut machine_resources = MachineResources::new();
        let sequence = generator.generate(8, &mut machine_resources);
        let expected_active_steps = vec![true, true, true, true, false, false, false, false];
        let actual_active_steps = sequence
            .iter()
            .map(|s| s.is_some())
            .collect::<std::vec::Vec<bool>>();
        assert_eq!(expected_active_steps, actual_active_steps);
    }

    #[test]
    fn sequence_generator_with_part_equal_a_should_have_two_identical_halves() {
        let mut generator = SequenceGenerator::default();
        generator.set_part(Part::A);
        generator.rhythm_machine = Box::new(RandMelodyMachine::new());
        let mut machine_resources = MachineResources::new();
        let sequence = generator.generate(12, &mut machine_resources);
        let half1 = &sequence.steps[0..6];
        let half2 = &sequence.steps[6..12];
        assert_eq!(half1, half2);
    }

    #[test]
    fn sequence_generator_with_part_equal_a_and_odd_len_should_have_an_even_prefix_with_two_identical_halves(
    ) {
        let mut generator = SequenceGenerator::default();
        generator.set_part(Part::A);
        let mut machine_resources = MachineResources::new();
        let sequence = generator.generate(7, &mut machine_resources);
        let half1 = &sequence.steps[0..3];
        let half2 = &sequence.steps[3..6];
        assert_eq!(half1, half2);
    }
}
