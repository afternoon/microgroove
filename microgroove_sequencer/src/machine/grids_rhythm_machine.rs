/// Machine which generates rhythms using patterns from Mutable Instruments Grids.
use super::Machine;
use crate::{
    machine_resources::MachineResources,
    param::{Param, ParamList},
    Sequence,
};

use alloc::boxed::Box;
use core::fmt::{Display, Formatter, Result as FmtResult};

#[rustfmt::skip]
const GRIDS_PATTERN_LUT_0: [u8; 96] = [
     255,      0,      0,      0,      0,      0,    145,      0,
       0,      0,      0,      0,    218,      0,      0,      0,
      72,      0,     36,      0,    182,      0,      0,      0,
     109,      0,      0,      0,     72,      0,      0,      0,
      36,      0,    109,      0,      0,      0,      8,      0,
     255,      0,      0,      0,      0,      0,     72,      0,
       0,      0,    182,      0,      0,      0,     36,      0,
     218,      0,      0,      0,    145,      0,      0,      0,
     170,      0,    113,      0,    255,      0,     56,      0,
     170,      0,    141,      0,    198,      0,     56,      0,
     170,      0,    113,      0,    226,      0,     28,      0,
     170,      0,    113,      0,    198,      0,     85,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_1: [u8; 96] = [
     229,      0,     25,      0,    102,      0,     25,      0,
     204,      0,     25,      0,     76,      0,      8,      0,
     255,      0,      8,      0,     51,      0,     25,      0,
     178,      0,     25,      0,    153,      0,    127,      0,
      28,      0,    198,      0,     56,      0,     56,      0,
     226,      0,     28,      0,    141,      0,     28,      0,
      28,      0,    170,      0,     28,      0,     28,      0,
     255,      0,    113,      0,     85,      0,     85,      0,
     159,      0,    159,      0,    255,      0,     63,      0,
     159,      0,    159,      0,    191,      0,     31,      0,
     159,      0,    127,      0,    255,      0,     31,      0,
     159,      0,    127,      0,    223,      0,     95,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_2: [u8; 96] = [
     255,      0,      0,      0,    127,      0,      0,      0,
       0,      0,    102,      0,      0,      0,    229,      0,
       0,      0,    178,      0,    204,      0,      0,      0,
      76,      0,     51,      0,    153,      0,     25,      0,
       0,      0,    127,      0,      0,      0,      0,      0,
     255,      0,    191,      0,     31,      0,     63,      0,
       0,      0,     95,      0,      0,      0,      0,      0,
     223,      0,      0,      0,     31,      0,    159,      0,
     255,      0,     85,      0,    148,      0,     85,      0,
     127,      0,     85,      0,    106,      0,     63,      0,
     212,      0,    170,      0,    191,      0,    170,      0,
      85,      0,     42,      0,    233,      0,     21,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_3: [u8; 96] = [
     255,      0,    212,      0,     63,      0,      0,      0,
     106,      0,    148,      0,     85,      0,    127,      0,
     191,      0,     21,      0,    233,      0,      0,      0,
      21,      0,    170,      0,      0,      0,     42,      0,
       0,      0,      0,      0,    141,      0,    113,      0,
     255,      0,    198,      0,      0,      0,     56,      0,
       0,      0,     85,      0,     56,      0,     28,      0,
     226,      0,     28,      0,    170,      0,     56,      0,
     255,      0,    231,      0,    255,      0,    208,      0,
     139,      0,     92,      0,    115,      0,     92,      0,
     185,      0,     69,      0,     46,      0,     46,      0,
     162,      0,     23,      0,    208,      0,     46,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_4: [u8; 96] = [
     255,      0,     31,      0,     63,      0,     63,      0,
     127,      0,     95,      0,    191,      0,     63,      0,
     223,      0,     31,      0,    159,      0,     63,      0,
      31,      0,     63,      0,     95,      0,     31,      0,
       8,      0,      0,      0,     95,      0,     63,      0,
     255,      0,      0,      0,    127,      0,      0,      0,
       8,      0,      0,      0,    159,      0,     63,      0,
     255,      0,    223,      0,    191,      0,     31,      0,
      76,      0,     25,      0,    255,      0,    127,      0,
     153,      0,     51,      0,    204,      0,    102,      0,
      76,      0,     51,      0,    229,      0,    127,      0,
     153,      0,     51,      0,    178,      0,    102,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_5: [u8; 96] = [
     255,      0,     51,      0,     25,      0,     76,      0,
       0,      0,      0,      0,    102,      0,      0,      0,
     204,      0,    229,      0,      0,      0,    178,      0,
       0,      0,    153,      0,    127,      0,      8,      0,
     178,      0,    127,      0,    153,      0,    204,      0,
     255,      0,      0,      0,     25,      0,     76,      0,
     102,      0,     51,      0,      0,      0,      0,      0,
     229,      0,     25,      0,     25,      0,    204,      0,
     178,      0,    102,      0,    255,      0,     76,      0,
     127,      0,     76,      0,    229,      0,     76,      0,
     153,      0,    102,      0,    255,      0,     25,      0,
     127,      0,     51,      0,    204,      0,     51,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_6: [u8; 96] = [
     255,      0,      0,      0,    223,      0,      0,      0,
      31,      0,      8,      0,    127,      0,      0,      0,
      95,      0,      0,      0,    159,      0,      0,      0,
      95,      0,     63,      0,    191,      0,      0,      0,
      51,      0,    204,      0,      0,      0,    102,      0,
     255,      0,    127,      0,      8,      0,    178,      0,
      25,      0,    229,      0,      0,      0,     76,      0,
     204,      0,    153,      0,     51,      0,     25,      0,
     255,      0,    226,      0,    255,      0,    255,      0,
     198,      0,     28,      0,    141,      0,     56,      0,
     170,      0,     56,      0,     85,      0,     28,      0,
     170,      0,     28,      0,    113,      0,     56,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_7: [u8; 96] = [
     223,      0,      0,      0,     63,      0,      0,      0,
      95,      0,      0,      0,    223,      0,     31,      0,
     255,      0,      0,      0,    159,      0,      0,      0,
     127,      0,     31,      0,    191,      0,     31,      0,
       0,      0,      0,      0,    109,      0,      0,      0,
     218,      0,      0,      0,    182,      0,     72,      0,
       8,      0,     36,      0,    145,      0,     36,      0,
     255,      0,      8,      0,    182,      0,     72,      0,
     255,      0,     72,      0,    218,      0,     36,      0,
     218,      0,      0,      0,    145,      0,      0,      0,
     255,      0,     36,      0,    182,      0,     36,      0,
     182,      0,      0,      0,    109,      0,      0,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_8: [u8; 96] = [
     255,      0,      0,      0,    218,      0,      0,      0,
      36,      0,      0,      0,    218,      0,      0,      0,
     182,      0,    109,      0,    255,      0,      0,      0,
       0,      0,      0,      0,    145,      0,     72,      0,
     159,      0,      0,      0,     31,      0,    127,      0,
     255,      0,     31,      0,      0,      0,     95,      0,
       8,      0,      0,      0,    191,      0,     31,      0,
     255,      0,     31,      0,    223,      0,     63,      0,
     255,      0,     31,      0,     63,      0,     31,      0,
      95,      0,     31,      0,     63,      0,    127,      0,
     159,      0,     31,      0,     63,      0,     31,      0,
     223,      0,    223,      0,    191,      0,    191,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_9: [u8; 96] = [
     226,      0,     28,      0,     28,      0,    141,      0,
       8,      0,      8,      0,    255,      0,      8,      0,
     113,      0,     28,      0,    198,      0,     85,      0,
      56,      0,    198,      0,    170,      0,     28,      0,
       8,      0,     95,      0,      8,      0,      8,      0,
     255,      0,     63,      0,     31,      0,    223,      0,
       8,      0,     31,      0,    191,      0,      8,      0,
     255,      0,    127,      0,    127,      0,    159,      0,
     115,      0,     46,      0,    255,      0,    185,      0,
     139,      0,     23,      0,    208,      0,    115,      0,
     231,      0,     69,      0,    255,      0,    162,      0,
     139,      0,    115,      0,    231,      0,     92,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_10: [u8; 96] = [
     145,      0,      0,      0,      0,      0,    109,      0,
       0,      0,      0,      0,    255,      0,    109,      0,
      72,      0,    218,      0,      0,      0,      0,      0,
      36,      0,      0,      0,    182,      0,      0,      0,
       0,      0,    127,      0,    159,      0,    127,      0,
     159,      0,    191,      0,    223,      0,     63,      0,
     255,      0,     95,      0,     31,      0,     95,      0,
      31,      0,      8,      0,     63,      0,      8,      0,
     255,      0,      0,      0,    145,      0,      0,      0,
     182,      0,    109,      0,    109,      0,    109,      0,
     218,      0,      0,      0,     72,      0,      0,      0,
     182,      0,     72,      0,    182,      0,     36,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_11: [u8; 96] = [
     255,      0,      0,      0,      0,      0,      0,      0,
       0,      0,      0,      0,      0,      0,      0,      0,
     255,      0,      0,      0,    218,      0,     72,     36,
       0,      0,    182,      0,      0,      0,    145,    109,
       0,      0,    127,      0,      0,      0,     42,      0,
     212,      0,      0,    212,      0,      0,    212,      0,
       0,      0,      0,      0,     42,      0,      0,      0,
     255,      0,      0,      0,    170,    170,    127,     85,
     145,      0,    109,    109,    218,    109,     72,      0,
     145,      0,     72,      0,    218,      0,    109,      0,
     182,      0,    109,      0,    255,      0,     72,      0,
     182,    109,     36,    109,    255,    109,    109,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_12: [u8; 96] = [
     255,      0,      0,      0,    255,      0,    191,      0,
       0,      0,      0,      0,     95,      0,     63,      0,
      31,      0,      0,      0,    223,      0,    223,      0,
       0,      0,      8,      0,    159,      0,    127,      0,
       0,      0,     85,      0,     56,      0,     28,      0,
     255,      0,     28,      0,      0,      0,    226,      0,
       0,      0,    170,      0,     56,      0,    113,      0,
     198,      0,      0,      0,    113,      0,    141,      0,
     255,      0,     42,      0,    233,      0,     63,      0,
     212,      0,     85,      0,    191,      0,    106,      0,
     191,      0,     21,      0,    170,      0,      8,      0,
     170,      0,    127,      0,    148,      0,    148,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_13: [u8; 96] = [
     255,      0,      0,      0,      0,      0,     63,      0,
     191,      0,     95,      0,     31,      0,    223,      0,
     255,      0,     63,      0,     95,      0,     63,      0,
     159,      0,      0,      0,      0,      0,    127,      0,
      72,      0,      0,      0,      0,      0,      0,      0,
     255,      0,      0,      0,      0,      0,      0,      0,
      72,      0,     72,      0,     36,      0,      8,      0,
     218,      0,    182,      0,    145,      0,    109,      0,
     255,      0,    162,      0,    231,      0,    162,      0,
     231,      0,    115,      0,    208,      0,    139,      0,
     185,      0,     92,      0,    185,      0,     46,      0,
     162,      0,     69,      0,    162,      0,     23,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_14: [u8; 96] = [
     255,      0,      0,      0,     51,      0,      0,      0,
       0,      0,      0,      0,    102,      0,      0,      0,
     204,      0,      0,      0,    153,      0,      0,      0,
       0,      0,      0,      0,     51,      0,      0,      0,
       0,      0,      0,      0,      8,      0,     36,      0,
     255,      0,      0,      0,    182,      0,      8,      0,
       0,      0,      0,      0,     72,      0,    109,      0,
     145,      0,      0,      0,    255,      0,    218,      0,
     212,      0,      8,      0,    170,      0,      0,      0,
     127,      0,      0,      0,     85,      0,      8,      0,
     255,      0,      8,      0,    170,      0,      0,      0,
     127,      0,      0,      0,     42,      0,      8,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_15: [u8; 96] = [
     255,      0,      0,      0,      0,      0,      0,      0,
      36,      0,      0,      0,    182,      0,      0,      0,
     218,      0,      0,      0,      0,      0,      0,      0,
      72,      0,      0,      0,    145,      0,    109,      0,
      36,      0,     36,      0,      0,      0,      0,      0,
     255,      0,      0,      0,    182,      0,      0,      0,
       0,      0,      0,      0,      0,      0,      0,    109,
     218,      0,      0,      0,    145,      0,     72,     72,
     255,      0,     28,      0,    226,      0,     56,      0,
     198,      0,      0,      0,      0,      0,     28,     28,
     170,      0,      0,      0,    141,      0,      0,      0,
     113,      0,      0,      0,     85,     85,     85,     85,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_16: [u8; 96] = [
     255,      0,      0,      0,      0,      0,     95,      0,
       0,      0,    127,      0,      0,      0,      0,      0,
     223,      0,     95,      0,     63,      0,     31,      0,
     191,      0,      0,      0,    159,      0,      0,      0,
       0,      0,     31,      0,    255,      0,      0,      0,
       0,      0,     95,      0,    223,      0,      0,      0,
       0,      0,     63,      0,    191,      0,      0,      0,
       0,      0,      0,      0,    159,      0,    127,      0,
     141,      0,     28,      0,     28,      0,     28,      0,
     113,      0,      8,      0,      8,      0,      8,      0,
     255,      0,      0,      0,    226,      0,      0,      0,
     198,      0,     56,      0,    170,      0,     85,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_17: [u8; 96] = [
     255,      0,      0,      0,      8,      0,      0,      0,
     182,      0,      0,      0,     72,      0,      0,      0,
     218,      0,      0,      0,     36,      0,      0,      0,
     145,      0,      0,      0,    109,      0,      0,      0,
       0,      0,     51,     25,     76,     25,     25,      0,
     153,      0,      0,      0,    127,    102,    178,      0,
     204,      0,      0,      0,      0,      0,    255,      0,
       0,      0,    102,      0,    229,      0,     76,      0,
     113,      0,      0,      0,    141,      0,     85,      0,
       0,      0,      0,      0,    170,      0,      0,      0,
      56,     28,    255,      0,      0,      0,      0,      0,
     198,      0,      0,      0,    226,      0,      0,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_18: [u8; 96] = [
     255,      0,      8,      0,     28,      0,     28,      0,
     198,      0,     56,      0,     56,      0,     85,      0,
     255,      0,     85,      0,    113,      0,    113,      0,
     226,      0,    141,      0,    170,      0,    141,      0,
       0,      0,      0,      0,      0,      0,      0,      0,
     255,      0,      0,      0,    127,      0,      0,      0,
       0,      0,      0,      0,      0,      0,      0,      0,
      63,      0,      0,      0,    191,      0,      0,      0,
     255,      0,      0,      0,    255,      0,    127,      0,
       0,      0,     85,      0,      0,      0,    212,      0,
       0,      0,    212,      0,     42,      0,    170,      0,
       0,      0,    127,      0,      0,      0,      0,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_19: [u8; 96] = [
     255,      0,      0,      0,      0,      0,    218,      0,
     182,      0,      0,      0,      0,      0,    145,      0,
     145,      0,     36,      0,      0,      0,    109,      0,
     109,      0,      0,      0,     72,      0,     36,      0,
       0,      0,      0,      0,    109,      0,      8,      0,
      72,      0,      0,      0,    255,      0,    182,      0,
       0,      0,      0,      0,    145,      0,      8,      0,
      36,      0,      8,      0,    218,      0,    182,      0,
     255,      0,      0,      0,      0,      0,    226,      0,
      85,      0,      0,      0,    141,      0,      0,      0,
       0,      0,      0,      0,    170,      0,     56,      0,
     198,      0,      0,      0,    113,      0,     28,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_20: [u8; 96] = [
     255,      0,      0,      0,    113,      0,      0,      0,
     198,      0,     56,      0,     85,      0,     28,      0,
     255,      0,      0,      0,    226,      0,      0,      0,
     170,      0,      0,      0,    141,      0,      0,      0,
       0,      0,      0,      0,      0,      0,      0,      0,
     255,      0,    145,      0,    109,      0,    218,      0,
      36,      0,    182,      0,     72,      0,     72,      0,
     255,      0,      0,      0,      0,      0,    109,      0,
      36,      0,     36,      0,    145,      0,      0,      0,
      72,      0,     72,      0,    182,      0,      0,      0,
      72,      0,     72,      0,    218,      0,      0,      0,
     109,      0,    109,      0,    255,      0,      0,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_21: [u8; 96] = [
     255,      0,      0,      0,    218,      0,      0,      0,
     145,      0,      0,      0,     36,      0,      0,      0,
     218,      0,      0,      0,     36,      0,      0,      0,
     182,      0,     72,      0,      0,      0,    109,      0,
       0,      0,      0,      0,      8,      0,      0,      0,
     255,      0,     85,      0,    212,      0,     42,      0,
       0,      0,      0,      0,      8,      0,      0,      0,
      85,      0,    170,      0,    127,      0,     42,      0,
     109,      0,    109,      0,    255,      0,      0,      0,
      72,      0,     72,      0,    218,      0,      0,      0,
     145,      0,    182,      0,    255,      0,      0,      0,
      36,      0,     36,      0,    218,      0,      8,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_22: [u8; 96] = [
     255,      0,      0,      0,     42,      0,      0,      0,
     212,      0,      0,      0,      8,      0,    212,      0,
     170,      0,      0,      0,     85,      0,      0,      0,
     212,      0,      8,      0,    127,      0,      8,      0,
     255,      0,     85,      0,      0,      0,      0,      0,
     226,      0,     85,      0,      0,      0,    198,      0,
       0,      0,    141,      0,     56,      0,      0,      0,
     170,      0,     28,      0,      0,      0,    113,      0,
     113,      0,     56,      0,    255,      0,      0,      0,
      85,      0,     56,      0,    226,      0,      0,      0,
       0,      0,    170,      0,      0,      0,    141,      0,
      28,      0,     28,      0,    198,      0,     28,      0,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_23: [u8; 96] = [
     255,      0,      0,      0,    229,      0,      0,      0,
     204,      0,    204,      0,      0,      0,     76,      0,
     178,      0,    153,      0,     51,      0,    178,      0,
     178,      0,    127,      0,    102,     51,     51,     25,
       0,      0,      0,      0,      0,      0,      0,     31,
       0,      0,      0,      0,    255,      0,      0,     31,
       0,      0,      8,      0,      0,      0,    191,    159,
     127,     95,     95,      0,    223,      0,     63,      0,
     255,      0,    255,      0,    204,    204,    204,    204,
       0,      0,     51,     51,     51,     51,      0,      0,
     204,      0,    204,      0,    153,    153,    153,    153,
     153,      0,      0,      0,    102,    102,    102,    102,
];
#[rustfmt::skip]
const GRIDS_PATTERN_LUT_24: [u8; 96] = [
     170,      0,      0,      0,      0,    255,      0,      0,
     198,      0,      0,      0,      0,     28,      0,      0,
     141,      0,      0,      0,      0,    226,      0,      0,
      56,      0,      0,    113,      0,     85,      0,      0,
     255,      0,      0,      0,      0,    113,      0,      0,
      85,      0,      0,      0,      0,    226,      0,      0,
     141,      0,      0,      8,      0,    170,     56,     56,
     198,      0,      0,     56,      0,    141,     28,      0,
     255,      0,      0,      0,      0,    191,      0,      0,
     159,      0,      0,      0,      0,    223,      0,      0,
      95,      0,      0,      0,      0,     63,      0,      0,
     127,      0,      0,      0,      0,     31,      0,      0,
];

const GRIDS_PATTERNS: [[u8; 96]; 25] = [
    GRIDS_PATTERN_LUT_0,
    GRIDS_PATTERN_LUT_1,
    GRIDS_PATTERN_LUT_2,
    GRIDS_PATTERN_LUT_3,
    GRIDS_PATTERN_LUT_4,
    GRIDS_PATTERN_LUT_5,
    GRIDS_PATTERN_LUT_6,
    GRIDS_PATTERN_LUT_7,
    GRIDS_PATTERN_LUT_8,
    GRIDS_PATTERN_LUT_9,
    GRIDS_PATTERN_LUT_10,
    GRIDS_PATTERN_LUT_11,
    GRIDS_PATTERN_LUT_12,
    GRIDS_PATTERN_LUT_13,
    GRIDS_PATTERN_LUT_14,
    GRIDS_PATTERN_LUT_15,
    GRIDS_PATTERN_LUT_16,
    GRIDS_PATTERN_LUT_17,
    GRIDS_PATTERN_LUT_18,
    GRIDS_PATTERN_LUT_19,
    GRIDS_PATTERN_LUT_20,
    GRIDS_PATTERN_LUT_21,
    GRIDS_PATTERN_LUT_22,
    GRIDS_PATTERN_LUT_23,
    GRIDS_PATTERN_LUT_24,
];

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Instrument {
    #[default]
    BD,
    SD,
    HH,
}

impl Display for Instrument {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "{}",
            match *self {
                Instrument::BD => "BD",
                Instrument::SD => "SD",
                Instrument::HH => "HH",
            }
        )
    }
}

impl TryFrom<u8> for Instrument {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Instrument::BD),
            1 => Ok(Instrument::SD),
            2 => Ok(Instrument::HH),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct GridsRhythmMachine {
    params: ParamList,
}

impl GridsRhythmMachine {
    pub fn new() -> GridsRhythmMachine {
        GridsRhythmMachine {
            params: ParamList::from_slice(&[
                Box::new(Param::new_instrument_param("INST")),
                Box::new(Param::new_number_param("TABLE", 0, 24, 0)),
                Box::new(Param::new_number_param("FILL", 0, 7, 4)),
                Box::new(Param::new_number_param("PERT", 0, 7, 0)),
            ])
            .unwrap(),
        }
    }

    fn process(
        sequence: Sequence,
        machine_resources: &mut MachineResources,
        table: u8,
        instrument: Instrument,
        fill: u8,
        perturbation: u8,
    ) -> Sequence {
        let pattern_start = 32 * instrument as usize;
        let pattern_end = pattern_start + 32;
        let pattern = &GRIDS_PATTERNS[table as usize][pattern_start..pattern_end];
        let threshold = 255 - fill * 32;
        let active_steps = pattern.iter().map(|&step_level| {
            let some_rand = machine_resources.random_u64() >> 56; // 8 bit = 0..=255
            let perturb_delta = (some_rand * perturbation as u64 >> 5) as u8;
            let level = step_level.saturating_add(perturb_delta);
            level > threshold
        });
        sequence.mask_steps(active_steps)
    }
}

impl Machine for GridsRhythmMachine {
    fn name(&self) -> &str {
        "GRIDS"
    }

    fn params(&self) -> &ParamList {
        &self.params
    }

    fn params_mut(&mut self) -> &mut ParamList {
        &mut self.params
    }

    fn apply(&self, sequence: Sequence, machine_resources: &mut MachineResources) -> Sequence {
        let instrument = self.params[0]
            .value()
            .try_into()
            .expect("unexpected instrument param for GridsRhythmMachine");
        let table = self.params[1]
            .value()
            .try_into()
            .expect("unexpected table param for GridsRhythmMachine");
        let fill = self.params[2]
            .value()
            .try_into()
            .expect("unexpected fill param for GridsRhythmMachine");
        let perturbation = self.params[3]
            .value()
            .try_into()
            .expect("unexpected perturbation param for GridsRhythmMachine");
        Self::process(
            sequence,
            machine_resources,
            table,
            instrument,
            fill,
            perturbation,
        )
    }
}

unsafe impl Send for GridsRhythmMachine {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        machine_resources::MachineResources, param::ParamValue,
        sequence_generator::SequenceGenerator,
    };

    #[test]
    fn grids_rhythm_machine_with_default_params_should_generate_default_beat() {
        let mut machine_resources = MachineResources::new();
        let machine = GridsRhythmMachine::new();
        let output_sequence = machine.apply(
            SequenceGenerator::initial_sequence(32),
            &mut machine_resources,
        );
        let active_steps: Vec<bool> = output_sequence.iter().map(|opt| opt.is_some()).collect();
        assert_eq!(
            active_steps,
            [
                true, false, false, false, false, false, true, false, false, false, false, false,
                true, false, false, false, false, false, false, false, true, false, false, false,
                false, false, false, false, false, false, false, false,
            ]
        );
    }

    #[test]
    fn grids_rhythm_machine_with_fill_maxxed_should_generate_filled_beat() {
        let mut machine_resources = MachineResources::new();
        let mut machine = GridsRhythmMachine::new();
        machine.params[2].set(ParamValue::Number(7)); // FILL
        let output_sequence = machine.apply(
            SequenceGenerator::initial_sequence(32),
            &mut machine_resources,
        );
        let active_steps: Vec<bool> = output_sequence.iter().map(|opt| opt.is_some()).collect();
        assert_eq!(
            active_steps,
            [
                true, false, false, false, false, false, true, false, false, false, false, false,
                true, false, false, false, true, false, true, false, true, false, false, false,
                true, false, false, false, true, false, false, false,
            ]
        );
    }

    #[test]
    fn grids_rhythm_machine_with_perturbation_enabled_should_flip_out_and_do_funky_shit() {
        let mut machine_resources = MachineResources::new();
        let mut machine = GridsRhythmMachine::new();
        machine.params[2].set(ParamValue::Number(7)); // FILL
        machine.params[3].set(ParamValue::Number(7)); // PERT
        let output_sequence = machine.apply(
            SequenceGenerator::initial_sequence(32),
            &mut machine_resources,
        );
        let active_steps: Vec<bool> = output_sequence.iter().map(|opt| opt.is_some()).collect();
        assert_ne!(
            active_steps,
            [
                true, false, false, false, false, false, true, false, false, false, false, false,
                true, false, false, false, true, false, true, false, true, false, false, false,
                true, false, false, false, true, false, false, false,
            ]
        );
    }
}
