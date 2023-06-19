use crate::SEQUENCE_MAX_STEPS;

use core::fmt::{Display, Formatter, Result as FmtResult};
use heapless::Vec;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Part {
    #[default]
    Sequence,
    Call,
    Response,
    A,
    B,
    C,
    Hook,
    Turnaround,
}

impl Display for Part {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}",
            match *self {
                Part::Sequence => "SEQ",
                Part::Call => "CALL",
                Part::Response => "RESP",
                Part::A => "A_A_",
                Part::B => "_B__",
                Part::C => "___C",
                Part::Hook => "HOOK",
                Part::Turnaround => "TURN",
            }
        )
    }
}

impl TryFrom<u8> for Part {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Part::Sequence),
            1 => Ok(Part::Call),
            2 => Ok(Part::Response),
            3 => Ok(Part::A),
            4 => Ok(Part::B),
            5 => Ok(Part::C),
            6 => Ok(Part::Hook),
            7 => Ok(Part::Turnaround),
            _ => Err(()),
        }
    }
}

impl Part {
    pub fn new_mask(part: Part, mask_len: usize) -> Vec<bool, SEQUENCE_MAX_STEPS> {
        let infinite_trues = [true].iter().cycle();
        let infinite_falses = [false].iter().cycle();
        match part {
            Part::Sequence => infinite_trues.take(mask_len).cloned().collect(),
            Part::Call => {
                let prefix_len = mask_len / 2;
                let prefix_mask = infinite_trues.take(prefix_len);
                let suffix_len = mask_len - prefix_len;
                let suffix_mask = infinite_falses.take(suffix_len);
                prefix_mask.chain(suffix_mask).cloned().collect()
            }
            Part::Response => {
                let prefix_len = mask_len / 2;
                let prefix_mask = infinite_falses.take(prefix_len);
                let suffix_len = mask_len - prefix_len;
                let suffix_mask = infinite_trues.take(suffix_len);
                prefix_mask.chain(suffix_mask).cloned().collect()
            }
            Part::A => {
                // A_A_ = XXXX____XXXX____
                let section_len = mask_len / 4;
                let a_section = infinite_trues.take(section_len);
                let b_section = infinite_falses.take(section_len);
                let a2_section = [true].iter().cycle().take(section_len);
                let c_section_len = mask_len - (section_len * 3);
                let c_section = [false].iter().cycle().take(c_section_len);
                a_section
                    .chain(b_section)
                    .chain(a2_section)
                    .chain(c_section)
                    .cloned()
                    .collect()
            }
            Part::B => {
                // _B__ = ____XXXX________
                let section_len = mask_len / 4;
                let a_section = infinite_falses.take(section_len);
                let b_section = infinite_trues.take(section_len);
                let suffix_len = mask_len - section_len * 2;
                let suffix_mask = [false].iter().cycle().take(suffix_len);
                a_section
                    .chain(b_section)
                    .chain(suffix_mask)
                    .cloned()
                    .collect()
            }
            Part::C => {
                // ___C = ____________XXXX
                let prefix_len = mask_len / 4 * 3;
                let prefix_mask = infinite_falses.take(prefix_len);
                let suffix_len = mask_len - prefix_len;
                let suffix_mask = infinite_trues.take(suffix_len);
                prefix_mask.chain(suffix_mask).cloned().collect()
            }
            Part::Hook => {
                // Hook => XXXXXXXXXXXXXX__
                let prefix_len = mask_len / 8 * 7;
                let prefix_mask = infinite_trues.take(prefix_len);
                let suffix_len = mask_len - prefix_len;
                let suffix_mask = infinite_falses.take(suffix_len);
                prefix_mask.chain(suffix_mask).cloned().collect()
            }
            Part::Turnaround => {
                // Turnaround => ______________XX
                let prefix_len = mask_len / 8 * 7;
                let prefix_mask = infinite_falses.take(prefix_len);
                let suffix_len = mask_len - prefix_len;
                let suffix_mask = infinite_trues.take(suffix_len);
                prefix_mask.chain(suffix_mask).cloned().collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_mask_should_be_same_length_as_mask_len_parameter() {
        let mask = Part::new_mask(Part::Sequence, 27);
        assert_eq!(27, mask.len());
    }

    #[test]
    fn part_sequence_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            true, true,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::Sequence, 16);
        assert_eq!(expected, actual);
    }

    #[test]
    fn part_call_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            true, true, true, true, true, true, true, true, false, false, false, false, false,
            false, false, false,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::Call, 16);
        assert_eq!(expected, actual);
    }

    #[test]
    fn part_response_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            false, false, false, false, false, false, false, false, true, true, true, true, true,
            true, true, true,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::Response, 16);
        assert_eq!(expected, actual);
    }

    #[test]
    fn part_a_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            true, true, true, true, false, false, false, false, true, true, true, true, false,
            false, false, false,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::A, 16);
        assert_eq!(expected, actual);
    }

    #[test]
    fn part_b_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            false, false, false, false, true, true, true, true, false, false, false, false, false,
            false, false, false,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::B, 16);
        assert_eq!(expected, actual);
    }

    #[test]
    fn part_c_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            true, true, true, true,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::C, 16);
        assert_eq!(expected, actual);
    }

    #[test]
    fn part_hook_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            true, true, true, true, true, true, true, true, true, true, true, true, true, true,
            false, false,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::Hook, 16);
        assert_eq!(expected, actual);
    }

    #[test]
    fn part_turnaround_mask_should_correct_for_len_16() {
        let expected: Vec<bool, 32> = Vec::from_slice(&[
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, true, true,
        ])
        .unwrap();
        let actual = Part::new_mask(Part::Turnaround, 16);
        assert_eq!(expected, actual);
    }
}
