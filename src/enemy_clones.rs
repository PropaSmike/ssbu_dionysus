use core::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};
use std::ffi::{c_char, CStr, CString};

pub(crate) const VANILLA_KINDS: u32 = 5;
pub(crate) const KIND_LIMIT: u32 = 32;
const NO_BASE: i32 = -1;
const NAME_LIMIT: usize = 48;

const PATH_KIND_REGISTER: usize = 1;
const PATH_NAME_REGISTER: usize = 2;
const PATH_CLONE_REGISTER: usize = 3;
const OFF_PATH_GUARD: usize = 0x17e_1dd0;
const PATH_GUARD_OPCODE: u32 = 0x7100_143f;

const OBJECT_ID: usize = 8;
const OBJECT_KIND: usize = 0xc;
const CATEGORY_SHIFT: u32 = 28;
const ENEMY_CATEGORY: u32 = 2;
const OFF_KIND_SWITCH: usize = 0x2f_9ef8;
const KIND_SWITCH_OPCODE: u32 = 0x7100_111f;
const KIND_SWITCH_KIND_REGISTER: usize = 8;
const KIND_SWITCH_OBJECT_REGISTER: usize = 22;
const OFF_AI_GET_KIND: usize = 0x37_68a0;
const AI_GET_KIND_OPCODE: u32 = 0xf81e_0ff3;
static BASE_CELLS: [u32; VANILLA_KINDS as usize] = [0, 1, 2, 3, 4];

pub(crate) const REFUSED_BASE: i32 = -2;
pub(crate) const REFUSED_NAME: i32 = -3;
pub(crate) const REFUSED_FULL: i32 = -4;

const UNSET_BASE: AtomicI32 = AtomicI32::new(NO_BASE);
const UNSET_NAME: AtomicUsize = AtomicUsize::new(0);
static BASES: [AtomicI32; KIND_LIMIT as usize] = [UNSET_BASE; KIND_LIMIT as usize];
static NAMES: [AtomicUsize; KIND_LIMIT as usize] = [UNSET_NAME; KIND_LIMIT as usize];
static NEXT: AtomicU32 = AtomicU32::new(VANILLA_KINDS);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static REGISTERING: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn base_of(kind: u32) -> Option<u32> {
    if !(VANILLA_KINDS..KIND_LIMIT).contains(&kind) {
        return None;
    }
    let base = BASES[kind as usize].load(Ordering::Acquire);
    (base != NO_BASE).then_some(base as u32)
}

pub(crate) fn known(kind: u32) -> bool {
    kind < VANILLA_KINDS || (INSTALLED.load(Ordering::Acquire) && base_of(kind).is_some())
}

fn name_pointer(kind: u32) -> usize {
    NAMES[kind as usize].load(Ordering::Acquire)
}

pub(crate) fn name_of(kind: u32) -> Option<&'static str> {
    base_of(kind)?;
    let pointer = name_pointer(kind);
    if pointer == 0 {
        return None;
    }
    unsafe { CStr::from_ptr(pointer as *const c_char) }.to_str().ok()
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_LIMIT
        && name.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

pub(crate) fn register(base: u32, name: &str) -> i32 {
    if base >= VANILLA_KINDS {
        return REFUSED_BASE;
    }
    if !valid_name(name) {
        return REFUSED_NAME;
    }
    let _guard = REGISTERING.lock();
    for kind in VANILLA_KINDS..NEXT.load(Ordering::Acquire) {
        let pointer = name_pointer(kind);
        if pointer != 0 && unsafe { CStr::from_ptr(pointer as *const c_char) }.to_bytes() == name.as_bytes() {
            if base_of(kind) != Some(base) {
                return REFUSED_NAME;
            }
            return kind as i32;
        }
    }
    let kind = NEXT.load(Ordering::Acquire);
    if kind >= KIND_LIMIT {
        return REFUSED_FULL;
    }
    let Ok(owned) = CString::new(name) else {
        return REFUSED_NAME;
    };
    NAMES[kind as usize].store(Box::leak(owned.into_boxed_c_str()).as_ptr() as usize, Ordering::Release);
    BASES[kind as usize].store(base as i32, Ordering::Release);
    NEXT.store(kind + 1, Ordering::Release);
    skyline::println!("[enemyclone] {name} = enemy kind {kind} on {base}");
    kind as i32
}

