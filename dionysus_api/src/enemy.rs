
use std::sync::atomic::AtomicUsize;

use crate::{resolve, Error};

type ArmFn = unsafe extern "C" fn(u32);
type SpawnFn = unsafe extern "C" fn(u32, f32, f32, f32, f32) -> u32;
type ObjectFn = unsafe extern "C" fn(u32) -> u32;
type CloneFn = unsafe extern "C" fn(u32, *const std::ffi::c_char) -> i32;
type StatusFn = unsafe extern "C" fn(u32) -> i32;
type DriveFn = unsafe extern "C" fn(u32, usize, u32) -> i32;

pub type Driver = unsafe extern "C" fn(u32, usize, i32);

static ARM_FN: AtomicUsize = AtomicUsize::new(0);
static SPAWN_FN: AtomicUsize = AtomicUsize::new(0);
static START_FN: AtomicUsize = AtomicUsize::new(0);
static OBJECT_FN: AtomicUsize = AtomicUsize::new(0);
static CLONE_FN: AtomicUsize = AtomicUsize::new(0);
static COMMON_FN: AtomicUsize = AtomicUsize::new(0);
static CLASS_STATUS_FN: AtomicUsize = AtomicUsize::new(0);
static DRIVE_FN: AtomicUsize = AtomicUsize::new(0);
static REMOVE_FN: AtomicUsize = AtomicUsize::new(0);
static DRIVE_NOW_FN: AtomicUsize = AtomicUsize::new(0);
static VTABLE_ENTRY_FN: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_ENTRY_FN: AtomicUsize = AtomicUsize::new(0);
static OBJECT_ENTRY_FN: AtomicUsize = AtomicUsize::new(0);
static OBJECT_ORIGINAL_FN: AtomicUsize = AtomicUsize::new(0);

pub mod object_slot {
    pub const INITIALIZE: u32 = 2;
    pub const START: u32 = 3;
    pub const UPDATE_1: u32 = 106;
    pub const UPDATE_2: u32 = 107;
    pub const UPDATE_3: u32 = 108;
    pub const UPDATE_4: u32 = 109;
    pub const UPDATE_5: u32 = 110;
    pub const UPDATE_6: u32 = 111;
    pub const UPDATE_7: u32 = 112;
    pub const UPDATE_8: u32 = 113;
    pub const UPDATE_9: u32 = 114;
    pub const COUNT: u32 = 163;
}

pub mod slot {
    pub const JOINT_ROTATION: u32 = 4;
    pub const DAMAGE: u32 = 9;
    pub const CHANGE_STATUS: u32 = 73;
    pub const REQUEST_STATUS: u32 = 74;
    pub const INSTALL_STATUS: u32 = 75;
    pub const INSTALL_REQUEST: u32 = 76;
    pub const COUNT: u32 = 77;
}

pub const YELLOW_DEVIL: u32 = 0;
pub const DARK_EMPEROR: u32 = 1;
pub const FLYING_MAN: u32 = 2;
pub const METAL_FACE: u32 = 3;
pub const NABBIT: u32 = 4;

pub const OBJECTS: u32 = 4;

pub const NO_OBJECT: u32 = 0x5000_0000;

const UNAVAILABLE: u32 = u32::MAX;

pub const fn kinds(kinds: &[u32]) -> u32 {
    bits(kinds)
}

pub const fn statuses(statuses: &[u32]) -> u32 {
    bits(statuses)
}

const fn bits(numbers: &[u32]) -> u32 {
    let mut mask = 0;
    let mut index = 0;
    while index < numbers.len() {
        if numbers[index] < u32::BITS {
            mask |= 1 << numbers[index];
        }
        index += 1;
    }
    mask
}

pub fn arm(kinds: u32) -> Result<(), Error> {
    let address = resolve(&ARM_FN, b"clone_engine_enemy_arm\0").ok_or(Error::EngineUnavailable)?;
    let function: ArmFn = unsafe { std::mem::transmute(address) };
    unsafe { function(kinds) };
    Ok(())
}

pub fn spawn(kind: u32, x: f32, y: f32, z: f32, lr: f32) -> Result<u32, Error> {
    let address = resolve(&SPAWN_FN, b"clone_engine_enemy_spawn\0").ok_or(Error::EngineUnavailable)?;
    let function: SpawnFn = unsafe { std::mem::transmute(address) };
    let request = unsafe { function(kind, x, y, z, lr) };
    if request == UNAVAILABLE {
        return Err(Error::Engine(-1));
    }
    Ok(request)
}

pub fn start(request: u32) -> Option<u32> {
    let address = resolve(&START_FN, b"clone_engine_enemy_start\0")?;
    let function: ObjectFn = unsafe { std::mem::transmute(address) };
    let object = unsafe { function(request) };
    (object != NO_OBJECT).then_some(object)
}

