use dionysus_core::enemy_toml;

const MOD_ROOT: &str = "sd:/ultimate/mods";

pub(crate) fn load_all() {
    let Ok(entries) = std::fs::read_dir(MOD_ROOT) else {
        return;
    };
    let mut packs: Vec<(String, String)> = entries
        .flatten()
        .filter_map(|entry| {
            let text = std::fs::read_to_string(entry.path().join("enemy.toml")).ok()?;
            if !crate::mod_folders::enabled(&entry.path()) {
                return None;
            }
            Some((entry.file_name().to_string_lossy().to_string(), text))
        })
        .collect();
    packs.sort_by(|a, b| a.0.cmp(&b.0));
    for (directory, text) in packs {
        let declarations = match enemy_toml::parse(&text) {
            Ok(declarations) => declarations,
            Err(error) => {
                skyline::println!("[enemypack] {directory}/enemy.toml is not readable: {error}; skipped");
                continue;
            }
        };
        for declaration in declarations {
            let base = enemy_toml::base_folder(declaration.base).unwrap_or("?");
            let kind = crate::enemy_clones::register(declaration.base, &declaration.name);
            if kind < 0 {
                skyline::println!(
                    "[enemypack] {directory}: {} on {base} REFUSED ({kind})",
                    declaration.name
                );
            } else {
                skyline::println!(
                    "[enemypack] {directory}: {} on {base} is enemy kind {kind}; files under enemy/{}/",
                    declaration.name,
                    declaration.name
                );
            }
        }
    }
}
