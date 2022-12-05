/// Rendering UI graphics to the display.
use core::{fmt::Write, iter::zip, str::FromStr};
use display_interface::DisplayError;
use embedded_graphics::{
    mono_font::{
        ascii::{FONT_4X6, FONT_6X10},
        MonoTextStyle,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Baseline, Text, TextStyle, TextStyleBuilder},
};

use crate::{input::InputMode, peripherals::Display};
use heapless::String;
use microgroove_sequencer::{params::ParamList, Track};

type DisplayResult = Result<(), DisplayError>;

const DISPLAY_WIDTH: i32 = 128;
const DISPLAY_HEIGHT: i32 = 64;
const DISPLAY_CENTER: i32 = DISPLAY_WIDTH / 2;

const CHAR_HEIGHT: u32 = 7;

const WARNING_Y_POS: i32 = 21;
const WARNING_PADDING: i32 = 4;
const WARNING_BORDER: u32 = 1;

const HEADER_WIDTH: u32 = DISPLAY_WIDTH as u32;
const HEADER_HEIGHT: u32 = 6;
const HEADER_PLAYING_ICON_X_POS: i32 = 24;

const SEQUENCE_X_POS: i32 = 0;
const SEQUENCE_Y_POS: i32 = HEADER_HEIGHT as i32 + 1;
const SEQUENCE_WIDTH: u32 = DISPLAY_WIDTH as u32;
const SEQUENCE_HEIGHT: u32 = 45;
const SEQUENCE_UNDERLINE_Y_POS: i32 = 45;

const PARAM_Y_POS: u32 = 51;

// TODO move to another crate where it can be tested
fn map_to_range(x: i32, in_min: i32, in_max: i32, out_min: i32, out_max: i32) -> i32 {
    (x - in_min) * (out_max - out_min + 1) / (in_max - in_min + 1) + out_min
}

/// Show snazzy splash screen.
pub fn render_splash_screen_view(display: &mut Display) -> DisplayResult {
    display.clear();
    Text::with_text_style(
        "MICROGROOVE",
        Point::new(DISPLAY_CENTER, WARNING_Y_POS),
        big_character_style(),
        centered(),
    )
    .draw(display)?;
    Text::with_baseline(
        "I wanna go bang",
        Point::new(37, 42),
        default_character_style(),
        Baseline::Top,
    )
    .draw(display)?;
    display.flush()?;
    Ok(())
}

pub fn render_perform_view(
    display: &mut Display,
    input_mode: InputMode,
    playing: bool,
    track: &Option<Track>,
    track_num: usize,
    active_step_num: Option<u32>,
) -> DisplayResult {
    draw_header(display, input_mode, playing, track_num)?;
    if let Some(track) = track {
        draw_sequence(display, track, active_step_num.unwrap())?;
        draw_params(display, input_mode, track, track_num)?;
    } else {
        draw_disabled_track_warning(display)?;
    }
    display.flush()?;
    Ok(())
}

fn draw_header(
    display: &mut Display,
    input_mode: InputMode,
    playing: bool,
    track_num: usize,
) -> DisplayResult {
    Rectangle::new(Point::zero(), Size::new(HEADER_WIDTH, HEADER_HEIGHT))
        .into_styled(background_style())
        .draw(display)?;
    let mut track_num_str: String<5> = String::from_str("TRK").unwrap();
    write!(track_num_str, "{:02}", track_num).unwrap();
    Text::with_baseline(
        track_num_str.as_str(),
        Point::zero(),
        default_character_style(),
        Baseline::Top,
    )
    .draw(display)?;
    if playing {
        Text::with_baseline(
            ">",
            Point::new(HEADER_PLAYING_ICON_X_POS, 0),
            default_character_style(),
            Baseline::Top,
        )
        .draw(display)?;
    }
    let title = match input_mode {
        InputMode::Track => "TRACK",
        InputMode::Groove => "GROOVE",
        InputMode::Melody => "MELODY",
    };
    Text::with_text_style(
        title,
        Point::new(DISPLAY_CENTER, 0),
        default_character_style(),
        centered(),
    )
    .draw(display)?;
    match input_mode {
        InputMode::Track => { /* don't do nuffink */ }
        InputMode::Groove | InputMode::Melody => {
            let machine_name = "MACHINE_NAME"; // TODO show actual machine name
            Text::with_text_style(
                machine_name,
                Point::new(DISPLAY_WIDTH, 0),
                default_character_style(),
                right_align(),
            )
            .draw(display)?;
        }
    }
    Ok(())
}

