use std::sync::atomic::AtomicUsize;

use crate::{resolve, Error};

type MotionRateFn = unsafe extern "C" fn(u64, f32) -> i32;
type MotionFn = unsafe extern "C" fn(u64, *mut f32, *mut f32) -> i32;
type DispOrderFn = unsafe extern "C" fn(u64, u16) -> i32;

static MOTION_RATE_FN: AtomicUsize = AtomicUsize::new(0);
static MOTION_FRAME_FN: AtomicUsize = AtomicUsize::new(0);
static MOTION_FN: AtomicUsize = AtomicUsize::new(0);
static DISP_ORDER_FN: AtomicUsize = AtomicUsize::new(0);

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
