use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::RwLock;

const VANILLA_STAGE_IDS: u32 = 364;
const STAGE_ID_GETTER: usize = 0x25d5d60;
const STAGE_RANDOM_SETTINGS: usize = 0x177a980;
const STAGE_RANDOM_SETTINGS_OPCODE: u32 = 0xfc190fe8;
const ENEMY_STAGE_FIRST: u32 = 0xc5;
const ENEMY_STAGE_SECOND: usize = 0x134b0b8;
const ENEMY_STAGE_SECOND_OPCODE: u32 = 0x51031508;
const ENEMY_STAGE_SECOND_REGISTER: usize = 8;
const STAGE_ID_FIELD: usize = 8;
const LUA2CPP_PREFIX: &[u8] = b"_ZN7lua2cpp";
const NRO_MOD0_AT: usize = 4;
const MOD0_MAGIC: u32 = u32::from_le_bytes(*b"MOD0");
const DT_NULL: u64 = 0;
const DT_HASH: u64 = 4;
const DT_STRTAB: u64 = 5;
const DT_SYMTAB: u64 = 6;
const DT_STRSZ: u64 = 10;
const DT_SYMENT: u64 = 11;
const MAX_DYNAMIC: usize = 512;
const MAX_SYMBOLS: usize = 400_000;
const SYMBOL_SIZE: usize = 24;
const LOG_BUDGET: u32 = 24;

const GETTER_RETURNS: [usize; 60] = [
    0x45504c, 0x4add18, 0x4b1880, 0x4b60ac, 0x61af38, 0x624ba8,
    0x642834, 0x6438d8, 0x643944, 0x643b18, 0x643ba4, 0x643bd0,
    0x68f9fc, 0x6970b8, 0x698ea8, 0x89293c, 0x8dce3c, 0x8ddd44,
    0x9ddd98, 0xadd6d0, 0xaf88a8, 0xb0d4c0, 0xb0e5a8, 0xe0ac6c,
    0xf0ffd0, 0x116e454, 0x12aed94, 0x1383dfc, 0x13849e8, 0x1647264,
    0x1659054, 0x1f985fc, 0x2252b7c, 0x24b3124, 0x33be82c, 0x33c7d40,
    0x33c8fac, 0x33df52c, 0x33e7fb8, 0x33ec6ec, 0x33f3eec, 0x3414bfc,
    0x341613c, 0x34326cc, 0x343306c, 0x34433e4, 0x34453b0, 0x346c0dc,
    0x347025c, 0x347602c, 0x34997bc, 0x34a7614, 0x34a77fc, 0x34a7ac8,
    0x34becec, 0x34c0874, 0x34d2288, 0x34d4bcc, 0x3531f5c, 0x353673c,
];