fn draw_disabled_track_warning(display: &mut Display) -> DisplayResult {
    Rectangle::new(
        Point::new(SEQUENCE_X_POS, SEQUENCE_Y_POS),
        Size::new(
            SEQUENCE_WIDTH,
            SEQUENCE_HEIGHT + (DISPLAY_HEIGHT as u32 - PARAM_Y_POS),
        ),
    )
    .into_styled(background_style())
    .draw(display)?;
    warning(display, "TRACK DISABLED")
}

fn draw_sequence(display: &mut Display, track: &Track, active_step_num: u32) -> DisplayResult {
    let step_width: u32 = if track.length <= 17 { 6 } else { 3 };
    let step_height: u32 = step_width;
    let display_sequence_margin_left =
        (DISPLAY_WIDTH - ((track.length as i32) * ((step_width as i32) + 1))) / 2;
    let (note_min, note_max) = note_min_max_as_u8s(track);
    let note_y_pos_min: u32 = 35;
    let note_y_pos_max: u32 = 9 + step_height as u32;
    let step_size = Size::new(step_width, step_height);
    let mut step_num: u32 = 0;

    // erase sequence region of display
    Rectangle::new(
        Point::new(SEQUENCE_X_POS, SEQUENCE_Y_POS),
        Size::new(SEQUENCE_WIDTH, SEQUENCE_HEIGHT),
    )
    .into_styled(background_style())
    .draw(display)?;

    for step in &track.sequence {
        if let Some(step) = step {
            let x = display_sequence_margin_left + (step_num as i32 * (step_width as i32 + 1));
            let x2 = x + step_width as i32;
            let note_num: u8 = step.note.into();
            let y = map_to_range(
                note_num as i32,
                note_min as i32,
                note_max as i32,
                note_y_pos_min as i32,
                note_y_pos_max as i32,
            );

            // draw step
            let step_style = if step_num == active_step_num {
                outline_style()
            } else {
                filled_style()
            };
            Rectangle::new(Point::new(x as i32, y as i32), step_size)
                .into_styled(step_style)
                .draw(display)?;

            // draw step underline
            Line::new(
                Point::new(x, SEQUENCE_UNDERLINE_Y_POS),
                Point::new(x2 - 1, SEQUENCE_UNDERLINE_Y_POS),
            )
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(display)?;
        }
        step_num += 1;
    }

    Ok(())
}

fn note_min_max_as_u8s(track: &Track) -> (u8, u8) {
    let note_min: u8 = track
        .sequence
        .iter()
        .min()
        .unwrap()
        .as_ref()
        .unwrap()
        .note
        .into();
    let note_max: u8 = track
        .sequence
        .iter()
        .max()
        .unwrap()
        .as_ref()
        .unwrap()
        .note
        .into();
    (note_min, note_max)
}

fn draw_params(
    display: &mut Display,
    input_mode: InputMode,
    track: &Track,
    track_num: usize,
) -> DisplayResult {
    let params = match input_mode {
        InputMode::Track => track.params(),
        InputMode::Groove => track.groove_machine.params(),
        InputMode::Melody => track.melody_machine.params(),
    };
    draw_param_table(display, input_mode, params, track_num)
}

