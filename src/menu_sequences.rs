use dionysus_core::menu::{self, Registry, SceneName, Sequence};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const MOD_ROOT: &str = "sd:/ultimate/mods";
const OFF_ROOT_TYPE_READ: usize = 0x1bf31f0;
const ROOT_TYPE_READ_OPCODE: u32 = 0xf81c0ff7;
const PRELOAD_LOG_BUDGET: u32 = 32;
const OFF_MAIN_MENU_DESCRIPTOR: usize = 0x50ab620;
const OFF_MAIN_MENU_FILES: usize = 0x4543ec8;
const MAIN_MENU_LAYOUT_COUNT: u64 = 6;
const MAIN_MENU_SHOW_RETURN: usize = 0x1bf3068;
const SHOW_INDEX_REGISTER: usize = 21;
const LINK_REGISTER: usize = 30;
const POPUP_LAYER: u64 = 1;
const OFF_ROUTER: usize = 0x235cbf0;
const ROUTER_OPCODE: u32 = 0xd104c3ff;
const OFF_SCENE_CHANGE: usize = 0x3725c90;
const ROUTER_MANAGER: u64 = 8;
const MANAGER_REQUEST: u64 = 0x20;
const MANAGER_CURRENT: u64 = 0x38;
const CURRENT_NAME: u64 = 0x10;
const QUEUE_FIRST: u64 = 8;
const NODE_TARGET: u64 = 0x10;
const NODE_NEXT: u64 = 8;
const NODE_FROM: u64 = 0x58;
const NAME_TEXT: u64 = 8;
const QUEUE_SHOWN: usize = 6;

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
static PRIORITIES: OnceLock<Vec<u64>> = OnceLock::new();
static ACTIVE: AtomicU32 = AtomicU32::new(0);
static FLOW: AtomicU32 = AtomicU32::new(0);
static PRELOAD_LOGS: AtomicU32 = AtomicU32::new(0);
static PRELOADS: AtomicBool = AtomicBool::new(false);
static EXTENDED: AtomicBool = AtomicBool::new(false);
static LAYOUTS_PENDING: AtomicBool = AtomicBool::new(false);
static DIRECT: AtomicU32 = AtomicU32::new(0);
static MANAGER: AtomicU64 = AtomicU64::new(0);

type SceneChangeFn = unsafe extern "C" fn(u64, *const SceneName, *const SceneName, u64) -> u64;

pub(crate) type FileRequestFn = unsafe extern "C" fn(usize, *const u64, i64, u32, u32) -> u64;

fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

pub(crate) fn register(code: u32, runs_as: u32, scene: Option<u8>) -> Result<(), menu::Refusal> {
    let Ok(mut registry) = registry().lock() else {
        return Err(menu::Refusal::Full);
    };
    registry
        .add_sequence(Sequence {
            code,
            runs_as,
            scene,
            flow: menu::Flow::default(),
            id: 0,
        })
        .map(|_| ())
}

pub(crate) fn code_of(id: u64) -> Option<u32> {
    registry().lock().ok()?.code_of(id)
}

pub(crate) fn active() -> u32 {
    ACTIVE.load(Ordering::Acquire)
}

pub(crate) fn active_flow() -> menu::Flow {
    menu::Flow::from_bits(FLOW.load(Ordering::Acquire))
}

fn sequence(code: u32) -> Option<Sequence> {
    registry().lock().ok().and_then(|registry| registry.sequence(code))
}

fn files_after(hash: u64) -> Vec<menu::PreloadFile> {
    registry()
        .lock()
        .map(|registry| registry.files_after(hash))
        .unwrap_or_default()
}

#[cfg(not(test))]
#[skyline::hook(offset = crate::game::rel(OFF_ROOT_TYPE_READ))]
unsafe fn root_type_read(script: u64) -> u32 {
    let code = call_original!(script);
    match sequence(code) {
        Some(found) => {
            ACTIVE.store(code, Ordering::Release);
            FLOW.store(found.flow.bits(), Ordering::Release);
            if let Some(index) = menu::mode_of_code(code) {
                crate::menu_modes::chosen(index);
            }
            if !found.flow.is_empty() {
                skyline::println!("[menuflow] {code:#010x} runs {:#010x} with {:?}", found.runs_as, found.flow);
            }
            let alias = found.runs_as;
            match found.scene.and_then(|index| menu::SCENES.get(usize::from(index))) {
                Some(scene) => {
                    DIRECT.store(u32::from(found.scene.unwrap_or(0)) + 1, Ordering::Release);
                    skyline::println!(
                        "[menuseq] main menu chose {code:#010x}; it runs as {alias:#010x} in {scene}"
                    );
                }
                None => {
                    DIRECT.store(0, Ordering::Release);
                    skyline::println!("[menuseq] main menu chose {code:#010x}; it runs as {alias:#010x}");
                }
            }
            alias
        }
        None => {
            ACTIVE.store(0, Ordering::Release);
            FLOW.store(0, Ordering::Release);
            DIRECT.store(0, Ordering::Release);
            if menu::is_custom(code) {
                skyline::println!(
                    "[menuseq] main menu chose {code:#010x}, which no menu.toml declares; the game will not know it"
                );
            }
            code
        }
    }
}

