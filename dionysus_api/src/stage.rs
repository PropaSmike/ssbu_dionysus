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

#[derive(Clone, Copy, PartialEq)]
enum Row {
    Sound,
    Area,
}

pub struct StageRow {
    row: Row,
    place: u64,
    field: String,
}

pub fn sound(place: &str, field: &str) -> StageRow {
    StageRow {
        row: Row::Sound,
        place: set_hash(place),
        field: field.to_string(),
    }
}

pub fn cpu_area(place: &str, field: &str) -> StageRow {
    StageRow {
        row: Row::Area,
        place: set_hash(place),
        field: field.to_string(),
    }
}

impl StageRow {
    fn apply(&self, value: f32) -> bool {
        if value.is_nan() {
            return false;
        }
        let field = self.field.trim_end_matches('_').to_ascii_lowercase();
        match self.row {
            Row::Sound => {
                let (reverb, scale, offset) = match field.as_str() {
                    "reverb_type" if value >= 0.0 => (value.round() as i32, f32::NAN, f32::NAN),
                    "scale" => (-1, value, f32::NAN),
                    "offset" => (-1, f32::NAN, value),
                    _ => return false,
                };
                let Some(address) = resolve(&SOUND_FN, b"clone_engine_stage_sound_set\0") else {
                    return false;
                };
                let function: SoundFn = unsafe { std::mem::transmute(address) };
                unsafe { function(self.place, reverb, scale, offset) >= 0 }
            }
            Row::Area => {
                let Some(edge) = ["x0", "y0", "x1", "y1"].iter().position(|name| *name == field) else {
                    return false;
                };
                let mut edges = [f32::NAN; 4];
                edges[edge] = value;
                let Some(address) = resolve(&AREA_FN, b"clone_engine_stage_area_set\0") else {
                    return false;
                };
                let function: AreaFn = unsafe { std::mem::transmute(address) };
                unsafe { function(self.place, edges[0], edges[1], edges[2], edges[3]) >= 0 }
            }
        }
    }

    pub fn set(self, value: f64) -> bool {
        self.apply(value as f32)
    }

    pub fn int(self, value: i32) -> bool {
        self.apply(value as f32)
    }
}

type ParamGetFn = unsafe extern "C" fn(u64, u64, *mut u64) -> i32;

static PARAM_GET_FN: AtomicUsize = AtomicUsize::new(0);

pub fn param(place: &str, name: &str) -> crate::AssetParam {
    crate::AssetParam::new(crate::SPACE_STAGE, Some(set_hash(place)), name)
}

fn read_param(name: &str) -> Option<(i32, u64)> {
    let address = resolve(&PARAM_GET_FN, b"clone_engine_stage_param_get\0")?;
    let function: ParamGetFn = unsafe { std::mem::transmute(address) };
    let (group, field) = dionysus_core::prcxml::asset_key(name);
    let mut bits = 0u64;
    let tag = unsafe { function(group, field, &mut bits) };
    (tag >= 0).then_some((tag, bits))
}

pub fn param_float(name: &str) -> Option<f32> {
    match read_param(name)? {
        (0, bits) => Some(f32::from_bits(bits as u32)),
        (_, bits) => Some(bits as u32 as i32 as f32),
    }
}

pub fn param_int(name: &str) -> Option<i32> {
    match read_param(name)? {
        (0, bits) => Some(f32::from_bits(bits as u32).round() as i32),
        (_, bits) => Some(bits as u32 as i32),
    }
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