const STAGE_VTABLES: [usize; 186] = [
    0x50c44c8, 0x50c4d70, 0x50c61f0, 0x50c6af8, 0x50c7b00, 0x50c84d8,
    0x50c98d8, 0x50cb6b0, 0x50cc068, 0x50cd7b8, 0x50ce728, 0x50cf160,
    0x50cfb98, 0x50d05d0, 0x50d0ed8, 0x50d22b8, 0x50d3c20, 0x50d44c8,
    0x50d5f78, 0x50d6c28, 0x50d7598, 0x50d7e40, 0x50d86e8, 0x50d9120,
    0x50d99c8, 0x50da2f0, 0x50dd878, 0x50de280, 0x50df578, 0x50dfe20,
    0x50e06c8, 0x50e1f08, 0x50e2808, 0x50e30e8, 0x50e3ab0, 0x50e49c0,
    0x50e52a8, 0x50e5fb0, 0x50e6d60, 0x50e7a18, 0x50e8810, 0x50e9588,
    0x50e9eb0, 0x50ea798, 0x50ee5f0, 0x50ef248, 0x50efb70, 0x50f0fd0,
    0x50f2f78, 0x50f4e58, 0x50f59f8, 0x50f6838, 0x50f7120, 0x50f7f00,
    0x50f87a8, 0x50f9840, 0x50fa388, 0x50fb468, 0x50fbec0, 0x50fc7a8,
    0x50fdca0, 0x50fe5c8, 0x50ffe50, 0x51009d8, 0x5101650, 0x5102240,
    0x5102b48, 0x5104dd0, 0x51056b8, 0x5106ce0, 0x51075c8, 0x5108048,
    0x5108b68, 0x5109450, 0x5109e68, 0x510a910, 0x510b390, 0x510be58,
    0x510c9b0, 0x510d298, 0x510dcf0, 0x510e918, 0x510f408, 0x5110438,
    0x5111048, 0x5112638, 0x5112ee0, 0x5113e08, 0x51146b0, 0x5115030,
    0x51159f8, 0x5116ff8, 0x5117ef8, 0x51187a0, 0x511a7b0, 0x511bb28,
    0x511c3d0, 0x511d688, 0x511df30, 0x511ed18, 0x511f750, 0x5120b00,
    0x5123d10, 0x5124878, 0x5126038, 0x51268e0, 0x5127f00, 0x5128ac8,
    0x51293b0, 0x512a180, 0x512ae20, 0x512b708, 0x512c078, 0x512d420,
    0x512dd38, 0x512e5e0, 0x5130308, 0x51326d8, 0x5133080, 0x5134d28,
    0x5135670, 0x5139d90, 0x513b0f0, 0x513be98, 0x513c868, 0x513d190,
    0x513df38, 0x513fae8, 0x51403d0, 0x5140fe8, 0x5141890, 0x5142a58,
    0x5143d40, 0x5144f68, 0x5146528, 0x51471d8, 0x5147c88, 0x5148be8,
    0x5149538, 0x5149de0, 0x514afb0, 0x514b898, 0x514c1b0, 0x514ca58,
    0x514d300, 0x514ddc8, 0x514e760, 0x514f588, 0x514fe70, 0x5150d98,
    0x51516c0, 0x51520f8, 0x5153178, 0x5153a60, 0x5154bc8, 0x5156168,
    0x5156a90, 0x51586e0, 0x5158f88, 0x5159870, 0x515adc0, 0x515c1f8,
    0x515cc18, 0x515d6e0, 0x515eb08, 0x515f3b0, 0x5161b00, 0x51624a8,
    0x5166bd0, 0x51674b8, 0x5168a60, 0x5169490, 0x5169e58, 0x516a7c0,
    0x516b1f8, 0x516bcb0, 0x516ced8, 0x516d7c0, 0x516e5f8, 0x516f010,
    0x516fc60, 0x51711f0, 0x5173858, 0x5174d98, 0x5175680, 0x51762e8,
];

static DONOR_IDS: [u32; VANILLA_STAGE_IDS as usize] = identity_ids();
static MODULES: RwLock<Vec<(usize, usize, bool)>> = RwLock::new(Vec::new());
static ANSWER_LOG: AtomicU32 = AtomicU32::new(0);
static RANDOM_LOG: AtomicU32 = AtomicU32::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static GATE_LOGGED: AtomicBool = AtomicBool::new(false);

const fn identity_ids() -> [u32; VANILLA_STAGE_IDS as usize] {
    let mut ids = [0u32; VANILLA_STAGE_IDS as usize];
    let mut index = 0;
    while index < ids.len() {
        ids[index] = index as u32;
        index += 1;
    }
    ids
}

fn enabled() -> bool {
    crate::settings::get().stage_donor_tests
}

unsafe fn read<T: Copy>(address: usize) -> T {
    core::ptr::read_volatile(address as *const T)
}

