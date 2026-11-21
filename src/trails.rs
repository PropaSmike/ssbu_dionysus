use std::sync::Mutex;

const TRAIL_FOLDER_SITE: usize = 0x355_fc30;
const TRAIL_FOLDER_WORD: u32 = 0xf900_33e8;
const SEARCH_KEY_MASK: u64 = 0xff_ffff_ffff;

static REPORTED: Mutex<Vec<u64>> = Mutex::new(Vec::new());

fn trail_hash(name: &str) -> u64 {
    crate::hash40::hash40(&format!("effect/fighter/{name}/trail"))
}

fn base_trail(built: u64) -> Option<(u64, &'static str, &'static str)> {
    let definitions = crate::clone_definitions().read().ok()?;
    definitions
        .iter()
        .find(|definition| trail_hash(definition.resource_name) == built)
        .map(|definition| {
            (
                trail_hash(definition.base_resource_name),
                definition.resource_name,
                definition.base_resource_name,
            )
        })
}

fn report(built: u64, clone: &str, base: &str) {
    let Ok(mut reported) = REPORTED.lock() else {
        return;
    };
    if reported.contains(&built) {
        return;
    }
    reported.push(built);
    crate::dbg_log_public(&format!(
        "[trail] {clone} has no effect/fighter/{clone}/trail, so it uses {base}'s trail textures"
    ));
}

#[skyline::hook(offset = crate::game::rel(TRAIL_FOLDER_SITE), inline)]
unsafe fn trail_folder(ctx: &mut skyline::hooks::InlineCtx) {
    let built = ctx.registers[8].x() & SEARCH_KEY_MASK;
    let Some((base_hash, clone, base)) = base_trail(built) else {
        return;
    };
    let service = crate::finalsmash_residency::resource_service();
    if crate::finalsmash_residency::directory_index_for(service, built).is_some()
        || crate::finalsmash_residency::directory_index_for(service, base_hash).is_none()
    {
        return;
    }
    ctx.registers[8].set_x(base_hash);
    report(built, clone, base);
}

pub(crate) fn install() {
    let word = unsafe { crate::game::text_word(TRAIL_FOLDER_SITE) };
    if word != TRAIL_FOLDER_WORD {
        skyline::println!(
            "[trail] effect loader not recognised ({word:#010x}); a clone without a trail folder of its own has no trails"
        );
        return;
    }
    skyline::install_hook!(trail_folder);
}
