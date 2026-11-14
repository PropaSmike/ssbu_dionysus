use dionysus_core::menu::{self, Patch, Registry, ScriptRoot};
use core::ffi::{c_char, CStr};
use core::sync::atomic::{AtomicU32, Ordering};
use std::cell::RefCell;
use std::ffi::CString;
use std::sync::{Mutex, OnceLock};

const OFF_SCRIPT_SEARCH: usize = 0x3374a30;
const SCRIPT_SEARCH_OPCODE: u32 = 0xf81b0ffc;
const OFF_PUSH_STRING: usize = 0x38f4570;
const PUSH_STRING_OPCODE: u32 = 0xa9bc5ff8;
const OFF_SCENE_SCRIPT: usize = 0x32caa90;
const SCENE_SCRIPT_OPCODE: u32 = 0xd102c3ff;
const OFF_CHUNK_LOAD: usize = 0x372d180;
const CHUNK_LOAD_OPCODE: u32 = 0xa9bd57fc;
const MAIN_CHUNK_NAME: &[u8] = b"UiScriptPlayer";
const PATCH_ROOT: &[u8] = b"ui/script/\0";
const STATE_TOP: usize = 0x10;
const VALUE_SIZE: usize = 0x10;
const VALUE_TAG: usize = 8;
const STRING_SHORT_LENGTH: usize = 0xb;
const STRING_LONG_LENGTH: usize = 0x10;
const STRING_DATA: usize = 0x18;
const LOG_BUDGET: u32 = 64;

type PushStringFn = unsafe extern "C" fn(usize, *const c_char) -> *const c_char;

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
static FILES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static LOGS: AtomicU32 = AtomicU32::new(0);