macro_rules! base_kind_sites {
    ($($name:ident = $offset:literal, $register:literal, $opcode:literal;)*) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                if let Some(base) = base_of(ctx.registers[$register].w()) {
                    ctx.registers[$register].set_x(base as u64);
                }
            }
        )*

        const BASE_KIND_SITES: &[(usize, u32)] = &[$(($offset, $opcode)),*];

        fn install_base_kind_sites() {
            skyline::install_hooks!($($name),*);
        }
    };
}

base_kind_sites! {
    object_scale_param = 0x565d30, 8, 0x7100111f;
    object_search_param = 0x565f58, 8, 0x7100111f;
    object_common_row_a = 0x566114, 8, 0x7100111f;
    object_common_row_b = 0x566528, 8, 0x7100111f;
    object_param_table_c = 0x566aac, 8, 0x7100111f;
    object_kind_work_int = 0x566828, 1, 0xf9405108;
    object_setup_tail = 0x566cd4, 8, 0x7100111f;
    object_common_row_c = 0x567188, 21, 0x710012bf;
    object_common_row_d = 0x56731c, 21, 0x710012bf;
    object_common_row_e = 0x567f58, 21, 0x710012bf;
    object_common_row_f = 0x56827c, 21, 0x710012bf;
    object_param_table_a = 0x56834c, 8, 0x7100111f;
    object_param_table_b = 0x5685d8, 8, 0x7100111f;
    object_common_row_g = 0x5696c0, 22, 0x710012df;
    object_common_row_h = 0x5699a4, 9, 0x8b091509;
    object_setup_scale = 0x56a384, 8, 0x7100111f;
    request_common_row_a = 0x56e600, 9, 0x7100113f;
    request_common_row_b = 0x56e690, 9, 0x7100113f;
    descriptor_kind_table = 0x571e54, 9, 0xb8697908;
    descriptor_common_row = 0x571e74, 8, 0x7100111f;
    descriptor_param_tables = 0x571f3c, 8, 0x7100111f;
    descriptor_tail = 0x572104, 8, 0x7100111f;
    kind_factory = 0x57fa70, 20, 0x7100129f;
    foreign_common_row_a = 0x90e8b4, 8, 0x7100111f;
    foreign_common_row_b = 0xb9f960, 8, 0x7100111f;
    foreign_common_row_c = 0xe627a8, 8, 0x8b081528;
    foreign_param_table = 0x34b0274, 25, 0x7100133f;
    foreign_common_row_d = 0x34b19f0, 8, 0x5280450a;
    foreign_common_row_e = 0x34b284c, 21, 0xf9420d08;
    foreign_common_row_f = 0x3516878, 8, 0x8b081528;
    thrown_node_kind = 0x154724c, 12, 0x6b0c01df;
}

#[skyline::hook(offset = crate::game::rel(OFF_PATH_GUARD), inline)]
unsafe fn path_guard(ctx: &mut skyline::hooks::InlineCtx) {
    let kind = ctx.registers[PATH_KIND_REGISTER].w();
    match base_of(kind) {
        Some(base) => {
            ctx.registers[PATH_KIND_REGISTER].set_x(base as u64);
            ctx.registers[PATH_CLONE_REGISTER].set_x(name_pointer(kind) as u64);
        }
        None => ctx.registers[PATH_CLONE_REGISTER].set_x(0),
    }
}

macro_rules! path_name_sites {
    ($($name:ident = $offset:literal, $opcode:literal;)*) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let name = ctx.registers[PATH_CLONE_REGISTER].x();
                if name != 0 {
                    ctx.registers[PATH_NAME_REGISTER].set_x(name);
                }
            }
        )*

        const PATH_NAME_SITES: &[(usize, u32)] = &[$(($offset, $opcode)),*];

        fn install_path_name_sites() {
            skyline::install_hooks!($($name),*);
        }
    };
}

path_name_sites! {
    path_model = 0x17e1e24, 0x910023f4;
    path_motion = 0x17e1ebc, 0x910023f4;
    path_motion_list = 0x17e1f40, 0x910023f5;
    path_effect = 0x17e1fcc, 0x910023f4;
    path_script = 0x17e2050, 0x910023f4;
    path_sound_bank = 0x17e20d4, 0x910023f5;
    path_tone_label = 0x17e2160, 0x910023f5;
    path_sound_audio = 0x17e21ec, 0x910023f4;
}

