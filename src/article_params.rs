use core::sync::atomic::{AtomicU32, Ordering};
use std::sync::RwLock;

const LOOKUP_SLOT: usize = 2;
const GAME_SPAN: usize = 0x6000000;
const KEY_MASK: u64 = 0xff_ffff_ffff;
const LOG_BUDGET: u32 = 12;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Adapter {
    vtable: usize,
    data: usize,
}

type Lookup = unsafe extern "C" fn(usize, u64) -> Adapter;

struct Patched {
    vtable: usize,
    original: usize,
    remaps: Vec<(u64, u64)>,
}

static PATCHED: RwLock<Vec<Patched>> = RwLock::new(Vec::new());
static SERVED_LOG: AtomicU32 = AtomicU32::new(0);

fn entry(vtable: usize, key: u64) -> Option<(usize, Option<u64>)> {
    let patched = PATCHED.read().ok()?;
    let found = patched.iter().find(|entry| entry.vtable == vtable)?;
    let own = found
        .remaps
        .iter()
        .find(|(from, _)| *from == key & KEY_MASK)
        .map(|(_, to)| *to);
    Some((found.original, own))
}

unsafe extern "C" fn lookup(object: usize, key: u64) -> Adapter {
    let vtable = core::ptr::read_volatile(object as *const usize);
    let Some((original, own)) = entry(vtable, key) else {
        return Adapter { vtable: 0, data: 0 };
    };
    let original: Lookup = core::mem::transmute(original);
    if let Some(own) = own {
        let served = original(object, own);
        if served.data != 0 {
            if SERVED_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
                crate::dbg_log_public(&format!(
                    "[articleparams] holder {object:#x}: {key:#x} answered from the pack's block {own:#x}"
                ));
            }
            return served;
        }
    }
    original(object, key)
}

fn in_game(address: usize) -> bool {
    let base = crate::game::raw_text_base();
    address >= base && address < base + GAME_SPAN
}

pub(crate) unsafe fn serve_own_blocks(clone_kind: i32, object: usize) {
    if object == 0 {
        return;
    }
    let Some(definition) = crate::clone_definition(clone_kind) else {
        return;
    };
    let remaps = crate::custom_articles::own_param_remaps(definition.resource_name);
    if remaps.is_empty() {
        return;
    }
    let vtable = core::ptr::read_volatile(object as *const usize);
    if vtable == 0 || in_game(vtable) {
        crate::dbg_log_public(&format!(
            "[articleparams] {}: its param holder is not Smashline's, so param_{}_<article> blocks are not read",
            definition.resource_name, definition.resource_name
        ));
        return;
    }
    let slot = (vtable + LOOKUP_SLOT * 8) as *mut usize;
    let original = core::ptr::read_volatile(slot);
    if original == 0 || original == lookup as usize {
        return;
    }
    let count = remaps.len();
    let Ok(mut patched) = PATCHED.write() else {
        return;
    };
    patched.retain(|entry| entry.vtable != vtable);
    patched.push(Patched {
        vtable,
        original,
        remaps,
    });
    drop(patched);
    core::ptr::write_volatile(slot, lookup as usize);
    crate::dbg_log_public(&format!(
        "[articleparams] {} (kind {clone_kind}): holder {object:#x} reads param_{}_<article> in place of its source's block for {count} article(s)",
        definition.resource_name, definition.resource_name
    ));
}
