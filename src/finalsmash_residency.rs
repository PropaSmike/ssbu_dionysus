use core::sync::atomic::Ordering;

const OFF_FINALSMASH_LOAD_GATE: usize = 0x60d490;
const OFF_FINALSMASH_PATH_READY: usize = 0x60d39c;
const OFF_SCENE_GATE_AHEAD: usize = 0x60d25c;
const SCENE_GATE_RETURN: usize = 0x60d28c;
const PATH_BUFFER: usize = 0x20;
const PATH_BUFFER_BYTES: usize = 0x800;
const OBJECT_REGISTER: usize = 19;
const KIND_REGISTER: usize = 21;
const FIRST_BYTE_REGISTER: usize = 11;
const OFF_START_FINALBG: usize = 0x21414d0;
const START_FINALBG_WORD: u32 = 0xD100_C3FF;
const REGISTERED_SLOTS: usize = 8;
const OFF_LOAD_DIRECTORY: usize = 0x35407a0;
const OFF_PRELOAD_FINALSMASH_LOOKUP: usize = 0x17e760c;
const PRELOAD_LOOKUP_RETURN: usize = 0x17e7680;
const DIRECTORY_NOT_FOUND: u32 = 0xffffff;
const RESOURCE_SERVICE: usize = 0x5331f20;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const SEARCH_KEY_MASK: u64 = 0xffffffffff;
const MAX_MODEL_PROBES: usize = 4;
const NO_MODEL_TO_LOAD: u64 = u64::MAX;

pub(crate) static SUPPRESSED_OWNER: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);
static FINALSMASH_DIRECTORY: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);
static SKIP_FINALSMASH_LOAD: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

static PROBE_INDICES: [core::sync::atomic::AtomicI32; MAX_MODEL_PROBES] =
    [const { core::sync::atomic::AtomicI32::new(-1) }; MAX_MODEL_PROBES];
static PROBE_COUNT: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(-1);
static RESOLVED_OWNER: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(-1);

static PRELOAD_KIND: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(-1);
static SUBSTITUTE_DIRECTORY: core::sync::atomic::AtomicI32 =
    core::sync::atomic::AtomicI32::new(-1);
static SUBSTITUTE_KIND: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(-1);
static SUBSTITUTE_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
const SUBSTITUTE_LOG_BUDGET: u32 = 4;
static GATE_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
const GATE_LOG_BUDGET: u32 = 8;
static REDIRECTED_OBJECT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
static CONSTRUCTING_OBJECT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
static CONSTRUCTING_KIND: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(-1);
static REDIRECT_LOG: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
const REDIRECT_LOG_BUDGET: u32 = 16;
static REGISTERED: [core::sync::atomic::AtomicU64; REGISTERED_SLOTS] =
    [const { core::sync::atomic::AtomicU64::new(u64::MAX) }; REGISTERED_SLOTS];

fn remember_registration(clone_kind: i32, registered: i32) {
    let packed = ((clone_kind as u32 as u64) << 32) | registered as u32 as u64;
    for slot in REGISTERED.iter() {
        let held = slot.load(Ordering::Acquire);
        if held == u64::MAX || (held >> 32) as u32 as i32 == clone_kind {
            slot.store(packed, Ordering::Release);
            return;
        }
    }
}

fn registered_kind(clone_kind: i32) -> Option<i32> {
    REGISTERED
        .iter()
        .map(|slot| slot.load(Ordering::Acquire))
        .find(|held| *held != u64::MAX && (held >> 32) as u32 as i32 == clone_kind)
        .map(|held| held as u32 as i32)
}

#[skyline::hook(offset = crate::game::rel(OFF_START_FINALBG))]
unsafe fn start_finalbg_hook(manager: u64, kind: i32, rate: f32) -> u64 {
    let kind = registered_kind(kind).unwrap_or(kind);
    call_original!(manager, kind, rate)
}

static FINALSMASH_OWNER_KINDS: [i32; 27] = [
    5, 7, 11, 20, 33, 42, 47, 49, 57, 59, 63, 64, 66, 67, 68, 69, 71, 82, 83, 84, 85, 86, 87, 88,
    89, 92, 93,
];

