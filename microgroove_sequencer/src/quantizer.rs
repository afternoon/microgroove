use crate::midi::Note;

use core::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Scale {
    #[default]
    Chromatic,
    Major,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    PentatonicMajor,
    PentatonicMinor,
    HexatonicBlues,
    WholeTone,
    MajorTriad,
    MinorTriad,
    DominantSeventh,
    DiminishedSeventh,
    Octave,
    OctaveAndFifth,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
}

impl Into<u8> for Scale {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Scale {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Scale::Chromatic),
            1 => Ok(Scale::Major),
            2 => Ok(Scale::NaturalMinor),
            3 => Ok(Scale::HarmonicMinor),
            4 => Ok(Scale::MelodicMinor),
            5 => Ok(Scale::PentatonicMajor),
            6 => Ok(Scale::PentatonicMinor),
            7 => Ok(Scale::HexatonicBlues),
            8 => Ok(Scale::WholeTone),
            9 => Ok(Scale::MajorTriad),
            10 => Ok(Scale::MinorTriad),
            11 => Ok(Scale::DominantSeventh),
            12 => Ok(Scale::DiminishedSeventh),
            13 => Ok(Scale::Octave),
            14 => Ok(Scale::OctaveAndFifth),
            15 => Ok(Scale::Dorian),
            16 => Ok(Scale::Phrygian),
            17 => Ok(Scale::Lydian),
            18 => Ok(Scale::Mixolydian),
            19 => Ok(Scale::Locrian),
            _ => Err(()),
        }
    }
}

impl Display for Scale {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}",
            match *self {
                Scale::Chromatic =>         "OFF",
                Scale::Major =>             "MAJ",
                Scale::NaturalMinor =>      "MIN",
                Scale::HarmonicMinor =>     "HMI",
                Scale::MelodicMinor =>      "MMI",
                Scale::PentatonicMajor =>   "PMA",
                Scale::PentatonicMinor =>   "PMI",
                Scale::HexatonicBlues =>    "BLU",
                Scale::WholeTone =>         "WHL",
                Scale::MajorTriad =>        "3MA",
                Scale::MinorTriad =>        "3MI",
                Scale::DominantSeventh =>   "7DO",
                Scale::DiminishedSeventh => "7DI",
                Scale::Octave =>            "OCT",
                Scale::OctaveAndFifth =>    "O+5",
                Scale::Dorian =>            "DOR",
                Scale::Phrygian =>          "PHR",
                Scale::Lydian =>            "LYD",
                Scale::Mixolydian =>        "MIX",
                Scale::Locrian =>           "LOC",
            }
        )
    }
}

/// Type to capture the mapping of notes in a chromatic octave to the quantized equivalent of
/// those notea in given scale. Each entry is an array of 12 values. The input note is used to
/// index into the array. The array value returned is the quantized note. This format allows for
/// some exciting scales (Reverse Phrygian anyone??).
type ScaleMap = [u8; 12];

