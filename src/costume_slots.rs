use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const MODS_DIRECTORY: &str = "sd:/ultimate/mods";

const MARKER_NAME: &str = "clone_engine_costumes.txt";

const MAX_SLOT_INDEX: u32 = 255;

const MAX_ENTRIES: usize = 512;

static DECLARED: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();

static COUNT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

fn declared() -> &'static Mutex<HashMap<String, u32>> {
    DECLARED.get_or_init(|| Mutex::new(HashMap::new()))
}

fn parse(contents: &str) -> HashMap<String, u32> {
    let mut declared = HashMap::new();
    for line in contents.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(identity), Some(slot)) = (fields.next(), fields.next()) else {
            continue;
        };
        if fields.next().is_some() {
            continue;
        }
        let Ok(slot) = slot.parse::<u32>() else {
            continue;
        };
        if slot > MAX_SLOT_INDEX || declared.len() >= MAX_ENTRIES {
            continue;
        }
        let entry = declared.entry(identity.to_string()).or_insert(0);
        *entry = (*entry).max(slot);
    }
    declared
}

fn declare(identity: &str, highest_slot: u32) -> bool {
    if identity.is_empty()
        || identity.split_whitespace().count() != 1
        || highest_slot > MAX_SLOT_INDEX
    {
        return false;
    }
    let Ok(mut declared) = declared().lock() else {
        return false;
    };
    if declared.len() >= MAX_ENTRIES && !declared.contains_key(identity) {
        return false;
    }
    let entry = declared.entry(identity.to_string()).or_insert(0);
    *entry = (*entry).max(highest_slot);
    COUNT.store(declared.len(), core::sync::atomic::Ordering::Relaxed);
    true
}

pub(crate) fn scan_mods() {
    let Ok(entries) = std::fs::read_dir(MODS_DIRECTORY) else {
        return;
    };
    let mut files = 0usize;
    for entry in entries.flatten() {
        let marker = entry.path().join(MARKER_NAME);
        let Ok(contents) = std::fs::read_to_string(&marker) else {
            continue;
        };
        if !crate::mod_folders::enabled(&entry.path()) {
            continue;
        }
        files += 1;
        for (identity, slot) in parse(&contents) {
            declare(&identity, slot);
        }
    }
    if files == 0 {
        return;
    }
    let Ok(declared) = declared().lock() else {
        return;
    };
    let mut listed = declared
        .iter()
        .map(|(identity, slot)| format!("{identity}=c{slot:02}"))
        .collect::<Vec<_>>();
    listed.sort();
    skyline::println!(
        "[costumeslots] {files} marker file(s) declare extra slots: {}",
        listed.join(" ")
    );
}

pub(crate) fn register(identity: &str, highest_slot: i32) -> bool {
    if highest_slot < 0 {
        return false;
    }
    declare(identity, highest_slot as u32)
}

pub(crate) fn effective_color_count(identity: &str, registered: u8, first_slot: u8) -> u8 {
    if COUNT.load(core::sync::atomic::Ordering::Relaxed) == 0 {
        return registered;
    }
    let Ok(declared) = declared().lock() else {
        return registered;
    };
    let Some(highest) = declared.get(identity).copied() else {
        return registered;
    };
    if highest < u32::from(first_slot) {
        return registered;
    }
    let needed = (highest - u32::from(first_slot) + 1).min(MAX_SLOT_INDEX + 1) as u16;
    needed
        .max(u16::from(registered))
        .min(256 - u16::from(first_slot))
        .min(255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_highest_index_per_identity() {
        let parsed = parse(
            "# skins\n\
             fighter_kind_example 9\n\
             fighter_kind_example 15\n\
             fighter_kind_example 11\n",
        );
        assert_eq!(parsed.get("fighter_kind_example"), Some(&15));
    }

    #[test]
    fn skips_malformed_lines_without_losing_good_ones() {
        let parsed = parse(
            "\n\
             fighter_kind_ok 8\n\
             fighter_kind_bad notanumber\n\
             fighter_kind_short\n\
             fighter_kind_extra 8 trailing\n\
             fighter_kind_huge 999\n\
             fighter_kind_comment 12 # mine\n",
        );
        assert_eq!(parsed.get("fighter_kind_ok"), Some(&8));
        assert_eq!(parsed.get("fighter_kind_bad"), None);
        assert_eq!(parsed.get("fighter_kind_short"), None);
        assert_eq!(parsed.get("fighter_kind_extra"), None);
        assert_eq!(parsed.get("fighter_kind_huge"), None);
        assert_eq!(parsed.get("fighter_kind_comment"), Some(&12));
        assert_eq!(parsed.len(), 2);
    }
}
