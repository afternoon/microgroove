/// Handle user input (encoder turns, button presses).
use microgroove_sequencer::sequencer::{self, Sequencer};
use crate::encoder::encoder_array::ENCODER_COUNT;
use core::iter::zip;
use defmt::debug;
use heapless::Vec;

#[derive(Clone, Copy, Debug)]
pub enum InputMode {
    Track,
    Groove,
    Melody,
}

/// Iterate over `encoder_values` and pass to either `Track`, groove `Machine` or
/// melody `Machine`, determined by `input_mode`.
pub fn map_encoder_input(
    input_mode: InputMode,
    sequencer: &mut Sequencer,
    encoder_values: Vec<i8, ENCODER_COUNT>,
) {
    let opt_track = sequencer.current_track_mut();
    opt_track.get_or_insert_with(|| sequencer::new_track_with_default_machines());
    let track = opt_track.as_mut().unwrap();
    let params_mut = match input_mode {
        InputMode::Track => track.params_mut(),
        InputMode::Groove => track.groove_machine.params_mut(),
        InputMode::Melody => track.melody_machine.params_mut(),
    };

    // update params
    let params_and_values = zip(params_mut, encoder_values);
    for (param, value) in params_and_values {
        debug!("increment param: {}, value: {}", param.name(), value);
        param.increment(value);
    }

    // write param data back to track member variables and set the current track in the
    // sequencer
    if let InputMode::Track = input_mode {
        let track_num = (track.params()[2].value_i8().unwrap() - 1) as u8;
        track.apply_params();
        sequencer.set_current_track(track_num);
    }
}