unsafe fn defines_lua2cpp(base: usize, end: usize) -> bool {
    let size = end - base;
    let mod0 = base + read::<u32>(base + NRO_MOD0_AT) as usize;
    if mod0 + 8 > end || read::<u32>(mod0) != MOD0_MAGIC {
        return false;
    }
    let dynamic = (mod0 as isize + read::<i32>(mod0 + 4) as isize) as usize;
    if dynamic < base || dynamic >= end {
        return false;
    }
    let place = |value: u64| -> usize {
        if (value as usize) < size {
            base + value as usize
        } else {
            value as usize
        }
    };
    let (mut strtab, mut symtab, mut hash, mut strsz, mut syment) = (0usize, 0usize, 0usize, 0usize, SYMBOL_SIZE);
    for index in 0..MAX_DYNAMIC {
        let tag = read::<u64>(dynamic + index * 16);
        let value = read::<u64>(dynamic + index * 16 + 8);
        match tag {
            DT_NULL => break,
            DT_STRTAB => strtab = place(value),
            DT_SYMTAB => symtab = place(value),
            DT_HASH => hash = place(value),
            DT_STRSZ => strsz = value as usize,
            DT_SYMENT => syment = value as usize,
            _ => {}
        }
    }
    if strtab < base || strtab >= end || symtab < base || symtab >= end || syment < SYMBOL_SIZE {
        return false;
    }
    let count = if hash >= base && hash < end {
        read::<u32>(hash + 4) as usize
    } else if strtab > symtab {
        (strtab - symtab) / syment
    } else {
        0
    };
    for index in 0..count.min(MAX_SYMBOLS) {
        let symbol = symtab + index * syment;
        if symbol + SYMBOL_SIZE > end {
            break;
        }
        let name = read::<u32>(symbol) as usize;
        let section = read::<u16>(symbol + 6);
        if section == 0 || name + LUA2CPP_PREFIX.len() > strsz || strtab + name + LUA2CPP_PREFIX.len() > end {
            continue;
        }
        let bytes = core::slice::from_raw_parts((strtab + name) as *const u8, LUA2CPP_PREFIX.len());
        if bytes == LUA2CPP_PREFIX {
            return true;
        }
    }
    false
}

pub(crate) unsafe fn caller_is_fighter_script(lr: usize) -> bool {
    if let Ok(modules) = MODULES.read() {
        if let Some((_, _, fighter)) = modules.iter().find(|(base, end, _)| (*base..*end).contains(&lr)) {
            return *fighter;
        }
    }
    let (base, end) = crate::final_grafts::module_range(lr);
    if base == 0 || end <= base {
        return false;
    }
    let fighter = defines_lua2cpp(base, end);
    if let Ok(mut modules) = MODULES.write() {
        if modules.iter().all(|(known, _, _)| *known != base) {
            modules.push((base, end, fighter));
        }
    }
    fighter
}

unsafe fn answers_as_donor(lr: usize) -> bool {
    if crate::caller_is_outside_main_text(lr) {
        return caller_is_fighter_script(lr);
    }
    GETTER_RETURNS.binary_search(&crate::game::offset_of(lr)).is_ok()
}

#[inline(never)]
unsafe extern "C" fn stage_id(stage: usize) -> u32 {
    let lr: usize;
    core::arch::asm!("mov {}, x30", out(reg) lr);
    let id = read::<u32>(stage + STAGE_ID_FIELD);
    if id < VANILLA_STAGE_IDS {
        return id;
    }
    let Some(donor) = crate::stage_dispatch::donor_for(id) else {
        return id;
    };
    if !answers_as_donor(lr) {
        return id;
    }
    if ANSWER_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        let at = if crate::caller_is_outside_main_text(lr) { lr } else { crate::game::offset_of(lr) };
        crate::dbg_log_public(&format!(
            "[stagedonor] stage id {id} answers its donor {donor} to a per-stage test returning to {at:#x}"
        ));
    }
    donor
}

pub(crate) fn enemy_gate_offset(offset: u32) -> Option<u32> {
    if !INSTALLED.load(Ordering::Acquire) {
        return None;
    }
    let id = offset.wrapping_add(ENEMY_STAGE_FIRST);
    if id < VANILLA_STAGE_IDS {
        return None;
    }
    let donor = crate::stage_dispatch::donor_for(id)?;
    if !GATE_LOGGED.swap(true, Ordering::Relaxed) {
        crate::dbg_log_public(&format!(
            "[stagedonor] stage id {id} answers its donor {donor} to the enemy stage test"
        ));
    }
    Some(donor.wrapping_sub(ENEMY_STAGE_FIRST))
}