pub(crate) unsafe fn request_after(request: FileRequestFn, loader: usize, hash: *const u64, index: i64, flags: u32) {
    if !PRELOADS.load(Ordering::Acquire) || hash.is_null() {
        return;
    }
    let trigger = core::ptr::read_volatile(hash);
    let files = files_after(trigger);
    if files.is_empty() {
        return;
    }
    for file in &files {
        let wanted = file.hash;
        request(loader, &wanted, index, flags, u32::from(file.script));
    }
    if PRELOAD_LOGS.fetch_add(1, Ordering::Relaxed) < PRELOAD_LOG_BUDGET {
        skyline::println!(
            "[menuseq] requested {} file(s) after {:#012x}",
            files.len(),
            trigger & menu::HASH_MASK
        );
    }
}

#[cfg(not(test))]
unsafe fn read_u64(address: u64) -> u64 {
    core::ptr::read_volatile(address as *const u64)
}

#[cfg(not(test))]
unsafe fn current_scene(manager: u64) -> String {
    let current = read_u64(manager + MANAGER_CURRENT);
    if current == 0 {
        return String::new();
    }
    let text = (current + CURRENT_NAME) as *const core::ffi::c_char;
    core::ffi::CStr::from_ptr(text).to_string_lossy().into_owned()
}

#[cfg(not(test))]
#[skyline::hook(offset = crate::game::rel(OFF_ROUTER))]
unsafe fn router(menu_scene: u64) {
    let pending = DIRECT.swap(0, Ordering::AcqRel);
    let scene = pending
        .checked_sub(1)
        .and_then(|index| menu::SCENES.get(index as usize));
    let manager = if menu_scene == 0 { 0 } else { read_u64(menu_scene + ROUTER_MANAGER) };
    if manager != 0 {
        MANAGER.store(manager, Ordering::Release);
    }
    let (Some(scene), true) = (scene, manager != 0) else {
        return call_original!(menu_scene);
    };
    change_scene(manager, scene);
    skyline::println!("[menuseq] {scene} starts straight from the main menu");
}

#[cfg(not(test))]
unsafe fn change_scene(manager: u64, scene: &str) {
    let request = read_u64(manager + MANAGER_REQUEST);
    let from = SceneName::new(&current_scene(manager));
    let target = SceneName::new(scene);
    let back = SceneName::new(menu::MENU_SCENE);
    let change: SceneChangeFn = core::mem::transmute(crate::game::addr(OFF_SCENE_CHANGE));
    change(request, &target, &from, 0);
    change(request, &back, &from, 0);
}

#[cfg(not(test))]
unsafe fn queue_next(manager: u64, scene: &str) {
    let request = read_u64(manager + MANAGER_REQUEST);
    let first = read_u64(request + QUEUE_FIRST);
    let from = SceneName::new(menu::MENU_SCENE);
    let change: SceneChangeFn = core::mem::transmute(crate::game::addr(OFF_SCENE_CHANGE));
    if first != 0 && first != request {
        core::ptr::write_volatile((first + NODE_TARGET) as *mut SceneName, SceneName::new(scene));
    } else {
        change(request, &SceneName::new(scene), &from, 0);
    }
    change(request, &SceneName::new(menu::MENU_SCENE), &from, 0);
}

#[cfg(not(test))]
pub(crate) fn describe_queue() -> String {
    let manager = MANAGER.load(Ordering::Acquire);
    if manager == 0 {
        return "no manager".to_string();
    }
    unsafe {
        let request = read_u64(manager + MANAGER_REQUEST);
        let mut node = read_u64(request + QUEUE_FIRST);
        let mut names = Vec::new();
        while node != 0 && node != request && names.len() < QUEUE_SHOWN {
            let target = (node + NODE_TARGET + NAME_TEXT) as *const core::ffi::c_char;
            let from = (node + NODE_FROM + NAME_TEXT) as *const core::ffi::c_char;
            names.push(format!(
                "{} from {}",
                core::ffi::CStr::from_ptr(target).to_string_lossy(),
                core::ffi::CStr::from_ptr(from).to_string_lossy()
            ));
            node = read_u64(node + NODE_NEXT);
        }
        format!("[{}] in {}", names.join(", "), current_scene(manager))
    }
}