static FINALSMASH_MODEL_PROBES: [(i32, &[&str]); 26] = [
    (5, &["shared/model/bg_set/model.numdlb", "shared/model/far_ground_set/model.numdlb"]),
    (7, &["shared/model/bg_set/model.numdlb", "shared/render/lensflare/lensflare_0/model.numdlb"]),
    (
        11,
        &[
            "shared/model/stc_bg_and_road_set/model.numdlb",
            "shared/model/stc_fs_captain_sky_set/model.numdlb",
        ],
    ),
    (
        20,
        &[
            "shared/model/bg_set/model.numdlb",
            "shared/render/lensflare/lensflare_0/model.numdlb",
        ],
    ),
    (33, &["shared/model/wario_final_bg_set/model.numdlb"]),
    (
        42,
        &[
            "shared/model/fs_dedede_ground_far_set/model.numdlb",
            "shared/model/fs_dedede_ground_set/model.numdlb",
        ],
    ),
    (
        47,
        &[
            "shared/model/bg_set/model.numdlb",
            "shared/render/lensflare/lensflare_0/model.numdlb",
        ],
    ),
    (49, &["shared/model/bg_all_set/model.numdlb", "shared/model/sky_set/model.numdlb"]),
    (57, &["shared/model/ground_set/model.numdlb", "shared/model/sky_set/model.numdlb"]),
    (59, &["shared/model/bg_set/model.numdlb", "shared/model/sky_set/model.numdlb"]),
    (63, &["shared/model/sky_set/model.numdlb", "shared/model/stc_bg_set/model.numdlb"]),
    (64, &["shared/model/bg_set/model.numdlb"]),
    (66, &["shared/model/fs_ridley_bg_set/model.numdlb"]),
    (
        67,
        &[
            "shared/model/fs_simon_bg_set/model.numdlb",
            "shared/model/fs_simon_moon_set/model.numdlb",
            "shared/model/fs_simon_sky_set/model.numdlb",
        ],
    ),
    (
        68,
        &[
            "shared/model/fs_simon_bg_set/model.numdlb",
            "shared/model/fs_simon_moon_set/model.numdlb",
            "shared/model/fs_simon_sky_set/model.numdlb",
        ],
    ),
    (
        69,
        &[
            "shared/model/fs_dkisland_bg_set/model.numdlb",
            "shared/model/fs_krool_inn_bg_set/model.numdlb",
            "shared/model/fs_krool_out_bg_set/model.numdlb",
            "shared/model/fs_kroolisland_bg_set/model.numdlb",
        ],
    ),
    (
        71,
        &[
            "shared/model/fs_gaogaen_bg_far_set/model.numdlb",
            "shared/model/fs_gaogaen_bg_set/model.numdlb",
            "shared/model/fs_gaogaen_parts_light_set/model.numdlb",
        ],
    ),
    (82, &["shared/model/bg_set/model.numdlb"]),
    (83, &["shared/model/bg_set/model.numdlb"]),
    (
        84,
        &[
            "shared/model/fs_buddy_bg_set/model.numdlb",
            "shared/model/main_ring_set/model.numdlb",
            "shared/model/statue_break_set/model.numdlb",
        ],
    ),
    (85, &["shared/model/fs_dolly_bg_set/model.numdlb"]),
    (
        86,
        &[
            "shared/model/bg_set/model.numdlb",
            "shared/model/fs_master_bg_set/model.numdlb",
            "shared/model/fs_master_screen_set/model.numdlb",
        ],
    ),
    (88, &["shared/model/bg_set/model.numdlb"]),
    (89, &["shared/model/bg_set/model.numdlb"]),
    (
        92,
        &[
            "shared/model/fs_demon_bg_set/model.numdlb",
            "shared/model/fs_demon_sky_set/model.numdlb",
        ],
    ),
    (93, &["shared/model/gate_set/model.numdlb", "shared/model/sky_set/model.numdlb"]),
];

fn owner_has_finalsmash(kind: i32) -> bool {
    FINALSMASH_OWNER_KINDS.binary_search(&kind).is_ok()
}

fn model_probes_for(kind: i32) -> &'static [&'static str] {
    FINALSMASH_MODEL_PROBES
        .iter()
        .find(|(owner, _)| *owner == kind)
        .map(|(_, paths)| *paths)
        .unwrap_or(&[])
}