#[skyline::hook(offset = crate::game::rel(ENEMY_STAGE_SECOND), inline)]
unsafe fn enemy_stage_second(ctx: &mut skyline::hooks::InlineCtx) {
    let id = ctx.registers[ENEMY_STAGE_SECOND_REGISTER].w();
    if let Some(offset) = enemy_gate_offset(id.wrapping_sub(ENEMY_STAGE_FIRST)) {
        ctx.registers[ENEMY_STAGE_SECOND_REGISTER].set_x(offset.wrapping_add(ENEMY_STAGE_FIRST) as u64);
    }
}

#[cfg(feature = "item_clone_backend")]
fn enemy_gate_hooked() -> bool {
    crate::enemy_spawn::gate_installed()
}

#[cfg(not(feature = "item_clone_backend"))]
fn enemy_gate_hooked() -> bool {
    false
}

#[skyline::hook(offset = crate::game::rel(STAGE_RANDOM_SETTINGS))]
unsafe fn stage_random_settings(parameter: *mut u32, seed: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64) -> u64 {
    if parameter.is_null() {
        return call_original!(parameter, seed, a2, a3, a4, a5, a6, a7);
    }
    let id = core::ptr::read_volatile(parameter);
    crate::stage_hazards::apply(parameter.cast(), id);
    let donor = if id < VANILLA_STAGE_IDS || !enabled() { None } else { crate::stage_dispatch::donor_for(id) };
    if let Some(donor) = donor {
        core::ptr::write_volatile(parameter, donor);
        if RANDOM_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
            crate::dbg_log_public(&format!(
                "[stagedonor] stage id {id} draws its donor {donor}'s random stage settings"
            ));
        }
    }
    let result = call_original!(parameter, seed, a2, a3, a4, a5, a6, a7);
    if donor.is_some() {
        core::ptr::write_volatile(parameter, id);
    }
    result
}

macro_rules! stage_donor_sites {
    ($install:ident; $($name:ident($reg:expr, $offset:expr, $opcode:expr));* $(;)?) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let id = ctx.registers[$reg].w();
                if id < VANILLA_STAGE_IDS {
                    return;
                }
                let Some(donor) = crate::stage_dispatch::donor_for(id) else {
                    return;
                };
                ctx.registers[$reg].set_x(donor as u64);
                static LOGGED: AtomicBool = AtomicBool::new(false);
                if !LOGGED.swap(true, Ordering::Relaxed) {
                    crate::dbg_log_public(&format!(
                        "[stagedonor] stage id {id} answers its donor {donor} at {:#x} ({})",
                        $offset as usize,
                        stringify!($name)
                    ));
                }
            }
        )*

        fn $install() -> usize {
            let mut installed = 0usize;
            $(
                if unsafe { crate::game::text_word($offset) } == $opcode {
                    skyline::install_hooks!($name);
                    installed += 1;
                } else {
                    skyline::println!("[stagedonor] {:#x} ({}) holds another instruction; left alone", $offset as usize, stringify!($name));
                }
            )*
            installed
        }
    };
}

stage_donor_sites! {
    install_inline_sites;
    enemy_stage_first(8, 0x134b094, 0x51031508);
    final_battle(8, 0x14e8480, 0x7105091f);
    item_stages_a(8, 0x160b7b8, 0x7101d91f);
    item_stages_b(8, 0x160b8ec, 0x7101d91f);
    kof_stadium_camera(8, 0x4ffa4c, 0x7105491f);
    map_or_stage_builder(8, 0x14f69a8, 0x7104e51f);
    stage_builder_a(8, 0x135e608, 0x7105151f);
    stage_builder_d(8, 0x15a8488, 0x7105151f);
    home_run_contest(8, 0x1342a04, 0x7104ed1f);
    mario_maker_stage_load(8, 0x25fd688, 0x71047d1f);
}