#[cfg(not(test))]
pub(crate) fn next_battle() -> Result<(), i32> {
    let manager = MANAGER.load(Ordering::Acquire);
    if manager == 0 {
        return Err(-1);
    }
    let Some(scene) = menu::scene_index(menu::BATTLE_SCENE).and_then(|index| menu::SCENES.get(usize::from(index))) else {
        return Err(-2);
    };
    unsafe { queue_next(manager, scene) };
    if let Err(refusal) = crate::menu_battle::end_battle() {
        return Err(refusal.code());
    }
    skyline::println!("[menuseq] {scene} restarts with the current selections; queue {}", describe_queue());
    Ok(())
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn clone_engine_next_battle() -> i32 {
    match next_battle() {
        Ok(()) => 0,
        Err(code) => code,
    }
}

#[cfg(not(test))]
pub(crate) unsafe fn place_layout(ctx: &mut skyline::hooks::InlineCtx) {
    crate::menu_loading::place_layout(ctx);
    if crate::menu_movie::place_layout(ctx) {
        return;
    }
    if !EXTENDED.load(Ordering::Acquire) {
        return;
    }
    let caller = crate::game::offset_of(ctx.registers[LINK_REGISTER].x() as usize);
    if caller != MAIN_MENU_SHOW_RETURN {
        return;
    }
    let index = ctx.registers[SHOW_INDEX_REGISTER].x();
    if index < MAIN_MENU_LAYOUT_COUNT {
        return;
    }
    let priority = PRIORITIES
        .get()
        .and_then(|priorities| priorities.get((index - MAIN_MENU_LAYOUT_COUNT) as usize).copied())
        .unwrap_or(menu::PACK_LAYOUT_PRIORITY + index);
    ctx.registers[1].set_x(POPUP_LAYER);
    ctx.registers[2].set_x(priority);
}

#[cfg(not(test))]
unsafe fn extend_main_menu(extra: &[u64]) -> Result<usize, String> {
    let descriptor = crate::game::addr(OFF_MAIN_MENU_DESCRIPTOR) as *mut u64;
    let table = core::ptr::read_volatile(descriptor) as usize;
    let count = core::ptr::read_volatile(descriptor.add(1));
    let expected = crate::game::addr(OFF_MAIN_MENU_FILES);
    if table != expected || count != MAIN_MENU_LAYOUT_COUNT {
        return Err(format!("descriptor holds {table:#x} x {count}, expected {expected:#x} x {MAIN_MENU_LAYOUT_COUNT}"));
    }
    let vanilla = table as *const u64;
    let first = core::ptr::read_volatile(vanilla) & menu::HASH_MASK;
    if first != menu::MAIN_MENU_LAYOUT {
        return Err(format!("first layout is {first:#012x}"));
    }
    let mut layouts: Vec<u64> = (0..count as usize)
        .map(|index| core::ptr::read_volatile(vanilla.add(index)))
        .collect();
    layouts.extend_from_slice(extra);
    let layouts: &'static [u64] = Box::leak(layouts.into_boxed_slice());
    core::ptr::write_volatile(descriptor.add(1), layouts.len() as u64);
    core::ptr::write_volatile(descriptor, layouts.as_ptr() as u64);
    EXTENDED.store(true, Ordering::Release);
    Ok(layouts.len())
}

#[cfg(not(test))]
fn install(preloads: bool) {
    unsafe {
        let observed = crate::game::text_word(OFF_ROOT_TYPE_READ);
        if observed != ROOT_TYPE_READ_OPCODE {
            skyline::println!(
                "[menuseq] REFUSED root_type reader {OFF_ROOT_TYPE_READ:#x}: expected {ROOT_TYPE_READ_OPCODE:#010x}, found {observed:#010x}; custom menu codes stay unrouted"
            );
            return;
        }
        skyline::install_hook!(root_type_read);
        let observed = crate::game::text_word(OFF_ROUTER);
        if observed == ROUTER_OPCODE {
            skyline::install_hook!(router);
        } else {
            skyline::println!(
                "[menuseq] REFUSED router {OFF_ROUTER:#x}: expected {ROUTER_OPCODE:#010x}, found {observed:#010x}; sequences with a scene run their vanilla mode"
            );
        }
    }
    crate::menu_calls::install();
    crate::menu_battle::install();
    crate::battle_pause::install();
    crate::battle_music::install();
    crate::menu_movie::install();
    crate::menu_loading::install();
    if !preloads {
        return;
    }
    if !crate::hud_layouts::armed() {
        skyline::println!("[menuseq] the file request hook is off (see [hudlayout]); menu.toml preloads are ignored");
        return;
    }
    PRELOADS.store(true, Ordering::Release);
}

#[cfg(not(test))]
pub(crate) fn load_all() {
    let mut packs: Vec<(String, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(MOD_ROOT) {
        for entry in entries.flatten() {
            let directory = entry.path();
            let Ok(text) = std::fs::read_to_string(directory.join("menu.toml")) else {
                continue;
            };
            if !crate::mod_folders::enabled(&directory) {
                continue;
            }
            packs.push((
                directory
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                text,
            ));
        }
    }
    packs.sort_by(|a, b| a.0.cmp(&b.0));
    for (directory, text) in &packs {
        let declaration = match menu::parse(text) {
            Ok(declaration) => declaration,
            Err(error) => {
                skyline::println!("[menuseq] {directory}/menu.toml is not readable: {error:?}; skipped");
                continue;
            }
        };
        let Ok(mut registry) = registry().lock() else {
            return;
        };
        for (sequence, id) in declaration.sequences.into_iter().zip(declaration.sequence_ids) {
            let named = if id.is_empty() { String::new() } else { format!("{id}, ") };
            match registry.add_sequence(sequence) {
                Ok(code) => skyline::println!(
                    "[menuseq] {directory}: {named}code {:#010x} runs as {:#010x}{}",
                    code,
                    sequence.runs_as,
                    sequence
                        .scene
                        .and_then(|index| menu::SCENES.get(usize::from(index)))
                        .map(|scene| format!(" in {scene}"))
                        .unwrap_or_default()
                ),
                Err(refusal) => skyline::println!(
                    "[menuseq] {directory}: {named}code {:#010x} REFUSED ({refusal:?})",
                    sequence.code
                ),
            }
        }
        for layout in declaration.main_menu_layouts {
            match registry.add_main_menu_layout(layout) {
                Ok(()) if declaration.under_movies.contains(&layout) => {
                    registry.under_movies(layout);
                    skyline::println!("[menuseq] {directory}: main menu layout {layout:#012x} under movies")
                }
                Ok(()) if declaration.over_side_panel.contains(&layout) => {
                    registry.cover_side_panel(layout);
                    skyline::println!("[menuseq] {directory}: main menu layout {layout:#012x} over the side panel")
                }
                Ok(()) => skyline::println!("[menuseq] {directory}: main menu layout {layout:#012x}"),
                Err(refusal) => skyline::println!(
                    "[menuseq] {directory}: main menu layout {layout:#012x} REFUSED ({refusal:?})"
                ),
            }
        }
        for patch in declaration.patches {
            let (module, script) = (format!("{}{}", patch.root.path(), patch.module), patch.script.clone());
            match crate::menu_scripts::add(patch, &format!("{MOD_ROOT}/{directory}")) {
                Ok(()) => skyline::println!("[menuseq] {directory}: {module} runs through {script}"),
                Err(refusal) => skyline::println!(
                    "[menuseq] {directory}: patch {script} for {module} REFUSED ({refusal:?})"
                ),
            }
        }
        for preload in declaration.preloads {
            let count = preload.files.len();
            let after = preload.after;
            match registry.add_preload(preload) {
                Ok(()) => skyline::println!(
                    "[menuseq] {directory}: {count} file(s) load after {after:#012x}"
                ),
                Err(refusal) => skyline::println!(
                    "[menuseq] {directory}: preload after {after:#012x} REFUSED ({refusal:?})"
                ),
            }
        }
        for layout in &declaration.gallery_layouts {
            let preload = menu::Preload {
                after: menu::GALLERY_HOST_LAYOUT,
                files: vec![menu::PreloadFile { hash: *layout, script: false }],
            };
            if let Err(refusal) = registry.add_preload(preload) {
                skyline::println!("[menuseq] {directory}: gallery layout {layout:#012x} REFUSED ({refusal:?})");
            }
        }
        crate::menu_viewer::declare(declaration.galleries, declaration.gallery_items, declaration.gallery_layouts, declaration.gallery_music);
        for mode in declaration.modes {
            let id = mode.id.clone();
            match registry.add_mode(mode) {
                Ok(()) => skyline::println!("[menumode] {directory}: mode {id}"),
                Err(refusal) => skyline::println!("[menumode] {directory}: mode {id} REFUSED ({refusal:?})"),
            }
        }
    }
    settle_modes();
    let (preloads, has_layouts) = registry()
        .lock()
        .map(|registry| (registry.wants_requests(), !registry.main_menu_layouts().is_empty()))
        .unwrap_or_default();
    crate::menu_scripts::install();
    if registry().lock().map(|registry| registry.has_flows()).unwrap_or(false) {
        crate::menu_flow::install();
    }
    install(preloads);
    if !has_layouts || !PRELOADS.load(Ordering::Acquire) {
        return;
    }
    LAYOUTS_PENDING.store(true, Ordering::Release);
    if crate::mount_events::mounted() {
        extend_pending();
    }
}

type DecompressedSizeFn = unsafe extern "C" fn(u64, *mut usize) -> bool;

#[cfg(not(test))]
fn served_check() -> Option<impl Fn(u64) -> bool> {
    let mut address = 0usize;
    let found = unsafe {
        skyline::nn::ro::LookupSymbol(&mut address, b"arcrop_get_decompressed_size\0".as_ptr())
    };
    if found != 0 || address == 0 {
        return None;
    }
    let size: DecompressedSizeFn = unsafe { core::mem::transmute(address) };
    let served = move |hash: u64| {
        let mut bytes = 0usize;
        unsafe { size(hash & menu::HASH_MASK, &mut bytes) }
    };
    served(menu::MAIN_MENU_LAYOUT).then_some(served)
}

#[cfg(not(test))]
pub(crate) fn extend_pending() {
    if !LAYOUTS_PENDING.swap(false, Ordering::AcqRel) {
        return;
    }
    let check = served_check();
    let checked = check.is_some();
    let Ok((layouts, priorities, dropped)) = registry().lock().map(|mut registry| {
        let dropped = match &check {
            Some(served) => registry.keep_main_menu_layouts(served),
            None => Vec::new(),
        };
        (
            registry.main_menu_layouts().to_vec(),
            registry.layout_priorities(MAIN_MENU_LAYOUT_COUNT),
            dropped,
        )
    }) else {
        return;
    };
    for hash in dropped {
        skyline::println!(
            "[menuseq] main menu layout {hash:#012x} is not in the game's files (its pack is switched off in ARCropolis, lacks the file, or lost it to a file conflict); left out so the main menu opens"
        );
    }
    if !checked {
        skyline::println!("[menuseq] ARCropolis cannot say which files it serves; every pack layout is kept");
    }
    let _ = PRIORITIES.set(priorities);
    if layouts.is_empty() {
        return;
    }
    match unsafe { extend_main_menu(&layouts) } {
        Ok(total) => skyline::println!(
            "[menuseq] main menu builds {total} layouts, {} from packs",
            layouts.len()
        ),
        Err(reason) => skyline::println!(
            "[menuseq] REFUSED to extend the main menu layouts: {reason}"
        ),
    }
}

#[cfg(not(test))]
fn settle_modes() {
    let Ok(mut registry) = registry().lock() else {
        return;
    };
    for (index, refusal) in registry.settle_modes() {
        let id = registry.modes().get(index).map(|mode| mode.id.clone()).unwrap_or_default();
        skyline::println!("[menumode] mode {id} gets no code ({refusal:?}); its sequence will not start");
    }
    let buttons: Vec<(String, u64)> = registry
        .modes()
        .iter()
        .filter_map(|mode| mode.button.as_deref().map(|path| (mode.id.clone(), menu::file(path).hash)))
        .collect();
    for (id, hash) in buttons {
        match registry.add_main_menu_layout(hash) {
            Ok(()) => registry.cover_side_panel(hash),
            Err(refusal) => skyline::println!("[menumode] mode {id}: button layout REFUSED ({refusal:?})"),
        }
    }
    let list = registry
        .modes()
        .iter()
        .enumerate()
        .map(|(index, mode)| (mode.clone(), registry.mode_sequence(index).unwrap_or(0)))
        .collect();
    drop(registry);
    crate::menu_modes::settle(list);
}

#[cfg(test)]
pub(crate) fn load_all() {}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn clone_engine_register_sequence(code: u32, runs_as: u32) -> i32 {
    match register(code, runs_as, None) {
        Ok(()) => 0,
        Err(refusal) => refusal.code(),
    }
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn clone_engine_active_sequence() -> u32 {
    active()
}