pub(crate) unsafe fn resource_service() -> usize {
    core::ptr::read_volatile((crate::game::addr(RESOURCE_SERVICE)) as *const usize)
}

pub(crate) unsafe fn directory_index_for(service: usize, hash: u64) -> Option<u32> {
    let chain = core::ptr::read_volatile((service + 0x78) as *const usize);
    if chain < LOWEST_PLAUSIBLE_POINTER {
        return None;
    }
    let node = core::ptr::read_volatile(chain as *const usize);
    if node < LOWEST_PLAUSIBLE_POINTER {
        return None;
    }
    let table = core::ptr::read_volatile((node + 0x70) as *const usize);
    let header = core::ptr::read_volatile((node + 0x40) as *const usize);
    if table < LOWEST_PLAUSIBLE_POINTER || header < LOWEST_PLAUSIBLE_POINTER {
        return None;
    }
    let count = core::ptr::read_volatile((header + 0xc) as *const u32) as usize;

    let mut low = 0usize;
    let mut len = count;
    while len > 0 {
        let half = len / 2;
        let probe = core::ptr::read_volatile((table + (low + half) * 8) as *const u64);
        if (probe & SEARCH_KEY_MASK) < hash {
            low += half + 1;
            len -= half + 1;
        } else {
            len = half;
        }
    }
    if low >= count {
        return None;
    }
    let entry = core::ptr::read_volatile((table + low * 8) as *const u64);
    ((entry & SEARCH_KEY_MASK) == hash).then(|| (entry >> 0x28) as u32)
}

pub(crate) unsafe fn directory_load_flags(service: usize, directory: u32) -> u64 {
    if service < LOWEST_PLAUSIBLE_POINTER {
        return 0xdead;
    }
    let count = core::ptr::read_volatile((service + 0x48) as *const u32);
    let table = core::ptr::read_volatile((service + 0x40) as *const usize);
    if table < LOWEST_PLAUSIBLE_POINTER || directory >= count {
        return 0xdead;
    }
    core::ptr::read_volatile((table + directory as usize * 0x48 + 8) as *const u8) as u64
}

unsafe fn resolve_finalsmash(owner: i32) {
    if RESOLVED_OWNER.swap(owner, Ordering::SeqCst) == owner {
        return;
    }
    let Some(name) = crate::custom_articles::fighter_name(owner).and_then(|n| n.to_str().ok())
    else {
        return;
    };
    let service = resource_service();
    if service < LOWEST_PLAUSIBLE_POINTER {
        return;
    }

    let probes = model_probes_for(owner);
    let mut stored = 0usize;
    let mut missing = 0usize;
    for relative in probes.iter().take(MAX_MODEL_PROBES) {
        let path = format!("finalsmash/{name}/{relative}");
        match crate::item_params::scan_file_path_index(crate::hash40::hash40(&path)) {
            Some(index) => {
                PROBE_INDICES[stored].store(index as i32, Ordering::SeqCst);
                stored += 1;
            }
            None => missing += 1,
        }
    }
    PROBE_COUNT.store(stored as i32, Ordering::SeqCst);

    let directory_path = format!("fighter/{name}/finalsmash/shared");
    let directory = directory_index_for(service, crate::hash40::hash40(&directory_path));
    FINALSMASH_DIRECTORY.store(
        directory.map(|index| index as i32).unwrap_or(-1),
        Ordering::SeqCst,
    );

    crate::dbg_log_public(&format!(
        "[fsload] kind {owner} ({name}) finalsmash directory {:?}, {stored} of {} model probes \
         resolved, {missing} missing. The gate at 0x60d490 lets the load run only once every \
         resolved probe holds real bytes, and a kind with no model at all is never gated",
        directory,
        probes.len()
    ));
}

