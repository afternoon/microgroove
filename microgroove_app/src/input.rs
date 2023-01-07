use crate::encoder::encoder_array::ENCODER_COUNT;
use microgroove_sequencer::{
    machine::{MelodyMachineId, RhythmMachineId},
    machine_resources::MachineResources,
    param::{wrapping_add, ParamValue, ParamList, ParamError},
    sequencer::Sequencer,
    sequence_generator::SequenceGenerator,
    Track, TRACK_COUNT,
};

use core::iter::zip;
use defmt::{debug, error, Format};
use heapless::Vec;

type EncoderValues = Vec<Option<i8>, ENCODER_COUNT>;

const TRACK_NUM_PARAM_INDEX: usize = 2;

#[derive(Clone, Copy, Debug, Default, Format)]
pub enum InputMode {
    #[default]
    Track,
    Sequence,
    Rhythm,
    Groove,
    Melody,
    Harmony,
}

/// Iterate over `encoder_values` and pass to a destination set of `Param`s
/// determined by `InputMode`. This may have side-effects, including that sequence data may need to be
/// regenerated.
pub fn apply_encoder_values(
    encoder_values: EncoderValues,
    input_mode: InputMode,
    current_track: &mut u8,
    sequencer: &mut Sequencer,
    sequence_generators: &mut Vec<SequenceGenerator, TRACK_COUNT>,
    machine_resources: &mut MachineResources,
) -> Result<(), ParamError> {
    if track_num_has_changed(input_mode, &encoder_values) {
        update_current_track(&encoder_values, current_track);
        return Ok(());
    }
    if track_disabled(sequencer, current_track) {
        enable_track(sequencer, current_track);
        return Ok(());
    }
    let generator = sequence_generators.get_mut(*current_track as usize).unwrap();
    match input_mode {
        InputMode::Track => {
            let track = sequencer.tracks.get_mut(*current_track as usize).unwrap().as_mut().unwrap();
            let params = track.params_mut();
            update_params(&encoder_values, params)?;
            if rhythm_machine_changed(input_mode, &encoder_values) {
                update_rhythm_machine(generator, params[0].value())
            }
            if melody_machine_changed(input_mode, &encoder_values) {
                update_melody_machine(generator, params[3].value())
            }
            track.apply_params()?;
        }
        InputMode::Sequence => {
            update_params(&encoder_values, sequencer.params_mut())?;
        }
        InputMode::Rhythm => {
            update_params(&encoder_values, generator.rhythm_machine.params_mut())?;
        }
        InputMode::Groove => {
            update_params(&encoder_values, generator.groove_params_mut())?;
        }
        InputMode::Melody => {
            update_params(&encoder_values, generator.melody_machine.params_mut())?;
        }
        InputMode::Harmony => {
            update_params(&encoder_values, generator.harmony_params_mut())?;
        }
    }
    update_sequence(sequencer, current_track, generator, machine_resources);
    Ok(())
}

fn update_current_track(encoder_values: &EncoderValues, current_track: &mut u8) {
    if let Some(track_num_increment) = encoder_values[TRACK_NUM_PARAM_INDEX] {
        let new_track_num = wrapping_add(
            *current_track as i32,
            track_num_increment as i32,
            TRACK_COUNT as i32 - 1,
        ) as u8;
        debug!("[map_encoder_input] current_track={}", new_track_num);
        *current_track = new_track_num;
    }
}

fn track_num_has_changed(input_mode: InputMode, encoder_values: &EncoderValues) -> bool {
    match input_mode {
        InputMode::Track => match encoder_values.as_slice() {
            [_, _, Some(_), _, _, _] => true,
            _ => false,
        },
        _ => false,
    }
}

fn rhythm_machine_changed(input_mode: InputMode, encoder_values: &EncoderValues) -> bool {
    match input_mode {
        InputMode::Track => match encoder_values.as_slice() {
            [Some(_), _, _, _, _, _] => true,
            _ => false,
        },
        _ => false,
    }
}

fn melody_machine_changed(input_mode: InputMode, encoder_values: &EncoderValues) -> bool {
    match input_mode {
        InputMode::Track => match encoder_values.as_slice() {
            [_, _, _, Some(_), _, _] => true,
            _ => false,
        },
        _ => false,
    }
}

fn track_disabled(sequencer: &Sequencer, track_num: &u8) -> bool {
    sequencer.tracks.get(*track_num as usize).unwrap().is_none()
}

fn enable_track(sequencer: &mut Sequencer, track_num: &u8) {
    let mut new_track = Track::default();
    new_track.midi_channel = (*track_num).into();
    new_track.sequence = SequenceGenerator::initial_sequence(new_track.length);
    let _ = sequencer.enable_track(*track_num, new_track);
}

fn update_params(encoder_values: &EncoderValues, params: &mut ParamList) -> Result<(), ParamError> {
    let params_and_values = zip(params.iter_mut(), encoder_values);
    for (param, &value) in params_and_values {
        if let Some(value) = value {
            debug!(
                "[map_encoder_input] increment param={}, value={}",
                param.name(),
                value
            );
            param.increment(value.into())?;
        }
    }
    Ok(())
}

fn update_rhythm_machine(generator: &mut SequenceGenerator, param_value: ParamValue) {
    let id: RhythmMachineId = param_value
        .try_into()
        .expect("unexpected rhythm machine param");
    generator.rhythm_machine = id.into();
}

fn update_melody_machine(generator: &mut SequenceGenerator, param_value: ParamValue) {
    let id: MelodyMachineId = param_value
        .try_into()
        .expect("unexpected melody machine param");
    generator.melody_machine = id.into();
}

fn update_sequence(sequencer: &mut Sequencer, track_num: &u8, generator: &SequenceGenerator, machine_resources: &mut MachineResources) {
    debug!("[update_sequence] track_num={}", track_num);
    match sequencer.tracks.get_mut(*track_num as usize) {
        Some(mut_track) => match mut_track.as_mut() {
            Some(track) => {
                track.sequence = generator.generate(track.length, machine_resources);
            }
            None => {
                error!("[update_sequence] tried to update sequence for disabled track");
            }
        },
        None => {
            error!("[update_sequence] couldn't get track from sequencer, track_num={}", track_num);
        }
    }
}