impl From<Scale> for ScaleMap {
    #[rustfmt::skip]
    fn from(scale: Scale) -> Self {
        match scale {
            Scale::Chromatic =>         [0,  1,  2,  3,  4,  5,  6,  7,  8,  9,  10, 11],
            Scale::Major =>             [0,  2,  2,  4,  4,  5,  7,  7,  9,  9,  11, 11],
            Scale::NaturalMinor =>      [0,  2,  2,  3,  5,  5,  7,  7,  8,  10, 10, 12],
            Scale::HarmonicMinor =>     [0,  2,  2,  3,  5,  5,  7,  7,  8,  8,  11, 11],
            Scale::MelodicMinor =>      [0,  2,  2,  3,  5,  5,  7,  7,  9,  9,  11, 11],
            Scale::PentatonicMajor =>   [0,  2,  2,  4,  4,  4,  7,  7,  9,  9,  9,  12],
            Scale::PentatonicMinor =>   [0,  0,  3,  3,  5,  5,  7,  7,  7,  10, 10, 10],
            Scale::HexatonicBlues =>    [0,  0,  3,  3,  5,  5,  6,  7,  7,  10, 10, 10],
            Scale::WholeTone =>         [0,  0,  2,  2,  4,  4,  6,  6,  8,  8,  10, 10],
            Scale::MajorTriad =>        [0,  0,  0,  0,  4,  4,  4,  7,  7,  7,  7,  7 ],
            Scale::MinorTriad =>        [0,  0,  0,  3,  3,  3,  3,  7,  7,  7,  7,  7 ],
            Scale::DominantSeventh =>   [0,  0,  0,  0,  4,  4,  4,  7,  7,  7,  10, 10],
            Scale::DiminishedSeventh => [0,  0,  0,  3,  3,  3,  6,  6,  6,  9,  9,  9 ],
            Scale::Octave =>            [0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0 ],
            Scale::OctaveAndFifth =>    [0,  0,  0,  0,  0,  0,  7,  7,  7,  7,  7,  7 ],
            Scale::Dorian =>            [0,  2,  2,  3,  3,  5,  7,  7,  9,  9,  10, 10],
            Scale::Phrygian =>          [0,  1,  1,  3,  3,  5,  5,  7,  8,  8,  10, 10],
            Scale::Lydian =>            [0,  2,  2,  4,  4,  6,  6,  7,  9,  9,  11, 11],
            Scale::Mixolydian =>        [0,  2,  2,  4,  4,  5,  7,  7,  9,  9,  10, 10],
            Scale::Locrian =>           [0,  1,  1,  3,  3,  5,  6,  6,  8,  8,  10, 10],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Key {
    #[default]
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl Into<u8> for Key {
    fn into(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Key {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Key::C),
            1 => Ok(Key::CSharp),
            2 => Ok(Key::D),
            3 => Ok(Key::DSharp),
            4 => Ok(Key::E),
            5 => Ok(Key::F),
            6 => Ok(Key::FSharp),
            7 => Ok(Key::G),
            8 => Ok(Key::GSharp),
            9 => Ok(Key::A),
            10 => Ok(Key::ASharp),
            11 => Ok(Key::B),
            _ => Err(()),
        }
    }
}

impl Display for Key {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}",
            match *self {
                Key::C => "C",
                Key::CSharp => "C#",
                Key::D => "D",
                Key::DSharp => "D#",
                Key::E => "E",
                Key::F => "F",
                Key::FSharp => "F#",
                Key::G => "G",
                Key::GSharp => "G#",
                Key::A => "A",
                Key::ASharp => "A#",
                Key::B => "B",
            }
        )
    }
}

pub fn quantize(note: Note, scale: Scale, key: Key) -> Note {
    let key_num: u8 = key.into();
    let offset = 12 - key_num;
    let note_num: u8 = note.into();
    let note_num_offset = note_num + offset;
    let octave = note_num_offset / 12;
    let degree = note_num_offset % 12;
    let interval_map: ScaleMap = scale.into();
    let quantized_degree = interval_map[degree as usize] as u8;
    let quantized_note_num = ((quantized_degree + octave * 12) - offset) as u8;
    quantized_note_num.min(127).try_into().expect("note number should be valid note")
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
        let quantized_notes = quantize_octave(input_notes(), Scale::Major, Key::C);
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
        let quantized_notes = quantize_octave(input_notes(), Scale::NaturalMinor, Key::C);
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
        let quantized_notes = quantize_octave(input_notes(), Scale::NaturalMinor, Key::GSharp);
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

    fn quantize_octave(input_notes: [Note; 12], scale: Scale, key: Key) -> [Note; 12] {
        input_notes
            .iter()
            .map(|&note| quantize(note, scale, key))
            .collect::<Vec<Note>>()
            .try_into()
            .unwrap()
    }
}
