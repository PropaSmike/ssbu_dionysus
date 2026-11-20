use core::ops::RangeInclusive;
use core::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crate::finalsmash_residency::{
    directory_index_for, directory_load_flags, fiber_note, resource_service,
};

const OFF_LOAD_DIRECTORY: usize = 0x35407a0;
const OFF_LOAD_CONTEXT_REGISTER: usize = 0x17e2e30;
const OFF_LOAD_CONTEXT_RELEASE: usize = 0x17e3020;
const OFF_RELEASE_KIND_GROUPS: usize = 0x17e8650;
const OFF_ENTRY_LOADER_PICK: usize = 0x6506f4;
const ENTRY_COUNT: usize = 8;
const LOADER_GROUP_COUNT: usize = 0x18;
const LOADER_MODULE_COUNT: usize = 0x4c0;
const LOADER_RESET: usize = 0x20;
const APPEND_OWNER_KIND: i32 = 82;
const CTX_MII_ENEMY_MODE: usize = 0x48;
const CTX_KIND: usize = 0x58;
const CTX_COLOR: usize = 0x64;
const CTX_NAME_FLAG: usize = 0x7d;
const KIND_KEY_STRIDE: u64 = 42;
const KIND_KEYS_RELEASED: [u64; 4] = [0, 1, 6, 7];
const MII_FIGHTER_KINDS: RangeInclusive<i32> = 0x48..=0x4a;
const MII_FIGHTER_KEYS: [u64; 3] = [2, 3, 4];
const MII_ENEMY_KINDS: RangeInclusive<i32> = 0x4e..=0x50;
const MII_ENEMY_MODE_WITH_KEY: u8 = 2;
const MII_ENEMY_KEY: u64 = 5;
const LOWEST_PLAUSIBLE_POINTER: usize = 0x1_0000;
const LOG_BUDGET: u32 = 8;

static LOG: AtomicU32 = AtomicU32::new(0);
static RELEASE_LOG: AtomicU32 = AtomicU32::new(0);
static REQUESTED: Mutex<Vec<(usize, u64)>> = Mutex::new(Vec::new());

#[skyline::from_offset(crate::game::rel(OFF_LOAD_DIRECTORY))]
unsafe fn load_directory(service: u64, directory: u32) -> usize;

#[skyline::from_offset(crate::game::rel(OFF_LOAD_CONTEXT_REGISTER))]
unsafe fn register_directory(context: u64, key: u64, directory: *const u32);

#[skyline::from_offset(crate::game::rel(OFF_LOAD_CONTEXT_RELEASE))]
unsafe fn release_directory(context: u64, key: u64);

unsafe fn directory_at(service: usize, path: &str) -> Option<(u32, u64)> {
    let hash = crate::hash40::hash40(path);
    directory_index_for(service, hash).map(|index| (index, hash))
}

unsafe fn directory_named(service: usize, name: &str, color: u32) -> Option<(u32, u64)> {
    directory_at(service, &format!("fighter/{name}/append/c{color:02}"))
}

unsafe fn request(service: usize, context: usize, directory: Option<(u32, u64)>) -> u64 {
    let Some((directory, hash)) = directory else {
        return 0xdead;
    };
    load_directory(service as u64, directory);
    let mut flags = directory_load_flags(service, directory);
    if flags & 1 == 0 {
        load_directory(service as u64, directory);
        flags = directory_load_flags(service, directory);
    }
    let index: u32 = directory;
    register_directory(context as u64, hash, &index);
    if let Ok(mut requested) = REQUESTED.lock() {
        requested.push((context, hash));
    }
    flags
}

fn take_requested(context: usize) -> Vec<u64> {
    let Ok(mut requested) = REQUESTED.lock() else {
        return Vec::new();
    };
    let mut taken = Vec::new();
    requested.retain(|&(owner, hash)| {
        if owner == context {
            taken.push(hash);
        }
        owner != context
    });
    taken
}

