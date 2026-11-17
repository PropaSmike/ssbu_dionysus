#![feature(proc_macro_hygiene)]
#![allow(non_snake_case, non_upper_case_globals, unused_imports, unused_variables)]

pub use smashline::skyline_smash as smash;

use dionysus_api::v2::{fighter, Grab, Hurtbox, Manifest, Motion};
use smash::lib::lua_const::*;
use smashline::{Agent, Main, Priority};

mod acmd;
mod articles;
mod csk;
mod fight;
mod float;
mod frame;
mod glide;
mod hooks;
mod hud;
mod kirby;
mod params;
mod status;
mod tether;
mod throws;

// Files the engine picks up when the pack ships them, with no code:
// - sound/bank/fighter_voice/vc_template_fighter_cNN: the fighter's voice, on any base
// - sound/bank/fighter_voice/vc_kirby_copy_template_fighter: Kirby's voice while he holds the copy
// - fighter/template_fighter/append/cNN/: more files loaded for the match
// - a param_template_fighter_template_fireball block in fighter/template_fighter/param/vl.prc:
//   the fireball's own params, read in place of Mario's param_fireball (Smashline 2, game 13.0.5)
// - finalsmash/template_fighter/shared/ and prebuilt;/movie/fighter/template_fighter/cNN/:
//   the Final Smash background and movies, which the Final Smash statuses start
// - ui/param/standard/standard_route_<name_label>.prc: the Classic Mode route

// TEMPLATE: the handle the rest of the crate uses.
// "template_fighter": the identity, the same string as Manifest::new's name.
fighter!(TEMPLATE, "template_fighter");

/// How many costumes the pack ships (c00..). Drives the manifest, the CSK
/// entry and the ParamConfig slots, so a pack with 6 or 12 changes one line.
pub const COSTUMES: u8 = 8;

/// The vanilla fighter the clone is built on, by resource name.
pub const BASE: &str = "mario";

extern "C" fn mods_mounted() {
    // work that needs the mods' files mounted by ARCropolis goes here
    dionysus_api::elog!("[template_v2] mods mounted");
}

