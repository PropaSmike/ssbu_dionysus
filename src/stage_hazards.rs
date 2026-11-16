#![allow(dead_code)]

use core::sync::atomic::{AtomicU8, Ordering};

use crate::stage_dispatch::EXPANDED_ENTRIES;

pub const IS_GIMMICK: usize = 0x10;

const PLAYER_RULE: u8 = 0;
const FORCED_OFF: u8 = 1;
const FORCED_ON: u8 = 2;

static FORCED: [AtomicU8; EXPANDED_ENTRIES] =
    [const { AtomicU8::new(PLAYER_RULE) }; EXPANDED_ENTRIES];

pub(crate) fn force(stage_id: u32, hazards: bool) -> bool {
    let Some(slot) = FORCED.get(stage_id as usize) else {
        return false;
    };
    slot.store(if hazards { FORCED_ON } else { FORCED_OFF }, Ordering::Release);
    true
}

pub(crate) fn forced(stage_id: u32) -> Option<bool> {
    match FORCED.get(stage_id as usize)?.load(Ordering::Acquire) {
        FORCED_OFF => Some(false),
        FORCED_ON => Some(true),
        _ => None,
    }
}

#[cfg(all(not(test), feature = "stage_mint"))]
pub(crate) unsafe fn apply(parameter: *mut u8, stage_id: u32) {
    use core::sync::atomic::AtomicU32;
    static LAST_REPORTED: AtomicU32 = AtomicU32::new(u32::MAX);
    if parameter.is_null() {
        return;
    }
    let Some(hazards) = forced(stage_id) else {
        return;
    };
    core::ptr::write_volatile(parameter.add(IS_GIMMICK), hazards as u8);
    if LAST_REPORTED.swap(stage_id, Ordering::Relaxed) != stage_id {
        skyline::println!(
            "[stagehazards] StageID {stage_id}: hazards {} whatever the rule says",
            if hazards { "on" } else { "off" }
        );
    }
}

#[cfg(not(all(not(test), feature = "stage_mint")))]
pub(crate) unsafe fn apply(_parameter: *mut u8, _stage_id: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stage_without_the_key_follows_the_rule() {
        assert_eq!(forced(400), None);
        assert_eq!(forced(0), None);
    }

    #[test]
    fn a_forced_stage_reports_its_setting() {
        assert!(force(500, false));
        assert_eq!(forced(500), Some(false));
        assert!(force(501, true));
        assert_eq!(forced(501), Some(true));
    }

    #[test]
    fn an_id_past_the_table_is_refused() {
        assert!(!force(EXPANDED_ENTRIES as u32, false));
        assert_eq!(forced(EXPANDED_ENTRIES as u32), None);
    }
}
