use dionysus_core::motion_append::{apply_global, MotionRecord, Outcome, Rejected};
use dionysus_core::motion_list::{parse, serialize};
use std::sync::{OnceLock, RwLock};

pub(crate) const MAX_MOTIONS: usize = 128;
pub(crate) const LAST_COSTUME: u32 = 255;

fn records() -> &'static RwLock<Vec<MotionRecord>> {
    static RECORDS: OnceLock<RwLock<Vec<MotionRecord>>> = OnceLock::new();
    RECORDS.get_or_init(|| RwLock::new(Vec::new()))
}

pub(crate) fn registered() -> Vec<MotionRecord> {
    records().read().map(|held| held.clone()).unwrap_or_default()
}

pub(crate) fn record(entry: MotionRecord) -> Result<(), &'static str> {
    let Ok(mut held) = records().write() else {
        return Err("registry poisoned");
    };
    if held.len() >= MAX_MOTIONS {
        return Err("global motion capacity reached");
    }
    if held.iter().any(|known| known.name == entry.name) {
        return Err("a global motion with that name is already registered");
    }
    held.push(entry);
    Ok(())
}

pub(crate) fn rebuild_with(
    original: &[u8],
    records: &[MotionRecord],
    has_animation: impl Fn(&str) -> bool,
) -> Option<(Vec<u8>, Outcome)> {
    let mut list = parse(original).ok()?;
    let outcome = apply_global(&mut list, records, &has_animation);
    let bytes = serialize(&list).ok()?;
    Some((bytes, outcome))
}

pub(crate) fn summary(directory: &str, sharers: &[String], outcome: &Outcome) -> String {
    let mut added = Vec::new();
    let mut kept = Vec::new();
    let mut skipped = Vec::new();
    for (name, result) in outcome {
        match result {
            Ok(()) => added.push(name.clone()),
            Err(Rejected::AlreadyPresent) => kept.push(name.clone()),
            Err(reason) => skipped.push(format!("{name} ({reason:?})")),
        }
    }
    let mut parts = Vec::new();
    if !added.is_empty() {
        parts.push(format!("added {}", added.join(", ")));
    }
    if !kept.is_empty() {
        parts.push(format!("kept the list's own {}", kept.join(", ")));
    }
    if !skipped.is_empty() {
        parts.push(format!("skipped {}", skipped.join(", ")));
    }
    if sharers.len() > 1 {
        let costumes: Vec<&str> = sharers
            .iter()
            .filter_map(|sharer| sharer.rsplit('/').next())
            .collect();
        parts.push(format!("one list for {}", costumes.join(" ")));
    }
    format!("[globalmotion] {directory}: {}", parts.join("; "))
}

#[cfg(not(test))]
pub(crate) use live::clone_engine_global_motion_v1;

#[cfg(not(test))]
pub(crate) fn arc_file_size(path: &str) -> Option<usize> {
    live::arc_size(path)
}

#[cfg(test)]
pub(crate) fn arc_file_size(_path: &str) -> Option<usize> {
    None
}

#[cfg(not(test))]
mod live {
    use super::*;
    use dionysus_api::{CloneCopyMotionV1, ERROR_NAME, ERROR_UNSUPPORTED};
    use dionysus_core::motion_append::{
        body_motion_directory, body_motion_list_path, BYTES_PER_MOTION, VANILLA_BODY_MOTION_OWNERS,
    };
    use dionysus_core::motion_list::hash40;
    use std::collections::{HashMap, HashSet};
    use std::sync::Mutex;

    type DecompressedSizeFn = unsafe extern "C" fn(u64, *mut usize) -> bool;