std::thread_local! {
    static RUNNING: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static SCENE_PATCH: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

pub(crate) fn add(patch: Patch, pack: &str) -> Result<(), menu::Refusal> {
    let script = patch.script.clone();
    let Ok(mut registry) = registry().lock() else {
        return Err(menu::Refusal::Full);
    };
    registry.add_patch(patch)?;
    if let Ok(mut files) = FILES.lock() {
        if !files.iter().any(|(known, _)| *known == script) {
            let path = format!("{pack}/ui/script/{script}.lc");
            files.push((script, path));
        }
    }
    Ok(())
}

fn patches_for(root: ScriptRoot, module: &str) -> Vec<String> {
    registry()
        .lock()
        .map(|registry| registry.patches_for(root, module))
        .unwrap_or_default()
}

fn file_of(script: &str) -> Option<String> {
    FILES
        .lock()
        .ok()
        .and_then(|files| files.iter().find(|(known, _)| known == script).map(|(_, path)| path.clone()))
}

fn key(root: ScriptRoot, module: &str) -> String {
    format!("{}{module}", root.path())
}

fn log(text: core::fmt::Arguments) {
    if LOGS.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        skyline::println!("[menuscript] {text}");
    }
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn requested_module(state: usize) -> Option<String> {
    let top: usize = read(state + STATE_TOP);
    let value = top.checked_sub(VALUE_SIZE)?;
    let tag: u32 = read(value + VALUE_TAG);
    if !menu::is_lua_string(tag & 0x3f) {
        return None;
    }
    let object: usize = read(value);
    if object == 0 {
        return None;
    }
    let length = if tag & 0x3f == menu::LUA_TSHRSTR {
        usize::from(read::<u8>(object + STRING_SHORT_LENGTH))
    } else {
        read::<usize>(object + STRING_LONG_LENGTH)
    };
    if length == 0 || length > menu::MAX_MODULE_NAME {
        return None;
    }
    let bytes = core::slice::from_raw_parts((object + STRING_DATA) as *const u8, length);
    core::str::from_utf8(bytes).ok().map(str::to_ascii_lowercase)
}

fn running(key: &str) -> (usize, usize) {
    RUNNING
        .try_with(|stack| {
            let stack = stack.borrow();
            (stack.iter().filter(|known| *known == key).count(), stack.len())
        })
        .unwrap_or((usize::MAX, usize::MAX))
}

fn enter(key: &str) {
    let _ = RUNNING.try_with(|stack| stack.borrow_mut().push(key.to_string()));
}

fn leave() {
    let _ = RUNNING.try_with(|stack| stack.borrow_mut().pop());
}

#[skyline::hook(offset = crate::game::rel(OFF_SCRIPT_SEARCH))]
unsafe fn script_search(state: usize, root: *const c_char) -> u32 {
    if state == 0 || root.is_null() {
        return call_original!(state, root);
    }
    let Some(script_root) = ScriptRoot::from_path(CStr::from_ptr(root).to_bytes()) else {
        return call_original!(state, root);
    };
    let Some(module) = requested_module(state) else {
        return call_original!(state, root);
    };
    let scripts = patches_for(script_root, &module);
    let module_key = key(script_root, &module);
    let (depth, nesting) = running(&module_key);
    let Some(script) = scripts.get(depth) else {
        return call_original!(state, root);
    };
    if nesting >= menu::MAX_PATCH_DEPTH {
        log(format_args!("{module_key}: patches nest {nesting} deep; the game's module is used"));
        return call_original!(state, root);
    }
    let Ok(name) = CString::new(script.as_str()) else {
        return call_original!(state, root);
    };
    let top: usize = read(state + STATE_TOP);
    let push: PushStringFn = core::mem::transmute(crate::game::addr(OFF_PUSH_STRING));
    push(state, name.as_ptr());
    enter(&module_key);
    let found = call_original!(state, PATCH_ROOT.as_ptr() as *const c_char);
    leave();
    if found != 0 {
        log(format_args!("{module_key} runs through {script}"));
        return found;
    }
    core::ptr::write_volatile((state + STATE_TOP) as *mut usize, top);
    log(format_args!(
        "{module_key}: patch {script} did not load or failed; the module runs without it"
    ));
    call_original!(state, root)
}

fn scene_patch(hash: u64) -> Option<(String, String)> {
    let (root, module) = registry().lock().ok()?.patched_script(hash)?;
    let module_key = key(root, &module);
    let depth = running(&module_key).0;
    let script = patches_for(root, &module).get(depth)?.clone();
    Some((module_key, script))
}

#[skyline::hook(offset = crate::game::rel(OFF_SCENE_SCRIPT))]
unsafe fn scene_script(player: usize, files: usize, descriptor: usize, hash: u64, flags: u32) -> u64 {
    #[cfg(feature = "css_slot")]
    crate::starlight_bridge::arm();
    let descriptor = crate::menu_viewer::scene_descriptor(hash, descriptor);
    let Some(patch) = scene_patch(hash) else {
        return call_original!(player, files, descriptor, hash, flags);
    };
    let _ = SCENE_PATCH.try_with(|pending| *pending.borrow_mut() = Some(patch));
    let result = call_original!(player, files, descriptor, hash, flags);
    let _ = SCENE_PATCH.try_with(|pending| pending.borrow_mut().take());
    result
}

#[skyline::hook(offset = crate::game::rel(OFF_CHUNK_LOAD))]
unsafe fn chunk_load(state: usize, data: *const u8, size: usize, name: *const c_char, mode: u32) -> u32 {
    if name.is_null() || CStr::from_ptr(name).to_bytes() != MAIN_CHUNK_NAME {
        return call_original!(state, data, size, name, mode);
    }
    let Some((module_key, script)) = SCENE_PATCH.try_with(|pending| pending.borrow_mut().take()).ok().flatten() else {
        return call_original!(state, data, size, name, mode);
    };
    let Some(path) = file_of(&script) else {
        log(format_args!("{module_key}: no file for patch {script}; the scene runs its own script"));
        return call_original!(state, data, size, name, mode);
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            log(format_args!("{module_key}: {path} is not readable ({error:?}); the scene runs its own script"));
            return call_original!(state, data, size, name, mode);
        }
    };
    let top: usize = read(state + STATE_TOP);
    enter(&module_key);
    let status = call_original!(state, bytes.as_ptr(), bytes.len(), name, mode);
    leave();
    if status == 0 {
        log(format_args!("{module_key} (scene script) runs through {script}"));
        return status;
    }
    core::ptr::write_volatile((state + STATE_TOP) as *mut usize, top);
    log(format_args!(
        "{module_key}: scene patch {script} failed ({status}); the scene runs its own script"
    ));
    call_original!(state, data, size, name, mode)
}

pub(crate) fn install() {
    let has_patches = registry().lock().map(|registry| registry.has_patches()).unwrap_or(false);
    if !has_patches {
        return;
    }
    let search = unsafe { crate::game::text_word(OFF_SCRIPT_SEARCH) };
    let push = unsafe { crate::game::text_word(OFF_PUSH_STRING) };
    if search != SCRIPT_SEARCH_OPCODE || push != PUSH_STRING_OPCODE {
        skyline::println!(
            "[menuscript] REFUSED script search {OFF_SCRIPT_SEARCH:#x} / lua_pushstring {OFF_PUSH_STRING:#x}: found {search:#010x} / {push:#010x}; menu.toml patches stay off"
        );
        return;
    }
    skyline::install_hook!(script_search);
    let scene = unsafe { crate::game::text_word(OFF_SCENE_SCRIPT) };
    let load = unsafe { crate::game::text_word(OFF_CHUNK_LOAD) };
    if scene != SCENE_SCRIPT_OPCODE || load != CHUNK_LOAD_OPCODE {
        skyline::println!(
            "[menuscript] REFUSED script player setup {OFF_SCENE_SCRIPT:#x} / chunk load {OFF_CHUNK_LOAD:#x}: found {scene:#010x} / {load:#010x}; scene script patches stay off"
        );
        return;
    }
    skyline::install_hook!(scene_script);
    skyline::install_hook!(chunk_load);
}
