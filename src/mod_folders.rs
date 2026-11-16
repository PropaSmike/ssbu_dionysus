use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

pub(crate) const MOD_ROOT: &str = "sd:/ultimate/mods";
const ARCROPOLIS_CONFIG: &str = "sd:/ultimate/arcropolis/config";
const SEEN_FILE: &str = "mod_cache";

type IsModEnabled = unsafe extern "C" fn(u64) -> bool;

static ANSWERS: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

pub(crate) fn arcropolis_key(name: &str) -> u64 {
    dionysus_core::hash::hash40(&format!("{MOD_ROOT}/{name}"))
}

pub(crate) fn numbers(text: &str) -> HashSet<u64> {
    text.split(|character: char| !character.is_ascii_digit())
        .filter_map(|number| number.parse().ok())
        .collect()
}

fn seen_by_arcropolis() -> &'static HashSet<u64> {
    static SEEN: OnceLock<HashSet<u64>> = OnceLock::new();
    SEEN.get_or_init(|| {
        let mut seen = HashSet::new();
        let Ok(users) = std::fs::read_dir(ARCROPOLIS_CONFIG) else {
            return seen;
        };
        for user in users.flatten() {
            let Ok(profiles) = std::fs::read_dir(user.path()) else {
                continue;
            };
            for profile in profiles.flatten() {
                if let Ok(text) = std::fs::read_to_string(profile.path().join(SEEN_FILE)) {
                    seen.extend(numbers(&text));
                }
            }
        }
        seen
    })
}

#[cfg(not(test))]
fn arcropolis_says_on(key: u64) -> Option<bool> {
    let mut address = 0usize;
    let found = unsafe { skyline::nn::ro::LookupSymbol(&mut address, b"arcrop_is_mod_enabled\0".as_ptr()) };
    if found != 0 || address == 0 {
        return None;
    }
    let function: IsModEnabled = unsafe { core::mem::transmute(address) };
    Some(unsafe { function(key) })
}

#[cfg(test)]
fn arcropolis_says_on(_key: u64) -> Option<bool> {
    None
}

pub(crate) fn switched_off(says_on: Option<bool>, seen_before: bool) -> bool {
    says_on == Some(false) && seen_before
}

pub(crate) fn enabled(folder: &Path) -> bool {
    folder.is_dir() && folder.file_name().and_then(|name| name.to_str()).is_some_and(answer)
}

fn answer(name: &str) -> bool {
    if let Ok(answers) = ANSWERS.lock() {
        if let Some(answer) = answers.as_ref().and_then(|answers| answers.get(name)) {
            return *answer;
        }
    }
    let key = arcropolis_key(name);
    let answer = !switched_off(arcropolis_says_on(key), seen_by_arcropolis().contains(&key));
    if !answer {
        #[cfg(not(test))]
        skyline::println!("[mods] {name} is switched off in ARCropolis, so Dionysus skips it too");
    }
    if let Ok(mut answers) = ANSWERS.lock() {
        answers.get_or_insert_with(HashMap::new).insert(name.to_string(), answer);
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_arcropolis_case_kept_hash_of_the_folder_path() {
        let path = "sd:/ultimate/mods/Dionysus Modes";
        assert_eq!(arcropolis_key("Dionysus Modes") >> 32, path.len() as u64);
        assert_ne!(arcropolis_key("Dionysus Modes"), arcropolis_key("dionysus modes"));
    }

    #[test]
    fn only_a_folder_arcropolis_knows_and_has_off_is_skipped() {
        assert!(switched_off(Some(false), true));
        assert!(!switched_off(Some(false), false));
        assert!(!switched_off(Some(true), true));
        assert!(!switched_off(None, true));
    }

    #[test]
    fn the_seen_list_is_read_as_numbers() {
        assert_eq!(numbers("[205788812376,193602620372]"), HashSet::from([205788812376, 193602620372]));
        assert!(numbers("[]").is_empty());
    }

    #[test]
    fn a_file_in_the_mods_folder_is_never_a_pack() {
        assert!(!enabled(Path::new("sd:/ultimate/mods/Some Pack.zip")));
    }
}