    fn served() -> &'static RwLock<HashMap<u64, String>> {
        static SERVED: OnceLock<RwLock<HashMap<u64, String>>> = OnceLock::new();
        SERVED.get_or_init(|| RwLock::new(HashMap::new()))
    }

    fn covered() -> &'static Mutex<HashSet<String>> {
        static COVERED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
        COVERED.get_or_init(|| Mutex::new(HashSet::new()))
    }

    fn decompressed_size() -> Option<DecompressedSizeFn> {
        static FOUND: OnceLock<usize> = OnceLock::new();
        let address = *FOUND.get_or_init(|| {
            let mut address = 0usize;
            let result = unsafe {
                skyline::nn::ro::LookupSymbol(
                    &mut address as *mut usize,
                    b"arcrop_get_decompressed_size\0".as_ptr(),
                )
            };
            if result == 0 {
                address
            } else {
                0
            }
        });
        (address != 0).then(|| unsafe { core::mem::transmute::<usize, DecompressedSizeFn>(address) })
    }

    pub(super) fn arc_size(path: &str) -> Option<usize> {
        let hash = hash40(path)?;
        let function = decompressed_size()?;
        let mut size = 0usize;
        unsafe { function(hash, &mut size) }.then_some(size)
    }

    #[no_mangle]
    pub unsafe extern "C" fn clone_engine_global_motion_v1(
        registration: *const CloneCopyMotionV1,
    ) -> i32 {
        let entry = match crate::kirby_motions::motion_record(registration, false) {
            Ok(entry) => entry,
            Err(code) => return code,
        };
        let who = if entry.fighter_kind >= 0 {
            format!("kind {}", entry.fighter_kind)
        } else {
            "a plugin".to_string()
        };
        let name = entry.name.clone();
        if entry.animation.is_empty() && entry.template.is_none() && !entry.victim {
            skyline::println!(
                "[globalmotion] REFUSED '{name}': it needs a template, an animation or the victim mode"
            );
            return ERROR_NAME;
        }
        let source = match (entry.victim, entry.template.as_deref(), entry.animation.as_str()) {
            (true, Some(template), "") => format!("victim from template {template}, the grabber's animation"),
            (true, Some(template), animation) => {
                format!("victim from template {template}, {animation} where the fighter ships it, else the grabber's")
            }
            (true, None, "") => "victim, the grabber's animation".to_string(),
            (true, None, animation) => format!("victim, {animation} where the fighter ships it, else the grabber's"),
            (false, Some(template), "") => format!("template {template}"),
            (false, Some(template), animation) => format!("{animation}, else template {template}"),
            (false, None, animation) => format!("{animation} where the fighter ships it"),
        };
        match record(entry) {
            Ok(()) => {
                skyline::println!("[globalmotion] {who} registered '{name}' for every fighter: {source}");
                if crate::mount_events::mounted() {
                    install();
                }
                0
            }
            Err(reason) => {
                skyline::println!("[globalmotion] REFUSED '{name}': {reason}");
                ERROR_UNSUPPORTED
            }
        }
    }

    extern "C" fn serve(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
        if buffer.is_null() || length == 0 {
            return false;
        }
        let Some(directory) = served()
            .read()
            .ok()
            .and_then(|held| held.get(&hash).cloned())
        else {
            return false;
        };
        let entries = registered();
        if entries.is_empty() {
            return false;
        }
        let mut original = vec![0u8; length];
        let Some(read) = arcropolis_api::load_original_file(hash, original.as_mut_slice()) else {
            skyline::println!("[globalmotion] {directory}: the motion list could not be read; leaving it alone");
            return false;
        };
        original.truncate(read);
        let sharers = sharers(&directory);
        let has_animation = |file: &str| {
            sharers
                .iter()
                .all(|sharer| arc_size(&format!("{sharer}/{file}")).is_some())
        };
        let Some((rebuilt, outcome)) = rebuild_with(&original, &entries, has_animation) else {
            skyline::println!("[globalmotion] {directory}: the motion list did not parse; leaving it alone");
            return false;
        };
        if rebuilt.len() > length {
            skyline::println!(
                "[globalmotion] {directory}: the rebuilt list is {} bytes but only {length} were reserved; leaving it alone",
                rebuilt.len()
            );
            return false;
        }
        unsafe { core::ptr::copy_nonoverlapping(rebuilt.as_ptr(), buffer, rebuilt.len()) };
        *out_size = rebuilt.len();
        skyline::println!("{}", summary(&directory, &sharers, &outcome));
        true
    }

    fn list_data(directory: &str) -> Option<u32> {
        let search = crate::fighter_modules::search_path_index(&format!("{directory}/motion_list.bin"));
        unsafe { crate::copy_model_probe::data_index(search) }
    }

    fn sharers(directory: &str) -> Vec<String> {
        let Some((owner, _)) = directory.rsplit_once('/') else {
            return vec![directory.to_string()];
        };
        let prefix = format!("{owner}/c");
        let mut candidates: Vec<String> = served()
            .read()
            .map(|held| {
                held.values()
                    .filter(|known| known.starts_with(&prefix))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        candidates.sort();
        match list_data(directory) {
            Some(own) => candidates
                .into_iter()
                .filter(|known| known == directory || list_data(known) == Some(own))
                .collect(),
            None => candidates,
        }
    }

    fn owners() -> Vec<String> {
        if registered().is_empty() {
            return Vec::new();
        }
        let mut owners: Vec<String> = VANILLA_BODY_MOTION_OWNERS
            .iter()
            .map(|owner| owner.to_string())
            .collect();
        if let Ok(definitions) = crate::clone_definitions().read() {
            for definition in definitions.iter() {
                let name = definition.resource_name.to_string();
                if !owners.contains(&name) {
                    owners.push(name);
                }
            }
        }
        owners
    }

    pub(crate) fn install() {
        let count = registered().len();
        if count == 0 {
            return;
        }
        if decompressed_size().is_none() {
            skyline::println!(
                "[globalmotion] ARCropolis does not export arcrop_get_decompressed_size; {count} global motion(s) are not served"
            );
            return;
        }
        let Ok(mut covered) = covered().lock() else {
            return;
        };
        let (mut lists, mut fighters, mut clones) = (0usize, 0usize, Vec::new());
        for owner in owners() {
            if !covered.insert(owner.clone()) {
                continue;
            }
            let mut found = false;
            for costume in 0..=LAST_COSTUME {
                let path = body_motion_list_path(&owner, costume);
                let (Some(size), Some(hash)) = (arc_size(&path), hash40(&path)) else {
                    continue;
                };
                if let Ok(mut held) = served().write() {
                    held.insert(hash, body_motion_directory(&owner, costume));
                }
                arcropolis_api::register_callback(
                    path.as_str(),
                    size + BYTES_PER_MOTION * MAX_MOTIONS,
                    serve,
                );
                lists += 1;
                found = true;
            }
            if found {
                fighters += 1;
                if !VANILLA_BODY_MOTION_OWNERS.contains(&owner.as_str()) {
                    clones.push(owner);
                }
            }
        }
        if lists == 0 {
            return;
        }
        skyline::println!(
            "[globalmotion] {count} global motion(s) go into {lists} body motion lists of {fighters} fighters; clones with their own lists: {}",
            if clones.is_empty() {
                "none".to_string()
            } else {
                clones.join(", ")
            }
        );
    }
}

#[cfg(not(test))]
pub(crate) fn install() {
    live::install();
}

#[cfg(test)]
pub(crate) fn install() {}
