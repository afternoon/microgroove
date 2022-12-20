use crate::midi::Note;

#[derive(Clone, Copy)]
pub enum Scale {
    Chromatic,
    Major,
    Minor,
    // Dorian,
    // Phrygian,
    // Lydian,
    // Mixolydian,
    // Locrian,
    // MajorPentatonic,
    // MinorPentatonic,
    // MajorBlues,
    // MinorBlues,
    // MajorTriad,
    // MinorTriad,
    Octave,
    OctaveAndFifth,
}

/// Type to capture the mapping of notes in a chromatic octave to the quantized equivalent of
/// those notea in given scale. Each entry is an array of 12 values. The input note is used to
/// index into the array. The array value returned is the quantized note. This format allows for
/// some exciting scales (Reverse Phrygian anyone??).
type IntervalMap = [u8; 12];

impl From<Scale> for IntervalMap {
    fn from(scale: Scale) -> Self {
        match scale {
            Scale::Chromatic => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Scale::Major => [0, 2, 2, 4, 4, 5, 7, 7, 9, 9, 11, 11],
            Scale::Minor => [0, 2, 2, 3, 5, 5, 7, 7, 8, 10, 10, 12],
            // Scale::Dorian => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::Phrygian => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::Lydian => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::Mixolydian => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::Locrian => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::MajorPentatonic => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::MinorPentatonic => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::MajorBlues => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::MinorBlues => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::MajorTriad => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            // Scale::MinorTriad => [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Scale::Octave => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            Scale::OctaveAndFifth => [0, 0, 0, 0, 0, 0, 7, 7, 7, 7, 7, 7],
        }
    }
}

pub fn quantize(note: Note, scale: Scale, root_note: Note) -> Note {
    let root_note_num: u8 = root_note.into();
    let root_note_degree = root_note_num % 12;
    let offset = 12 - root_note_degree;
    let note_num: u8 = note.into();
    let note_num_offset = note_num + offset;
    let octave = note_num_offset / 12;
    let degree = note_num_offset % 12;
    let interval_map: IntervalMap = scale.into();
    let quantized_degree = interval_map[degree as usize] as u8;
    let quantized_note_num = ((quantized_degree + octave * 12) - offset) as u8;
    quantized_note_num.min(127).try_into().unwrap()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn quantize_should_quantize_c_major() {
        let expected_notes = [
            Note::C3,
            Note::D3,
            Note::D3,
            Note::E3,
            Note::E3,
            Note::F3,
            Note::G3,
            Note::G3,
            Note::A3,
            Note::A3,
            Note::B3,
            Note::B3,
        ];
        let quantized_notes = quantize_octave(input_notes(), Scale::Major, Note::C0);
        assert_eq!(expected_notes, quantized_notes);
    }

    #[test]
    fn quantize_should_quantize_c_minor() {
        let expected_notes = [
            Note::C3,
            Note::D3,
            Note::D3,
            Note::DSharp3,
            Note::F3,
            Note::F3,
            Note::G3,
            Note::G3,
            Note::GSharp3,
            Note::ASharp3,
            Note::ASharp3,
            Note::C4,
        ];
        let quantized_notes = quantize_octave(input_notes(), Scale::Minor, Note::C0);
        assert_eq!(expected_notes, quantized_notes);
    }

    #[test]
    fn quantize_should_quantize_g_sharp_minor() {
        let expected_notes = [
            Note::CSharp3,
            Note::CSharp3,
            Note::DSharp3,
            Note::DSharp3,
            Note::E3,
            Note::FSharp3,
            Note::FSharp3,
            Note::GSharp3,
            Note::GSharp3,
            Note::ASharp3,
            Note::ASharp3,
            Note::B3,
        ];
        let quantized_notes = quantize_octave(input_notes(), Scale::Minor, Note::GSharp0);
        assert_eq!(expected_notes, quantized_notes);
    }

    fn input_notes() -> [Note; 12] {
        [
            Note::C3,
            Note::CSharp3,
            Note::D3,
            Note::DSharp3,
            Note::E3,
            Note::F3,
            Note::FSharp3,
            Note::G3,
            Note::GSharp3,
            Note::A3,
            Note::ASharp3,
            Note::B3,
        ]
    }

    fn quantize_octave(input_notes: [Note; 12], scale: Scale, root: Note) -> [Note; 12] {
        input_notes
            .iter()
            .map(|&note| quantize(note, scale, root))
            .collect::<Vec<Note>>()
            .try_into()
            .unwrap()
    }
}