unsafe fn released_key_offsets(context: usize, kind: i32) -> Vec<u64> {
    let mut offsets = KIND_KEYS_RELEASED.to_vec();
    if MII_FIGHTER_KINDS.contains(&kind) {
        offsets.extend(MII_FIGHTER_KEYS);
    } else if MII_ENEMY_KINDS.contains(&kind)
        && core::ptr::read_volatile((context + CTX_MII_ENEMY_MODE) as *const u8)
            == MII_ENEMY_MODE_WITH_KEY
    {
        offsets.push(MII_ENEMY_KEY);
    }
    offsets
}

unsafe fn release_clone_groups(context: usize, kind: i32) {
    let own = core::ptr::read_volatile((context + CTX_KIND) as *const i32);
    let Some(definition) = crate::clone_definition(own) else {
        return;
    };
    if kind != own && kind != definition.base_kind {
        return;
    }
    let mut kind_keys = 0u64;
    if kind != own {
        let first = own as u64 * KIND_KEY_STRIDE;
        for offset in released_key_offsets(context, kind) {
            release_directory(context as u64, first + offset);
            kind_keys += 1;
        }
    }
    let requested = take_requested(context);
    for hash in &requested {
        release_directory(context as u64, *hash);
    }
    if RELEASE_LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        fiber_note(
            b"[groups]",
            &[
                (b"context", context as u64),
                (b"kind", own as u32 as u64),
                (b"released_as", kind as u32 as u64),
                (b"kind_keys", kind_keys),
                (b"engine_keys", requested.len() as u64),
            ],
        );
    }
}

#[skyline::hook(offset = crate::game::rel(OFF_RELEASE_KIND_GROUPS))]
unsafe fn release_kind_groups(context: usize, kind: i32) {
    if context >= LOWEST_PLAUSIBLE_POINTER {
        match crate::entry_groups::release(context, kind) {
            crate::entry_groups::Release::Deferred => {
                fiber_note(b"[partner] first of the pair loaded; its files stay held for the other", &[(b"kind", kind as u32 as u64)]);
                return;
            }
            crate::entry_groups::Release::EarlierFirst(earlier) => {
                fiber_note(b"[partner] second of the pair loaded; releasing both", &[(b"kind", kind as u32 as u64)]);
                release_engine_groups(context, earlier);
                call_original!(context, earlier);
            }
            crate::entry_groups::Release::Normal => {}
        }
        release_engine_groups(context, kind);
    }
    call_original!(context, kind);
}

unsafe fn release_engine_groups(context: usize, kind: i32) {
    if let Some(member) = crate::entry_groups::member_for_release(context, kind) {
        let first = member as u64 * KIND_KEY_STRIDE;
        for offset in KIND_KEYS_RELEASED {
            release_directory(context as u64, first + offset);
        }
        fiber_note(b"[switch] member loaded; its groups released", &[(b"kind", member as u32 as u64)]);
    }
    release_clone_groups(context, kind)
}

#[skyline::hook(offset = crate::game::rel(OFF_ENTRY_LOADER_PICK), inline)]
unsafe fn entry_loader_pick(ctx: &mut skyline::hooks::InlineCtx) {
    let manager = ctx.registers[26].x() as usize;
    let entry = ctx.registers[22].x() as usize;
    if manager < LOWEST_PLAUSIBLE_POINTER || entry >= ENTRY_COUNT {
        return;
    }
    let loader = core::ptr::read_volatile((manager + entry * 8) as *const usize);
    if loader < LOWEST_PLAUSIBLE_POINTER
        || core::ptr::read_volatile((loader + LOADER_MODULE_COUNT) as *const u64) != 0
        || core::ptr::read_volatile((loader + LOADER_GROUP_COUNT) as *const u64) == 0
    {
        return;
    }
    let own = core::ptr::read_volatile((loader + CTX_KIND) as *const i32);
    let Some(owner) = crate::entry_groups::loader_owner(own) else {
        return;
    };
    let first = owner as u64 * KIND_KEY_STRIDE;
    for offset in KIND_KEYS_RELEASED {
        release_directory(loader as u64, first + offset);
    }
    let emptied = core::ptr::read_volatile((loader + LOADER_GROUP_COUNT) as *const u64) == 0;
    if emptied {
        let vtable = core::ptr::read_volatile(loader as *const usize);
        let reset: extern "C" fn(usize) = core::mem::transmute(core::ptr::read_volatile((vtable + LOADER_RESET) as *const usize));
        reset(loader);
    }
    fiber_note(
        b"[switch] the group's fighter is gone; its shared owner's groups released so the loader is free again",
        &[(b"kind", own as u32 as u64), (b"owner", owner as u32 as u64), (b"reset", emptied as u64)],
    );
}