#[skyline::main(name = "dionysus_moveset_template_v2")]
pub fn main() {
    // The same declaration can be a fighter.toml beside config.json; the engine
    // then registers at boot and register() below just returns that kind.
    // A table the installed Dionysus cannot read (a global motion, a grab, the
    // HUD, command inputs, a tether...) is left out by register(), which logs it;
    // the rest of the fighter still registers.
    let manifest = Manifest::new(
        "template_fighter", // name: folder under fighter/, Smashline agent name, identity
        BASE,               // base: the vanilla fighter it is built on
    )
    .costumes(COSTUMES as u32)                      // the costume count
    .color_start(0)                                 // the first costume folder, c00
    .own_css()                                      // csk::publish writes the select entry, not the engine
    .article("template_fireball", "mario/fireball") // fighter/template_fighter/model/template_fireball, copied from Mario's fireball
    .article_hurtbox(
        "template_fireball",      // the article
        Hurtbox::new("rot", 2.5), // a joint of its model, radius: attacks can hit the fireball; its status scripts decide what a hit does
    )
    // .article_original_code("template_fireball", false) // only your scripts run on it, like Smashline's use_original_code = false (Smashline 2, game 13.0.5)
    // .article_original_kind("template_fireball", true)  // with the line above: the game still sees Mario's fireball kind
    .kirby_article("template_fighter_fireball", "mario/fireball") // the copy's own article, files under fighter/kirby/model/template_fighter_fireball
    .kirby(1)                                       // one status number for Kirby's copy
    .kirby_motion(
        // name (must start with the fighter's name), animation file
        Motion::new("template_fighter_special_n", "template_fighterd00specialn.nuanmb")
            .template("mario_special_n")        // the Kirby motion whose settings it copies
            .scripts("templatefighterspecialn"), // game_/sound_/effect_/expression_ + this, on Agent::new("kirby")
    )
    .kirby_motion(
        Motion::new("template_fighter_special_air_n", "template_fighterd00specialairn.nuanmb")
            .template("mario_special_air_n")
            .scripts("templatefighterspecialairn"),
    );

    // A throw with its own opponent pose: every fighter, clones included, gets
    // THROWN with no animation, and the opponent plays this fighter's file
    // (throws.rs starts it).
    let manifest = manifest.global_motion(
        Motion::victim(throws::THROWN)       // the opponent's motion name, the same on every fighter
            .animation(throws::THROWN_FILE), // this fighter's own file; fighters that do not ship it get none
    );

    // A command grab: the engine accepts the CATCH box in acmd::game_specials.
    // For a Ganondorf-style hold instead, ask for CATCHED_GANON, go to a status
    // of your own, and ship catched_ganon, catched_ganon_dx and
    // catched_ganon_girl in your motion list: every opponent plays your file.
    let manifest = manifest.grab(
        Grab::new(*FIGHTER_STATUS_KIND_SPECIAL_S)        // the grabber status the CATCH box is out in
            .victim(*FIGHTER_STATUS_KIND_CAPTURE_PULLED) // the status the box gives the opponent
            .then(*FIGHTER_STATUS_KIND_CATCH_PULL),      // the grabber's status once caught: an ordinary grab from here
    );

    // Joker's Rebellion gauge by this fighter's portrait; hud.rs fills it.
    // .hud_layout("ui/layout/hud/template_fighter/layout.arc") draws a layout
    // the pack ships instead, or as well.
    let manifest = manifest.hud("jack"); // the fighter whose HUD it shows

    // The game's command inputs and one of this fighter's own, read in fight.rs.
    let manifest = fight::commands(manifest);

    // Samus's air tether and ledge grab, off until tether.rs is filled in and
    // her beam is minted above with .article("template_gbeam", "samus/gbeam").
    // let manifest = manifest
    //     .tether("samus")                        // the donor: its air_lasso_data and ledge boxes
    //     .tether_box([60.0, 48.0], [5.0, -34.0]); // a ledge box of your own, two corners from the fighter

    // More a manifest can declare, in docs/wiki/FIGHTERS.md:
    // .partner(true): a copy of this fighter follows it, the way Nana follows Popo
    // .switch_with(&["other_fighter"]): shares the player slot with another fighter of yours
    let kind = match TEMPLATE.register(manifest) {
        Ok(kind) => kind,
        Err(error) => {
            dionysus_api::elog!("[template_v2] registration refused: {error:?}");
            return;
        }
    };

    csk::publish();
    params::install(kind);

    Agent::new("template_fighter") // scripts under the clone's own name, never "mario"
        .game_acmd("game_attack11", acmd::game_attack11, Priority::Default) // script name, function, priority
        .effect_acmd("effect_attack11", acmd::effect_attack11, Priority::Default)
        .sound_acmd("sound_attack11", acmd::sound_attack11, Priority::Default)
        .expression_acmd("expression_attack11", acmd::expression_attack11, Priority::Default)
        .game_acmd("game_appeallw", acmd::game_appeallw, Priority::Default)
        .game_acmd("game_specials", acmd::game_specials, Priority::Default) // the command grab
        .status(smashline::Exec, *smash::lib::lua_const::FIGHTER_STATUS_KIND_WAIT, status::wait_exec) // status line, status kind, function
        .on_line(Main, frame::once_per_frame) // once per frame
        .on_start(frame::on_start)            // when the fighter is created
        .install();

    articles::install(kind);
    kirby::install();
    hooks::install();
    float::install();
    // glide::install(); // Brawl's glide starts from the same input as the float: install one of the two
    tether::install();
    fight::install();

    if let Err(error) = dionysus_api::v2::on_mods_mounted(mods_mounted) {
        dionysus_api::elog!("[template_v2] mount event unavailable: {error:?}");
    }

    dionysus_api::elog!("[template_v2] installed; kind {kind}");
}
