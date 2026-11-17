use crate::{resolve, Error};
use std::ffi::CString;
use std::sync::atomic::AtomicUsize;

pub use dionysus_core::article_hurtbox::Hurtbox;
pub use dionysus_core::article_lvd::{ArticleLvd, LvdVertex};
pub use dionysus_core::manifest_builder::{Feature, Grab, ItemManifest, Manifest, Motion};

type ManifestRegisterFn = unsafe extern "C" fn(*const std::os::raw::c_char, *const std::os::raw::c_char) -> i32;
static FIGHTER_MANIFEST_FN: AtomicUsize = AtomicUsize::new(0);
static ITEM_MANIFEST_FN: AtomicUsize = AtomicUsize::new(0);

fn call(slot: &AtomicUsize, symbol: &'static [u8], label: &str, text: &str) -> Result<i32, Error> {
    let address = resolve(slot, symbol).ok_or(Error::EngineUnavailable)?;
    let (Ok(label), Ok(text)) = (CString::new(label), CString::new(text)) else {
        return Err(Error::InvalidName);
    };
    let function: ManifestRegisterFn = unsafe { core::mem::transmute(address) };
    let result = unsafe { function(label.as_ptr(), text.as_ptr()) };
    if result < 0 {
        Err(Error::Engine(result))
    } else {
        Ok(result)
    }
}

pub fn register_fighter_manifest(label: &str, toml: &str) -> Result<i32, Error> {
    call(&FIGHTER_MANIFEST_FN, b"clone_engine_fighter_manifest_register_v1\0", label, toml)
}

pub fn register_item_manifest(label: &str, toml: &str) -> Result<i32, Error> {
    call(&ITEM_MANIFEST_FN, b"clone_engine_item_manifest_register_v1\0", label, toml)
}