pub(crate) fn refresh_suppression() {
    let mut watched = -1;
    let mut found = -1;
    for slot in crate::css_registration::CSS_CUSTOM_ENTRY_KINDS.iter() {
        let kind = slot.load(Ordering::SeqCst);
        if kind < 0 {
            continue;
        }
        let Some(definition) = crate::clone_definition(kind) else {
            continue;
        };
        if !owner_has_finalsmash(definition.base_kind) {
            continue;
        }
        watched = definition.base_kind;
        if !crate::css_registration::vanilla_kind_in_match(definition.base_kind) {
            found = definition.base_kind;
        }
    }
    SUPPRESSED_OWNER.store(found, Ordering::SeqCst);

    if watched >= 0 {
        unsafe { resolve_finalsmash(watched) };
    }
}

unsafe fn put(out: &mut [u8], mut at: usize, text: &[u8]) -> usize {
    for byte in text {
        if at < out.len() {
            out[at] = *byte;
            at += 1;
        }
    }
    at
}

unsafe fn put_hex(out: &mut [u8], mut at: usize, value: u64) -> usize {
    at = put(out, at, b"0x");
    let mut seen = false;
    for shift in (0..16).rev() {
        let nibble = ((value >> (shift * 4)) & 0xf) as u8;
        if nibble != 0 || seen || shift == 0 {
            seen = true;
            let ch = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            };
            at = put(out, at, &[ch]);
        }
    }
    at
}

pub(crate) unsafe fn fiber_note(tag: &[u8], values: &[(&[u8], u64)]) {
    let mut buf = [0u8; 224];
    let mut at = put(&mut buf, 0, tag);
    for (name, value) in values {
        at = put(&mut buf, at, b" ");
        at = put(&mut buf, at, name);
        at = put(&mut buf, at, b"=");
        at = put_hex(&mut buf, at, *value);
    }
    crate::dbg_out_public(core::str::from_utf8_unchecked(&buf[..at]));
}

unsafe fn file_payload(service: usize, index: u32) -> u64 {
    if service < LOWEST_PLAUSIBLE_POINTER {
        return 0;
    }
    let count = core::ptr::read_volatile((service + 0x18) as *const u32);
    let table = core::ptr::read_volatile((service + 0x08) as *const usize);
    let pool = core::ptr::read_volatile((service + 0x10) as *const usize);
    if table < LOWEST_PLAUSIBLE_POINTER || pool < LOWEST_PLAUSIBLE_POINTER || index >= count {
        return 0;
    }
    let slot = table + index as usize * 8;
    if core::ptr::read_volatile((slot + 4) as *const u8) == 0 {
        return 0;
    }
    let entry = core::ptr::read_volatile(slot as *const u32);
    let capacity = core::ptr::read_volatile((service + 0x1c) as *const u32);
    if entry >= capacity {
        return 0;
    }
    let record = pool + entry as usize * 0x18;
    if core::ptr::read_volatile((record + 0xc) as *const u8) == 0 {
        return 0;
    }
    core::ptr::read_volatile(record as *const u64)
}

unsafe fn finalsmash_ready() -> Option<u64> {
    let count = PROBE_COUNT.load(Ordering::Relaxed);
    if count < 0 {
        return None;
    }
    if count == 0 {
        return Some(NO_MODEL_TO_LOAD);
    }
    let service = resource_service();
    let mut first = 0u64;
    for slot in PROBE_INDICES.iter().take(count as usize) {
        let index = slot.load(Ordering::Relaxed);
        if index < 0 {
            return None;
        }
        let payload = file_payload(service, index as u32);
        if payload == 0 {
            return None;
        }
        if first == 0 {
            first = payload;
        }
    }
    Some(first)
}

fn tree_path(name: &str) -> String {
    format!("fighter/{name}/finalsmash/shared")
}

unsafe fn tree_exists(service: usize, name: &str) -> bool {
    service >= LOWEST_PLAUSIBLE_POINTER
        && directory_index_for(service, crate::hash40::hash40(&tree_path(name))).is_some()
}

unsafe fn redirected_tree(clone_kind: i32) -> Option<String> {
    let definition = crate::clone_definition(clone_kind)?;
    let service = resource_service();
    tree_exists(service, definition.resource_name).then(|| tree_path(definition.resource_name))
}