pub(crate) fn enemy_base_of_id(id: u32, kind: u32) -> Option<u32> {
    if id >> CATEGORY_SHIFT != ENEMY_CATEGORY || !INSTALLED.load(Ordering::Acquire) {
        return None;
    }
    base_of(kind)
}

#[skyline::hook(offset = crate::game::rel(OFF_KIND_SWITCH), inline)]
unsafe fn kind_switch(ctx: &mut skyline::hooks::InlineCtx) {
    let object = ctx.registers[KIND_SWITCH_OBJECT_REGISTER].x() as usize;
    if object == 0 {
        return;
    }
    let id = core::ptr::read_volatile((object + OBJECT_ID) as *const u32);
    if let Some(base) = enemy_base_of_id(id, ctx.registers[KIND_SWITCH_KIND_REGISTER].w()) {
        ctx.registers[KIND_SWITCH_KIND_REGISTER].set_x(base as u64);
    }
}

macro_rules! enemy_kind_loads {
    ($($name:ident = $offset:literal, $register:literal, $opcode:literal;)*) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let object = ctx.registers[$register].x() as usize;
                if object == 0 {
                    return;
                }
                let id = core::ptr::read_volatile((object + OBJECT_ID) as *const u32);
                let kind = core::ptr::read_volatile((object + OBJECT_KIND) as *const u32);
                if let Some(cell) = enemy_base_of_id(id, kind).and_then(|base| BASE_CELLS.get(base as usize)) {
                    ctx.registers[$register].set_x((cell as *const u32 as usize - OBJECT_KIND) as u64);
                }
            }
        )*

        const ENEMY_KIND_LOADS: &[(usize, u32)] = &[$(($offset, $opcode)),*];

        fn install_enemy_kind_loads() {
            skyline::install_hooks!($($name),*);
        }
    };
}

enemy_kind_loads! {
    yellow_devil_test = 0x6c85b8, 25, 0xb9400f28;
    flying_man_list_test = 0x30f7f4, 8, 0xb9400d08;
    dark_emperor_record_test = 0x32d2a4, 14, 0xb9400dce;
}

#[skyline::hook(offset = crate::game::rel(OFF_AI_GET_KIND))]
unsafe fn ai_get_kind(id: u32) -> i32 {
    let kind = call_original!(id);
    match enemy_base_of_id(id, kind as u32) {
        Some(base) => base as i32,
        None => kind,
    }
}

pub(crate) fn install() {
    let sites = BASE_KIND_SITES
        .iter()
        .chain(PATH_NAME_SITES)
        .chain(core::iter::once(&(OFF_PATH_GUARD, PATH_GUARD_OPCODE)))
        .chain(core::iter::once(&(OFF_KIND_SWITCH, KIND_SWITCH_OPCODE)))
        .chain(ENEMY_KIND_LOADS)
        .chain(core::iter::once(&(OFF_AI_GET_KIND, AI_GET_KIND_OPCODE)));
    for &(offset, expected) in sites {
        let observed = unsafe { crate::game::text_word(offset) };
        if observed != expected {
            skyline::println!("[enemyclone] REFUSED {offset:#x}: expected {expected:#010x}, found {observed:#010x}");
            return;
        }
    }
    install_base_kind_sites();
    install_path_name_sites();
    skyline::install_hooks!(path_guard, kind_switch, ai_get_kind);
    install_enemy_kind_loads();
    INSTALLED.store(true, Ordering::Release);
    skyline::println!(
        "[enemyclone] {} kind sites and {} path sites patched; enemy kind tests and fighter AI see a clone's base",
        BASE_KIND_SITES.len() + 1 + ENEMY_KIND_LOADS.len(),
        PATH_NAME_SITES.len() + 1
    );
}

#[no_mangle]
pub unsafe extern "C" fn clone_engine_enemy_clone(base: u32, name: *const c_char) -> i32 {
    if name.is_null() {
        return REFUSED_NAME;
    }
    match CStr::from_ptr(name).to_str() {
        Ok(name) => register(base, name),
        Err(_) => REFUSED_NAME,
    }
}
