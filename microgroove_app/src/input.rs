use crate::encoder::encoder_array::ENCODER_COUNT;
use microgroove_sequencer::{params::wrapping_add, sequencer::Sequencer, Track, TRACK_COUNT};

use core::iter::zip;
use defmt::{debug, trace, Format};
use heapless::Vec;

type EncoderValues = Vec<Option<i8>, ENCODER_COUNT>;

const TRACK_NUM_PARAM_INDEX: usize = 2;

#[derive(Clone, Copy, Debug, Default, Format)]
pub enum InputMode {
    #[default]
    Track,
    Groove,
    Melody,
}

/// Iterate over `encoder_values` and pass to either `Track`, groove `Machine` or
/// melody `Machine`, determined by `input_mode`.
pub fn map_encoder_input(
    input_mode: InputMode,
    sequencer: &mut Sequencer,
    encoder_values: EncoderValues,
) {
    // set the current track in the sequencer if track mode && track param has changed
    trace!("[map_encoder_input] input_mode={}", input_mode);
    if let InputMode::Track = input_mode {
        if let Some(track_num_increment) = encoder_values[TRACK_NUM_PARAM_INDEX] {
            let new_track_num = wrapping_add(
                sequencer.current_track_num() as i32 - 1,
                track_num_increment as i32,
                TRACK_COUNT as i32 - 1,
            );
            debug!("[map_encoder_input] new_track_num={}", new_track_num);
            sequencer.set_current_track(new_track_num as u8);
        }
    }

    // make sure we have the latest track num
    let track_num = sequencer.current_track_num() as u8;

    // The current track might be disabled (None in the sequencer's `Vec` of tracks). If the user
    // browses through tracks using the track num encoder on the track page, then we do nothing
    // more here. Any other encoder input triggers the creation of a new track in the current slot.
    let maybe_track = sequencer.current_track_mut();
    if let None = maybe_track {
        if only_track_num_has_changed(input_mode, &encoder_values) {
            return;
        }
        let new_track = Track {
            midi_channel: (track_num - 1).into(),
            ..Default::default()
        };
        let _ = maybe_track.insert(new_track);
    }

    // get &mut to the relevant set of params
    let track = maybe_track.as_mut().unwrap();
    let params_mut = match input_mode {
        InputMode::Track => track.params_mut(),
        InputMode::Groove => track.groove_machine.params_mut(),
        InputMode::Melody => track.melody_machine.params_mut(),
    };

    // update params
    let params_and_values = zip(params_mut, encoder_values);
    for (param, value) in params_and_values {
        if let Some(value) = value {
            debug!(
                "[map_encoder_input] increment param={}, value={}",
                param.name(),
                value
            );
            param.increment(value);
        }
    }

    // write param data back to track member variables
    if let InputMode::Track = input_mode {
        track.apply_params();
    }
}

fn only_track_num_has_changed(input_mode: InputMode, encoder_values: &EncoderValues) -> bool {
    match input_mode {
        InputMode::Track => match encoder_values.as_slice() {
            [None, None, Some(_), None, None, None] => true,
            _ => false,
        },
        _ => false,
    }
}