pub(crate) unsafe fn scene_gate_answer(kind: i32, return_address: usize) -> Option<bool> {
    let clone_kind = if return_address == crate::game::addr(SCENE_GATE_RETURN) {
        let object = CONSTRUCTING_OBJECT.swap(0, Ordering::SeqCst);
        if CONSTRUCTING_KIND.swap(-1, Ordering::SeqCst) != kind {
            return None;
        }
        crate::clone_kind_of_object(object as u64)?
    } else {
        crate::clone_definition(kind)?;
        kind
    };
    redirected_tree(clone_kind).map(|_| true)
}

#[skyline::hook(offset = crate::game::rel(OFF_SCENE_GATE_AHEAD), inline)]
unsafe fn scene_gate_ahead(ctx: &mut skyline::hooks::InlineCtx) {
    CONSTRUCTING_OBJECT.store(ctx.registers[OBJECT_REGISTER].x() as usize, Ordering::SeqCst);
    CONSTRUCTING_KIND.store(ctx.registers[KIND_REGISTER].x() as i32, Ordering::SeqCst);
}

pub(crate) fn forget_constructing_object() {
    CONSTRUCTING_OBJECT.store(0, Ordering::SeqCst);
    CONSTRUCTING_KIND.store(-1, Ordering::SeqCst);
}

#[skyline::hook(offset = crate::game::rel(OFF_FINALSMASH_PATH_READY), inline)]
unsafe fn finalsmash_path_ready(ctx: &mut skyline::hooks::InlineCtx) {
    let object = ctx.registers[OBJECT_REGISTER].x();
    let Some(clone_kind) = crate::clone_kind_of_object(object) else {
        return;
    };
    let Some(path) = redirected_tree(clone_kind) else {
        return;
    };
    let buffer = (ctx.sp.x() as usize + PATH_BUFFER) as *mut u8;
    let prefix = b"fighter/";
    if (0..prefix.len()).any(|at| core::ptr::read_volatile(buffer.add(at)) != prefix[at])
        || path.len() + 1 >= PATH_BUFFER_BYTES
    {
        if REDIRECT_LOG.fetch_add(1, Ordering::Relaxed) < REDIRECT_LOG_BUDGET {
            crate::dbg_log_public(&format!(
                "[fsload] clone kind {clone_kind}: the finalsmash path buffer did not read as fighter/..; {path} left unused"
            ));
        }
        return;
    }
    for (at, byte) in path.bytes().chain(core::iter::once(0)).enumerate() {
        core::ptr::write_volatile(buffer.add(at), byte);
    }
    ctx.registers[FIRST_BYTE_REGISTER].set_x(path.as_bytes()[0] as u64);
    let registered = ctx.registers[KIND_REGISTER].x() as u32 as i32;
    REDIRECTED_OBJECT.store(object as usize, Ordering::SeqCst);
    remember_registration(clone_kind, registered);
    if REDIRECT_LOG.fetch_add(1, Ordering::Relaxed) < REDIRECT_LOG_BUDGET {
        crate::dbg_log_public(&format!(
            "[fsload] clone kind {clone_kind} builds its own Final Smash background from {path}, started by start_finalbg with kind {registered}"
        ));
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_FINALSMASH_LOAD_GATE), inline)]
unsafe fn finalsmash_load_gate(ctx: &mut skyline::hooks::InlineCtx) {
    if REDIRECTED_OBJECT.swap(0, Ordering::SeqCst) != 0 {
        return;
    }
    let owner = ctx.registers[21].x() as u32 as i32;
    if SUPPRESSED_OWNER.load(Ordering::Relaxed) != owner {
        return;
    }

    let ready = if SKIP_FINALSMASH_LOAD.load(Ordering::Relaxed) {
        None
    } else {
        finalsmash_ready()
    };
    let logging = GATE_LOG.fetch_add(1, Ordering::Relaxed) < GATE_LOG_BUDGET;

    if let Some(payload) = ready {
        if logging {
            fiber_note(
                b"[fsgate] resident",
                &[
                    (b"kind", owner as u64),
                    (b"probes", PROBE_COUNT.load(Ordering::Relaxed) as u32 as u64),
                    (b"data", payload),
                ],
            );
        }
        return;
    }

    let skip = ctx.registers[23].x();
    ctx.registers[27].set_x(skip);
    if logging {
        fiber_note(
            b"[fsgate] skip",
            &[
                (b"kind", owner as u64),
                (b"probes", PROBE_COUNT.load(Ordering::Relaxed) as u32 as u64),
            ],
        );
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_PRELOAD_FINALSMASH_LOOKUP), inline)]
unsafe fn finalsmash_preload_kind(ctx: &mut skyline::hooks::InlineCtx) {
    PRELOAD_KIND.store(ctx.registers[19].x() as u32 as i32, Ordering::Relaxed);
}

