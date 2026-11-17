use std::sync::atomic::AtomicUsize;

use crate::{resolve, Error};

type MotionRateFn = unsafe extern "C" fn(u64, f32) -> i32;
type MotionFn = unsafe extern "C" fn(u64, *mut f32, *mut f32) -> i32;
type DispOrderFn = unsafe extern "C" fn(u64, u16) -> i32;

static MOTION_RATE_FN: AtomicUsize = AtomicUsize::new(0);
static MOTION_FRAME_FN: AtomicUsize = AtomicUsize::new(0);
static MOTION_FN: AtomicUsize = AtomicUsize::new(0);
static DISP_ORDER_FN: AtomicUsize = AtomicUsize::new(0);
static SOUND_FN: AtomicUsize = AtomicUsize::new(0);
static AREA_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_FN: AtomicUsize = AtomicUsize::new(0);
static ITEMS_START_FN: AtomicUsize = AtomicUsize::new(0);

type SoundFn = unsafe extern "C" fn(u64, i32, f32, f32) -> i32;
type AreaFn = unsafe extern "C" fn(u64, f32, f32, f32, f32) -> i32;
type ItemFn = unsafe extern "C" fn(u64, *const std::ffi::c_char, i32) -> i32;
type ItemsStartFn = unsafe extern "C" fn(u64, i32) -> i32;

fn checked(result: i32) -> Result<(), Error> {
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

fn sound(place: &str, reverb: i32, scale: f32, offset: f32) -> Result<(), Error> {
    let address = resolve(&SOUND_FN, b"clone_engine_stage_sound_set\0").ok_or(Error::EngineUnavailable)?;
    let function: SoundFn = unsafe { std::mem::transmute(address) };
    checked(unsafe { function(set_hash(place), reverb, scale, offset) })
}

pub fn set_reverb(place: &str, preset: u32) -> Result<(), Error> {
    sound(place, preset.min(i32::MAX as u32) as i32, f32::NAN, f32::NAN)
}

pub fn set_sound_scale(place: &str, scale: f32) -> Result<(), Error> {
    sound(place, -1, scale, f32::NAN)
}

pub fn set_sound_offset(place: &str, offset: f32) -> Result<(), Error> {
    sound(place, -1, f32::NAN, offset)
}

pub fn set_cpu_area(place: &str, x0: f32, y0: f32, x1: f32, y1: f32) -> Result<(), Error> {
    let address = resolve(&AREA_FN, b"clone_engine_stage_area_set\0").ok_or(Error::EngineUnavailable)?;
    let function: AreaFn = unsafe { std::mem::transmute(address) };
    checked(unsafe { function(set_hash(place), x0, y0, x1, y1) })
}

pub fn set_item(place: &str, item: &str, on: bool) -> Result<(), Error> {
    let address = resolve(&ITEM_FN, b"clone_engine_stage_item_set\0").ok_or(Error::EngineUnavailable)?;
    let function: ItemFn = unsafe { std::mem::transmute(address) };
    let item = std::ffi::CString::new(item).map_err(|_| Error::InvalidName)?;
    checked(unsafe { function(set_hash(place), item.as_ptr(), i32::from(on)) })
}

pub fn set_items_start(place: &str, all: bool) -> Result<(), Error> {
    let address =
        resolve(&ITEMS_START_FN, b"clone_engine_stage_items_start\0").ok_or(Error::EngineUnavailable)?;
    let function: ItemsStartFn = unsafe { std::mem::transmute(address) };
    checked(unsafe { function(set_hash(place), i32::from(all)) })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub frame: f32,
    pub rate: f32,
}

fn set_hash(set: &str) -> u64 {
    dionysus_core::hash40(&set.to_ascii_lowercase())
}

pub fn set_disp_order(ui_stage: &str, disp_order: u16) -> Result<(), Error> {
    let address = resolve(&DISP_ORDER_FN, b"clone_engine_set_stage_disp_order\0").ok_or(Error::EngineUnavailable)?;
    let function: DispOrderFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(set_hash(ui_stage), disp_order) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn motion(set: &str) -> Result<Motion, Error> {
    let address = resolve(&MOTION_FN, b"clone_engine_stage_motion\0").ok_or(Error::EngineUnavailable)?;
    let function: MotionFn = unsafe { std::mem::transmute(address) };
    let (mut frame, mut rate) = (0.0f32, 0.0f32);
    let result = unsafe { function(set_hash(set), &mut frame, &mut rate) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(Motion { frame, rate })
}

pub fn set_motion_rate(set: &str, rate: f32) -> Result<(), Error> {
    let address =
        resolve(&MOTION_RATE_FN, b"clone_engine_stage_motion_rate\0").ok_or(Error::EngineUnavailable)?;
    let function: MotionRateFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(set_hash(set), rate) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn set_motion_frame(set: &str, frame: f32) -> Result<(), Error> {
    let address =
        resolve(&MOTION_FRAME_FN, b"clone_engine_stage_motion_frame\0").ok_or(Error::EngineUnavailable)?;
    let function: MotionRateFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(set_hash(set), frame) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}
