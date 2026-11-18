//! Stats. Every value is a `param()` call: `vl.prc` values go on to
//! ParamConfig (the game reads them through the getters it hooks); the engine
//! writes the values the game reads past those getters: `fighter_param.prc`
//! attributes, `fighter_param_motion.prc`, the `fighter/common/param/` files
//! and throw positions.

use crate::TEMPLATE;

pub fn install() {
    let last = crate::COSTUMES as i32 - 1;

    // vl.prc values, "group.field". ParamConfig answers with the first entry
    // that covers the costume and holds the field, so the one-costume entry is
    // registered before the one for every costume.
    let applied = TEMPLATE.param("param_special_n.fireball_speed_mul").slot(last).set(1.3);
    let applied = applied & TEMPLATE.param("param_special_n.fireball_speed_mul").set(1.15);
    // multiply the base's value instead of setting it
    let applied = applied & TEMPLATE.param("param_special_hi.y_spd_air").mul(1.1);

    // fighter_param.prc attribute: read straight out of the row by main, so the engine
    let applied = applied & TEMPLATE.param("run_speed_max").mul(1.10);
    // an integer attribute
    let applied = applied & TEMPLATE.param("jump_squat_frame").int(3);
    // one costume only: .slot(costume) before the value
    let applied = applied & TEMPLATE.param("jump_y").slot(last).mul(0.95);
    // fighter_param_motion.prc field
    let applied = applied & TEMPLATE.param("param_motion.escape_air_slide_distance").set(60.0);
    // one of the six fighter/common/param/ files; set shield_reset with shield_max
    let applied = applied & TEMPLATE.param("common.shield_max").set(20.0);
    let applied = applied & TEMPLATE.param("common.shield_reset").set(7.5);
    // where a fighter this clone throws sits: forward throw, Y
    let applied = applied & TEMPLATE.param("param_thrown.offset_f_y").set(8.0);
    if !applied {
        dionysus_api::elog!("[template_v2] a param was refused; ParamConfig missing, or see the log");
    }
}