pub(crate) unsafe fn request_append_group(context: usize, kind: i32) {
    if context < LOWEST_PLAUSIBLE_POINTER
        || core::ptr::read_volatile((context + CTX_NAME_FLAG) as *const u8) != 0
    {
        return;
    }
    let Some(definition) = crate::clone_definition(kind) else {
        return;
    };
    let service = resource_service();
    if service < LOWEST_PLAUSIBLE_POINTER {
        return;
    }
    let color = core::ptr::read_volatile((context + CTX_COLOR) as *const u32);
    let own_movie = directory_at(
        service,
        &format!("fighter/{}/movie/c{color:02}", definition.resource_name),
    );
    let own_movie_flags = request(service, context, own_movie);
    let movie = directory_at(
        service,
        &format!("fighter/{}/movie/c{color:02}", definition.base_resource_name),
    );
    let movie_flags = request(service, context, movie);
    let own = directory_named(service, definition.resource_name, color);
    let own_flags = request(service, context, own);
    if let Some(name) = crate::cpu_ai::chosen(kind)
        .and_then(crate::custom_articles::fighter_name)
        .and_then(|name| name.to_str().ok())
    {
        let ai_group = directory_at(service, &format!("fighter/{name}/cmn/ai"));
        let ai_flags = request(service, context, ai_group);
        fiber_note(
            b"[cpuai] AI group requested with the fighter",
            &[
                (b"kind", kind as u32 as u64),
                (b"group", ai_group.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"flags", ai_flags),
            ],
        );
    }
    if definition.base_kind != APPEND_OWNER_KIND {
        if let Some((index, _)) = own {
            if LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
                fiber_note(
                    b"[append]",
                    &[
                        (b"kind", kind as u32 as u64),
                        (b"color", color as u64),
                        (b"own", u64::from(index)),
                        (b"own_flags", own_flags),
                    ],
                );
            }
        }
        if (own_movie.is_some() || movie.is_some())
            && LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET
        {
            fiber_note(
                b"[movie]",
                &[
                    (b"kind", kind as u32 as u64),
                    (b"color", color as u64),
                    (b"own", own_movie.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                    (b"own_flags", own_movie_flags),
                    (b"group", movie.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                    (b"group_flags", movie_flags),
                ],
            );
        }
        return;
    }
    let base = directory_named(service, definition.base_resource_name, color);
    let base_flags = request(service, context, base);
    let prebuilt = directory_at(
        service,
        &format!("prebuilt:/movie/fighter/{}/c{color:02}", definition.base_resource_name),
    );
    let prebuilt_flags = request(service, context, prebuilt);
    if LOG.fetch_add(1, Ordering::Relaxed) < LOG_BUDGET {
        fiber_note(
            b"[append]",
            &[
                (b"kind", kind as u32 as u64),
                (b"color", color as u64),
                (b"own", own.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"own_flags", own_flags),
                (b"base", base.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"base_flags", base_flags),
                (b"movie", movie.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"movie_flags", movie_flags),
                (b"prebuilt", prebuilt.map(|(index, _)| u64::from(index)).unwrap_or(u64::MAX)),
                (b"prebuilt_flags", prebuilt_flags),
            ],
        );
    }
}

pub(crate) fn install() {
    skyline::install_hooks!(release_kind_groups, entry_loader_pick);
    crate::dbg_log_public("[append] armed: every clone's own append and movie groups, its base's movie group and a Joker clone's base append group are requested from the load pass; they and the groups loaded under the clone's own kind are released when the game releases the fighter under its base kind");
}