fn draw_param_table(
    display: &mut Display,
    input_mode: InputMode,
    params: &ParamList,
    track_num: usize,
) -> DisplayResult {
    let is_track = match input_mode {
        InputMode::Track => true,
        _ => false,
    };

    let col_content_width = 40;
    let col_padding = 8;
    let col_width = col_content_width + col_padding;

    let name0_x: i32 = 0;
    let name1_x: i32 = if is_track { 60 } else { col_width };
    let name2_x: i32 = if is_track { 96 } else { col_width * 2 };

    let value0_x: i32 = if is_track {
        51
    } else {
        name0_x + col_content_width
    };
    let value1_x: i32 = if is_track {
        88
    } else {
        name1_x + col_content_width
    };
    let value2_x: i32 = DISPLAY_WIDTH;

    let row0_y = PARAM_Y_POS as i32;
    let row1_y = (PARAM_Y_POS + CHAR_HEIGHT) as i32;

    let param_name_points = [
        Point::new(name0_x, row0_y),
        Point::new(name1_x, row0_y),
        Point::new(name2_x, row0_y),
        Point::new(name0_x, row1_y),
        Point::new(name1_x, row1_y),
        Point::new(name2_x, row1_y),
    ];
    let param_value_points = [
        Point::new(value0_x, row0_y),
        Point::new(value1_x, row0_y),
        Point::new(value2_x, row0_y),
        Point::new(value0_x, row1_y),
        Point::new(value1_x, row1_y),
        Point::new(value2_x, row1_y),
    ];
    let params = zip(params, zip(param_name_points, param_value_points));

    Rectangle::new(
        Point::new(0, PARAM_Y_POS as i32),
        Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32 - PARAM_Y_POS),
    )
    .into_styled(background_style())
    .draw(display)?;

    for (param, (name_point, value_point)) in params {
        Text::with_baseline(
            param.name(),
            name_point,
            default_character_style(),
            Baseline::Top,
        )
        .draw(display)?;
        Text::with_text_style(
            param.value_str().as_str(),
            value_point,
            default_character_style(),
            right_align(),
        )
        .draw(display)?;
    }

    // HACK HACK HACK
    // track num isn't actually stored in a param, so here we just write the real track num over
    // the top of whatever junk value came from the param.
    if is_track {
        let mut track_num_str: String<5> = String::new();
        write!(track_num_str, "{}", track_num).unwrap();
        Rectangle::new(Point::new(116, row0_y), Size::new(13, 6))
            .into_styled(background_style())
            .draw(display)?;
        Text::with_text_style(
            track_num_str.as_str(),
            Point::new(DISPLAY_WIDTH, row0_y),
            default_character_style(),
            right_align(),
        )
        .draw(display)?;
    }

    Ok(())
}

fn default_character_style<'a>() -> MonoTextStyle<'a, BinaryColor> {
    MonoTextStyle::new(&FONT_4X6, BinaryColor::On)
}

fn big_character_style<'a>() -> MonoTextStyle<'a, BinaryColor> {
    MonoTextStyle::new(&FONT_6X10, BinaryColor::On)
}

fn background_style() -> PrimitiveStyle<BinaryColor> {
    PrimitiveStyle::with_fill(BinaryColor::Off)
}

fn filled_style() -> PrimitiveStyle<BinaryColor> {
    PrimitiveStyle::with_fill(BinaryColor::On)
}

fn outline_style() -> PrimitiveStyle<BinaryColor> {
    PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(1)
        .fill_color(BinaryColor::Off)
        .build()
}

fn warning_style() -> PrimitiveStyle<BinaryColor> {
    PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(WARNING_BORDER)
        .fill_color(BinaryColor::Off)
        .build()
}

fn centered() -> TextStyle {
    TextStyleBuilder::new()
        .alignment(Alignment::Center)
        .baseline(Baseline::Top)
        .build()
}

fn right_align() -> TextStyle {
    TextStyleBuilder::new()
        .alignment(Alignment::Right)
        .baseline(Baseline::Top)
        .build()
}

// TODO rendering of warnings is broken - border outside width of display, padding unevenn
fn warning(display: &mut Display, text: &str) -> DisplayResult {
    let char_width = 6;
    let char_height = 10;
    let space_width = 1;
    let text_width = ((text.len() * char_width)
        + ((text.len() - 1) * space_width)
        + (WARNING_PADDING as usize * 2)) as i32;
    let text_margin_left = (DISPLAY_WIDTH - text_width) / 2;
    let warning_width = DISPLAY_WIDTH - (text_margin_left * 2);
    let warning_height = char_height + WARNING_PADDING * 2 + WARNING_BORDER as i32 * 2;
    let warning_text_y_pos = WARNING_Y_POS + WARNING_PADDING + WARNING_BORDER as i32;
    Rectangle::new(
        Point::new(text_margin_left, WARNING_Y_POS),
        Size::new(warning_width as u32, warning_height as u32),
    )
    .into_styled(warning_style())
    .draw(display)?;
    Text::with_text_style(
        text,
        Point::new(DISPLAY_CENTER, warning_text_y_pos),
        big_character_style(),
        centered(),
    )
    .draw(display)?;
    Ok(())
}