unsafe fn substitute_directory_for_preload() -> Option<u32> {
    let kind = PRELOAD_KIND.load(Ordering::Relaxed);
    if SUBSTITUTE_KIND.load(Ordering::Relaxed) == kind {
        let cached = SUBSTITUTE_DIRECTORY.load(Ordering::Relaxed);
        if cached == -2 {
            return None;
        }
        if cached >= 0 {
            return Some(cached as u32);
        }
    }
    SUBSTITUTE_KIND.store(kind, Ordering::Relaxed);
    SUBSTITUTE_DIRECTORY.store(-1, Ordering::Relaxed);

    let Some(definition) = crate::clone_definition(kind) else {
        return None;
    };
    if !owner_has_finalsmash(definition.base_kind) {
        return None;
    }
    let bytes = definition.base_resource_name_cstr;
    let Ok(owner_name) = core::str::from_utf8(bytes.split(|byte| *byte == 0).next().unwrap_or(&[])) else {
        return None;
    };
    let owner = definition.base_kind;

    let service = resource_service();
    let resolved = (service >= LOWEST_PLAUSIBLE_POINTER)
        .then(|| directory_index_for(service, crate::hash40::hash40(&tree_path(owner_name))))
        .flatten();

    match resolved {
        Some(directory) => {
            SUBSTITUTE_DIRECTORY.store(directory as i32, Ordering::Relaxed);
            crate::dbg_log_public(&format!(
                "[fsload] clone kind {kind} asked the loading screen preload for its own \
                 finalsmash tree and the arc has none, so kind {owner} ({owner_name}) directory \
                 {directory:#x} is loaded instead, in the phase where the async read still lands"
            ));
            Some(directory)
        }
        None => {
            SUBSTITUTE_DIRECTORY.store(-2, Ordering::Relaxed);
            None
        }
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_LOAD_DIRECTORY))]
unsafe fn load_directory_hook(service: u64, directory: u32) -> usize {
    let caller: u64;
    core::arch::asm!("mov {}, x30", out(reg) caller, options(nomem, nostack));

    let mut directory = directory;
    if directory == DIRECTORY_NOT_FOUND
        && caller == (crate::game::addr(PRELOAD_LOOKUP_RETURN)) as u64
    {
        if let Some(substitute) = substitute_directory_for_preload() {
            if SUBSTITUTE_LOG.fetch_add(1, Ordering::Relaxed) < SUBSTITUTE_LOG_BUDGET {
                fiber_note(
                    b"[fsdir] substituted",
                    &[
                        (b"kind", PRELOAD_KIND.load(Ordering::Relaxed) as u32 as u64),
                        (b"dir", substitute as u64),
                        (
                            b"already_loaded",
                            directory_load_flags(service as usize, substitute),
                        ),
                    ],
                );
            }
            directory = substitute;
        }
    }

    call_original!(service, directory)
}

pub(crate) fn install() {
    skyline::install_hooks!(
        scene_gate_ahead,
        finalsmash_path_ready,
        finalsmash_load_gate,
        finalsmash_preload_kind,
        load_directory_hook
    );
    if unsafe { crate::game::text_word(OFF_START_FINALBG) } == START_FINALBG_WORD {
        skyline::install_hooks!(start_finalbg_hook);
    }
    crate::dbg_log_public(
        "[fsload] armed. The loading screen preload at 0x17e74a4 indexes LOWERCASE_FIGHTER_NAMES \
         under a 0x75 bound, so a minted clone kind falls through to the literal name none and \
         asks for a directory the arc does not have. Nothing is queued in the only phase where \
         the async reader services work, and the model build at 0x35c22e0 later dereferences a \
         registered but empty record. The base directory is substituted at that one call site, \
         and 0x60d490 falls back to the game own skip while any model probe is still empty.",
    );
}
