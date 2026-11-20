//! Command inputs and facing the opponent, the controls Ryu has. lib.rs adds
//! the commands to the manifest with `commands`; `install` reads them.

use crate::smash;
use dionysus_api::fight::{self, Button, Command};
use dionysus_api::v2::Manifest;
use smash::app::lua_bind::*;
use smash::lib::lua_const::*;
use smash::lua2cpp::L2CFighterCommon;
use smashline::{Agent, Main};

/// This fighter's own command, by the name the code reads it with.
const FIREBALL: &str = "fireball";

/// The commands. An engine without command inputs would refuse the whole
/// manifest, so lib.rs calls this only when it has them.
pub fn commands(manifest: Manifest) -> Manifest {
    manifest
        // the game's own commands: ControlModule::get_command_flag_cat(boma, 3)
        // then carries the FIGHTER_PAD_CMD_CAT4_FLAG_* bits, as on Ryu
        .vanilla_commands(true)
        .special_command_life_max(12) // frames allowed between two directions, every command (the game's 10)
        .command(
            Command::new(FIREBALL, "41236") // name, numpad input read for the way the fighter faces as it starts
                .button(Button::Special)     // the button that ends it: Special, Attack, Either or None
                .cpu(0.25),                  // a CPU's special press reads it 25% of the time
        )
}

unsafe extern "C" fn every_frame(fighter: &mut L2CFighterCommon) {
    // in a 1-on-1 on the ground, turn toward the opponent: in wait, crouch,
    // landing and shield, and as jabs, tilts, specials and grabs start
    fight::face_opponent(fighter);

    // 41236 + special throws the fireball. The input ends with the stick
    // forward, so the game starts side special: switch it to neutral special.
    let boma = fighter.module_accessor;
    let status = StatusModule::status_kind(boma);
    if status != *FIGHTER_STATUS_KIND_SPECIAL_S && status != *FIGHTER_STATUS_KIND_SPECIAL_N {
        return;
    }
    // lr: the way the command was input for. The 4 at its start turns the
    // fighter around on the ground, so face that way again.
    if let Some(lr) = fight::command_lr(fighter, FIREBALL) {
        fight::clear_command(fighter, FIREBALL); // used up
        PostureModule::set_lr(boma, lr);
        PostureModule::update_rot_y_lr(boma);
        if status == *FIGHTER_STATUS_KIND_SPECIAL_S {
            StatusModule::change_status_request_from_script(boma, *FIGHTER_STATUS_KIND_SPECIAL_N, false);
        }
    }
}

pub fn install() {
    Agent::new("template_fighter")
        .on_line(Main, every_frame) // once per frame
        .install();
}