pub fn object(index: u32) -> u32 {
    let Some(address) = resolve(&OBJECT_FN, b"clone_engine_enemy_object\0") else {
        return NO_OBJECT;
    };
    let function: ObjectFn = unsafe { std::mem::transmute(address) };
    unsafe { function(index) }
}

pub fn param(kind: u32, name: &str) -> crate::AssetParam {
    crate::AssetParam::new(crate::SPACE_ENEMY, Some(u64::from(kind)), name)
}

pub fn common(kind: u32, field: &str) -> crate::RowParam {
    crate::RowParam::new(&COMMON_FN, b"clone_engine_enemy_common_set\0", Some(kind as i32), field)
}

pub fn clone(base: u32, name: &str) -> Result<u32, Error> {
    let address = resolve(&CLONE_FN, b"clone_engine_enemy_clone\0").ok_or(Error::EngineUnavailable)?;
    let function: CloneFn = unsafe { std::mem::transmute(address) };
    let name = std::ffi::CString::new(name).map_err(|_| Error::Engine(-3))?;
    let kind = unsafe { function(base, name.as_ptr()) };
    if kind < 0 {
        return Err(Error::Engine(kind));
    }
    Ok(kind as u32)
}

pub fn class_status(object: u32) -> Option<i32> {
    let address = resolve(&CLASS_STATUS_FN, b"clone_engine_enemy_class_status\0")?;
    let function: StatusFn = unsafe { std::mem::transmute(address) };
    let status = unsafe { function(object) };
    (status >= 0).then_some(status)
}

pub fn drive(kind: u32, driver: Option<Driver>, native: u32) -> Result<(), Error> {
    let address = resolve(&DRIVE_FN, b"clone_engine_enemy_drive\0").ok_or(Error::EngineUnavailable)?;
    let function: DriveFn = unsafe { std::mem::transmute(address) };
    let result = unsafe { function(kind, driver.map_or(0, |driver| driver as usize), native) };
    if result < 0 {
        return Err(Error::Engine(result));
    }
    Ok(())
}

pub fn set_vtable_entry(kind: u32, slot: u32, function: *const ()) -> Result<(), Error> {
    let address = resolve(&VTABLE_ENTRY_FN, b"clone_engine_enemy_vtable_entry_v1\0").ok_or(Error::EngineUnavailable)?;
    let function_ptr: unsafe extern "C" fn(u32, u32, usize) -> i32 = unsafe { std::mem::transmute(address) };
    match unsafe { function_ptr(kind, slot, function as usize) } {
        0 => Ok(()),
        code => Err(Error::Engine(code)),
    }
}

pub fn original_entry(kind: u32, slot: u32) -> Option<usize> {
    let address = resolve(&ORIGINAL_ENTRY_FN, b"clone_engine_enemy_original_entry_v1\0")?;
    let function: unsafe extern "C" fn(u32, u32) -> usize = unsafe { std::mem::transmute(address) };
    match unsafe { function(kind, slot) } {
        0 => None,
        entry => Some(entry),
    }
}

pub fn set_object_vtable_entry(kind: u32, slot: u32, function: *const ()) -> Result<(), Error> {
    let address = resolve(&OBJECT_ENTRY_FN, b"clone_engine_enemy_object_entry_v1\0").ok_or(Error::EngineUnavailable)?;
    let set: unsafe extern "C" fn(u32, u32, usize) -> i32 = unsafe { std::mem::transmute(address) };
    match unsafe { set(kind, slot, function as usize) } {
        0 => Ok(()),
        code => Err(Error::Engine(code)),
    }
}

pub fn original_object_entry(slot: u32) -> Option<usize> {
    let address = resolve(&OBJECT_ORIGINAL_FN, b"clone_engine_enemy_object_original_v1\0")?;
    let function: unsafe extern "C" fn(u32) -> usize = unsafe { std::mem::transmute(address) };
    match unsafe { function(slot) } {
        0 => None,
        entry => Some(entry),
    }
}

pub fn remove(object: u32) -> bool {
    let Some(address) = resolve(&REMOVE_FN, b"clone_engine_enemy_remove\0") else {
        return false;
    };
    let function: ObjectFn = unsafe { std::mem::transmute(address) };
    unsafe { function(object) != 0 }
}

pub fn drive_now(object: u32) -> bool {
    let Some(address) = resolve(&DRIVE_NOW_FN, b"clone_engine_enemy_drive_now\0") else {
        return false;
    };
    let function: ObjectFn = unsafe { std::mem::transmute(address) };
    unsafe { function(object) != 0 }
}
