use dionysus_core::settings::{self, Settings, DEFAULT_TEXT};
use std::sync::OnceLock;

const SETTINGS_DIRECTORY: &str = "sd:/ultimate/dionysus";
const SETTINGS_PATH: &str = "sd:/ultimate/dionysus/settings.toml";

static SETTINGS: OnceLock<Settings> = OnceLock::new();

pub(crate) fn get() -> Settings {
    *SETTINGS.get_or_init(load)
}

fn load() -> Settings {
    crate::upgrade::carry_old_folder();
    let loaded = match std::fs::read_to_string(SETTINGS_PATH) {
        Ok(text) => {
            let (loaded, problems) = settings::parse(&text);
            for problem in problems {
                skyline::println!("[settings] {SETTINGS_PATH} {problem}");
            }
            loaded
        }
        Err(error) if std::path::Path::new(SETTINGS_PATH).exists() => {
            skyline::println!("[settings] {SETTINGS_PATH} is not readable ({error}); using the defaults");
            Settings::default()
        }
        Err(_) => {
            let _ = std::fs::create_dir_all(SETTINGS_DIRECTORY);
            match std::fs::write(SETTINGS_PATH, DEFAULT_TEXT) {
                Ok(()) => skyline::println!("[settings] wrote {SETTINGS_PATH} with the defaults"),
                Err(error) => skyline::println!("[settings] could not write {SETTINGS_PATH} ({error}); using the defaults"),
            }
            Settings::default()
        }
    };
    skyline::println!(
        "[settings] gliding {}, stage select {}",
        if loaded.gliding { "on" } else { "off" },
        loaded.stage_select_ui.name()
    );
    loaded
}

#[no_mangle]
pub extern "C" fn clone_engine_gliding_v1() -> bool {
    get().gliding
}
