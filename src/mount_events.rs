use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

const MOD_FILESYSTEM_MOUNTED: u32 = 1;

static SUBSCRIBED: AtomicBool = AtomicBool::new(false);
static MOUNTED: AtomicBool = AtomicBool::new(false);
static MODS_FS: AtomicBool = AtomicBool::new(false);

fn listeners() -> &'static Mutex<Vec<usize>> {
    static LISTENERS: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();
    LISTENERS.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) fn mounted() -> bool {
    MOUNTED.load(Ordering::Acquire)
}

pub(crate) fn mods_fs_mounted() -> bool {
    MODS_FS.load(Ordering::Acquire)
}

fn run(address: usize) {
    if address == 0 {
        return;
    }
    let callback: extern "C" fn() = unsafe { core::mem::transmute(address) };
    callback();
}

extern "C" fn on_mod_filesystem_mounted(_event: u32) {
    MODS_FS.store(true, Ordering::Release);
    mounted_now();
}

fn mounted_now() {
    MOUNTED.store(true, Ordering::Release);
    crate::fighter_packs::apply_params();
    let pending = listeners()
        .lock()
        .map(|list| list.clone())
        .unwrap_or_default();
    skyline::println!(
        "[mount] mod filesystem mounted; {} pack listener(s)",
        pending.len()
    );
    for address in pending {
        run(address);
    }
    #[cfg(feature = "css_slot")]
    crate::global_motions::install();
    crate::stage_select_ui::install();
    #[cfg(feature = "css_slot")]
    crate::chara_orders::watch();
    #[cfg(feature = "css_slot")]
    crate::csk_row_bridge::apply_player_orders();
}

pub(crate) fn subscribe() {
    if SUBSCRIBED.swap(true, Ordering::AcqRel) {
        return;
    }
    type Register = unsafe extern "C" fn(u32, extern "C" fn(u32));
    #[cfg(feature = "css_slot")]
    let found = unsafe { crate::css_registration::lookup_symbol(b"arcrop_register_event_callback\0") };
    #[cfg(not(feature = "css_slot"))]
    let found: Option<usize> = None;
    match found {
        Some(address) => unsafe {
            let register: Register = core::mem::transmute(address);
            register(MOD_FILESYSTEM_MOUNTED, on_mod_filesystem_mounted);
        },
        None => {
            skyline::println!(
                "[mount] arcrop_register_event_callback is not exported; fighter.toml [params] and pack mount listeners run now"
            );
            mounted_now();
        }
    }
}

pub(crate) fn add_listener(address: usize) -> i32 {
    if address == 0 {
        return dionysus_api::ERROR_NULL;
    }
    if mounted() {
        run(address);
        return dionysus_api::RESULT_OK;
    }
    match listeners().lock() {
        Ok(mut list) => {
            if !list.contains(&address) {
                list.push(address);
            }
            dionysus_api::RESULT_OK
        }
        Err(_) => dionysus_api::ERROR_UNSUPPORTED,
    }
}