macro_rules! stage_donor_loads {
    ($install:ident; $($name:ident($base:expr, $field:expr, $offset:expr, $opcode:expr));* $(;)?) => {
        $(
            #[skyline::hook(offset = crate::game::rel($offset), inline)]
            unsafe fn $name(ctx: &mut skyline::hooks::InlineCtx) {
                let base = ctx.registers[$base].x() as usize;
                if base == 0 {
                    return;
                }
                let id = read::<u32>(base + $field);
                if id < VANILLA_STAGE_IDS {
                    return;
                }
                let Some(donor) = crate::stage_dispatch::donor_for(id) else {
                    return;
                };
                let Some(cell) = DONOR_IDS.get(donor as usize) else {
                    return;
                };
                ctx.registers[$base].set_x((cell as *const u32 as usize - $field) as u64);
                static LOGGED: AtomicBool = AtomicBool::new(false);
                if !LOGGED.swap(true, Ordering::Relaxed) {
                    crate::dbg_log_public(&format!(
                        "[stagedonor] stage id {id} loads its donor {donor} at {:#x} ({})",
                        $offset as usize,
                        stringify!($name)
                    ));
                }
            }
        )*

        fn $install() -> usize {
            let mut installed = 0usize;
            $(
                if unsafe { crate::game::text_word($offset) } == $opcode {
                    skyline::install_hooks!($name);
                    installed += 1;
                } else {
                    skyline::println!("[stagedonor] {:#x} ({}) holds another instruction; left alone", $offset as usize, stringify!($name));
                }
            )*
            installed
        }
    };
}

stage_donor_loads! {
    install_load_sites;
    kof_stadium_camera_b(8, 0x450, 0x4ff690, 0xb9445108);
    spla_maker_camera(8, 0x450, 0x500f14, 0xb9445108);
    stage_builder_c(8, 0x450, 0x14e10b0, 0xb9445108);
    stage_builder_b(8, 0x5d0, 0x1342cfc, 0xb945d108);
}

fn install_random_settings() -> bool {
    let random = unsafe { crate::game::text_word(STAGE_RANDOM_SETTINGS) } == STAGE_RANDOM_SETTINGS_OPCODE;
    if random {
        skyline::install_hooks!(stage_random_settings);
    }
    random
}

pub(crate) fn install() {
    if !enabled() {
        install_random_settings();
        skyline::println!("[stagedonor] off: cloned stages answer their own id to per-stage tests (settings.toml stage_donor_tests = false)");
        return;
    }
    let getter = crate::game::addr(STAGE_ID_GETTER);
    let wrapper = (stage_id as usize).to_le_bytes();
    let mut replaced = 0usize;
    for vtable in STAGE_VTABLES {
        let slot = crate::game::addr(vtable);
        if unsafe { read::<usize>(slot) } != getter {
            continue;
        }
        if unsafe { crate::text_patch::write_bytes(slot, &wrapper) } {
            replaced += 1;
        }
    }
    let mut inline = install_inline_sites() + install_load_sites();
    INSTALLED.store(true, Ordering::Release);
    let gate = if enemy_gate_hooked() {
        "the enemy spawner's stage gate"
    } else if unsafe { crate::game::text_word(ENEMY_STAGE_SECOND) } == ENEMY_STAGE_SECOND_OPCODE {
        skyline::install_hooks!(enemy_stage_second);
        inline += 1;
        "its own hook"
    } else {
        "nothing (the site holds another instruction)"
    };
    skyline::println!("[stagedonor] the second enemy stage test goes through {gate}");
    let random = install_random_settings();
    skyline::println!(
        "[stagedonor] cloned stages answer their donor's id to per-stage tests: {replaced} of {} stage classes, {} getter call sites, {inline} inline sites, random stage settings {}, fighter scripts",
        STAGE_VTABLES.len(),
        GETTER_RETURNS.len(),
        if random { "on" } else { "off (the function holds another instruction)" }
    );
}
