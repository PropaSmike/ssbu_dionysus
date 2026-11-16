#![allow(dead_code)]

use crate::stage_bounds::{StageSwitch, STAGE_SWITCHES, STAGE_SWITCH_COUNT, UNEXTENDED_SWITCHES};
use crate::stage_relocation::{encode_add, encode_adrp, PlanError};

pub const EXPANDED_ENTRIES: usize = 896;

pub fn required_bytes() -> usize {
    STAGE_SWITCHES.len() * (0x1000 + EXPANDED_ENTRIES * 4)
}

pub fn aligned_base(cursor: usize, page_offset: usize) -> usize {
    let base = (cursor & !0xFFF) | page_offset;
    if base >= cursor {
        base
    } else {
        base + 0x1000
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DispatchError {
    OpcodeMismatch {
        address: usize,
        expected: u32,
        actual: u32,
    },
    TargetOutOfReach {
        jump_table: usize,
        index: usize,
        target: usize,
        new_base: usize,
    },
    DonorOutOfRange {
        donor: usize,
        entries: usize,
    },
    MintedOutOfRange {
        minted: usize,
    },
    NotPageCongruent {
        old_base: usize,
        new_base: usize,
    },
    Unreachable(PlanError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchPlan {
    pub cmp_at: usize,
    pub cmp_word: u32,
    pub adrp_at: usize,
    pub adrp_word: u32,
    pub new_base: usize,
    pub entries: Vec<i32>,
}

impl SwitchPlan {
    pub fn target(&self, index: usize) -> usize {
        (self.new_base as i64 + self.entries[index] as i64) as usize
    }
}

pub fn plan(
    switch: &StageSwitch,
    new_base: usize,
    text_base: usize,
    read_entry: impl Fn(usize) -> i32,
    read_word: impl Fn(usize) -> u32,
) -> Result<SwitchPlan, DispatchError> {
    let cmp_at = text_base + switch.cmp_at;
    let adrp_at = text_base + switch.adrp_at;
    for (address, expected) in [
        (cmp_at, switch.cmp_opcode),
        (adrp_at, switch.adrp_opcode),
        (adrp_at + 4, switch.add_opcode),
    ] {
        let actual = read_word(address);
        if actual != expected {
            return Err(DispatchError::OpcodeMismatch {
                address,
                expected,
                actual,
            });
        }
    }

    let old_base = text_base + switch.jump_table;
    if new_base & 0xFFF != old_base & 0xFFF
        || encode_add(new_base, switch.base_register) != switch.add_opcode
    {
        return Err(DispatchError::NotPageCongruent { old_base, new_base });
    }
    let default_target = text_base + switch.default_target;
    let mut entries = Vec::with_capacity(EXPANDED_ENTRIES);
    for index in 0..EXPANDED_ENTRIES {
        let target = if index < switch.entries {
            (old_base as i64 + read_entry(old_base + index * 4) as i64) as usize
        } else {
            default_target
        };
        let offset = target as i64 - new_base as i64;
        if offset > i32::MAX as i64 || offset < i32::MIN as i64 {
            return Err(DispatchError::TargetOutOfReach {
                jump_table: old_base,
                index,
                target,
                new_base,
            });
        }
        entries.push(offset as i32);
    }

    Ok(SwitchPlan {
        cmp_at,
        cmp_word: switch.new_cmp_opcode,
        adrp_at,
        adrp_word: encode_adrp(adrp_at, new_base, switch.base_register)
            .map_err(DispatchError::Unreachable)?,
        new_base,
        entries,
    })
}

pub fn set_donor(
    plan: &mut SwitchPlan,
    switch: &StageSwitch,
    minted: usize,
    donor: usize,
) -> Result<(), DispatchError> {
    if donor >= switch.entries {
        return Err(DispatchError::DonorOutOfRange {
            donor,
            entries: switch.entries,
        });
    }
    if minted >= EXPANDED_ENTRIES {
        return Err(DispatchError::MintedOutOfRange { minted });
    }
    plan.entries[minted] = plan.entries[donor];
    Ok(())
}

const NO_DONOR: u32 = u32::MAX;
static DONOR_BASE: [core::sync::atomic::AtomicU32; EXPANDED_ENTRIES] =
    [const { core::sync::atomic::AtomicU32::new(NO_DONOR) }; EXPANDED_ENTRIES];

static DONOR_OWNS_STDAT: [core::sync::atomic::AtomicBool; EXPANDED_ENTRIES] =
    [const { core::sync::atomic::AtomicBool::new(false) }; EXPANDED_ENTRIES];

fn donor_slot(minted: u32) -> Option<usize> {
    let slot = minted as usize;
    (minted != 0 && slot < EXPANDED_ENTRIES).then_some(slot)
}

pub(crate) fn set_owns_stdat(minted: u32) -> bool {
    use core::sync::atomic::Ordering;
    let Some(slot) = donor_slot(minted) else {
        return false;
    };
    if DONOR_BASE[slot].load(Ordering::Acquire) == NO_DONOR {
        return false;
    }
    DONOR_OWNS_STDAT[slot].store(true, Ordering::Release);
    true
}

pub(crate) fn owns_stdat(minted: u32) -> bool {
    use core::sync::atomic::Ordering;
    donor_slot(minted).is_some_and(|slot| {
        DONOR_BASE[slot].load(Ordering::Acquire) != NO_DONOR
            && DONOR_OWNS_STDAT[slot].load(Ordering::Acquire)
    })
}

pub(crate) fn set_donor_kind(minted: u32, donor: u32) -> bool {
    use core::sync::atomic::Ordering;
    let Some(slot) = donor_slot(minted) else {
        return false;
    };
    if donor == NO_DONOR {
        return false;
    }
    DONOR_BASE[slot].store(donor, Ordering::Release);
    true
}

pub(crate) fn donor_for(id: u32) -> Option<u32> {
    use core::sync::atomic::Ordering;
    let slot = donor_slot(id)?;
    match DONOR_BASE[slot].load(Ordering::Acquire) {
        NO_DONOR => None,
        donor => Some(donor),
    }
}

const STAGE_FACTORY_DISPATCH: usize = 0x2633d10;
const STAGE_FACTORY_DISPATCH_OPCODE: u32 = 0x7105ad1f;

const STAGE_DATA_FACTORY_DISPATCH: usize = 0x240ef9c;
const STAGE_DATA_FACTORY_DISPATCH_OPCODE: u32 = 0x7105ad1f;

const STAGE_DATA_CACHE_GATE: usize = 0x25e3c80;
const STAGE_DATA_CACHE_GATE_OPCODE: u32 = 0xb5000168;

const STAGE_DATA_DONOR_GATE: usize = 0x245c810;
const STAGE_DATA_DONOR_GATE_OPCODE: u32 = 0x7103451f;

const STAGE_STDAT_METADATA_LOOKUP: usize = 0x25ff924;
const STAGE_STDAT_METADATA_LOOKUP_OPCODE: u32 = 0x5280090a;

const END_FLAT_STAGE_ID_GATE: usize = 0x28348fc;
const END_FLAT_STAGE_ID_GATE_OPCODE: u32 = 0x71000d3f;

const END_FOUR_PLATE_GATE: usize = 0x2835ed4;
const END_FOUR_PLATE_GATE_OPCODE: u32 = 0x7100113f;

const END_BATTLE_DECORATION_DECISION: usize = 0x28394a4;
const END_BATTLE_DECORATION_DECISION_OPCODE: u32 = 0xf9418a6a;

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(STAGE_FACTORY_DISPATCH), inline)]
unsafe fn stage_factory_dispatch_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let minted = ctx.registers[8].x() as u32;
    let Some(donor) = donor_for(minted) else {
        return;
    };
    ctx.registers[8].set_x(donor as u64);
    let slot = ctx.registers[22].x() as u32;
    skyline::println!(
        "[stagedisp] slot {}: dispatch StageID {} through StageID {}; setting preserved",
        slot,
        minted,
        donor
    );
}

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(STAGE_DATA_FACTORY_DISPATCH), inline)]
unsafe fn stage_data_factory_dispatch_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let minted = ctx.registers[8].x() as u32;
    let Some(donor) = donor_for(minted) else {
        return;
    };
    ctx.registers[8].set_x(donor as u64);
    skyline::println!(
        "[stageparamdisp] dispatch StageID {} through StageID {}; source preserved at {:#x}",
        minted,
        donor,
        ctx.registers[19].x()
    );
}

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(STAGE_DATA_CACHE_GATE), inline)]
unsafe fn stage_data_cache_gate_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let cached = ctx.registers[8].x() as usize;
    let wrapper = ctx.registers[19].x() as usize;
    if cached == 0 || wrapper == 0 {
        return;
    }

    let source = core::ptr::read_volatile(wrapper as *const usize);
    if source == 0 {
        return;
    }
    let minted = core::ptr::read_volatile((source + 8) as *const u32);
    let Some(donor) = donor_for(minted) else {
        return;
    };
    let old_vtable = core::ptr::read_volatile(cached as *const usize);

    ctx.registers[8].set_x(0);
    skyline::println!(
        "[stageparamcache] rebuild cached runtime data for StageID {} via donor {}; wrapper={:#x} old={:#x} vtable={:#x}",
        minted,
        donor,
        wrapper,
        cached,
        old_vtable
    );
}

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(STAGE_DATA_DONOR_GATE), inline)]
unsafe fn stage_data_donor_gate_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let minted = ctx.registers[8].x() as u32;
    let Some(donor) = donor_for(minted) else {
        return;
    };
    ctx.registers[8].set_x(donor as u64);
    skyline::println!(
        "[stageparamgate] compare StageID {} as donor StageID {}; source preserved",
        minted,
        donor
    );
}

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(STAGE_STDAT_METADATA_LOOKUP), inline)]
unsafe fn stage_stdat_metadata_lookup_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let minted = ctx.registers[9].x() as u32;
    let Some(donor) = donor_for(minted) else {
        return;
    };
    crate::stage_collision_probe::arm_stdat_scan(minted, donor);
    if owns_stdat(minted) {
        skyline::println!(
            "[stagestdatscan] StageID {} keeps its own place for the .stdat scan;              {}'s is not borrowed",
            minted,
            donor
        );
        return;
    }
    ctx.registers[9].set_x(donor as u64);
    skyline::println!(
        "[stagestdatscan] use donor StageID {} metadata for StageID {}; source at {:#x} preserved",
        donor,
        minted,
        ctx.registers[25].x()
    );
}

#[cfg(feature = "stage_mint_places")]
unsafe fn translate_end_stage_id(
    ctx: &mut skyline::hooks::InlineCtx,
    register: usize,
    label: &str,
) {
    let minted = ctx.registers[register].x() as u32;
    let Some(donor) = donor_for(minted) else {
        return;
    };
    ctx.registers[register].set_x(donor as u64);
    skyline::println!(
        "[endform] {label}: compare StageID {} as donor StageID {}",
        minted,
        donor,
    );
}

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(END_FLAT_STAGE_ID_GATE), inline)]
unsafe fn end_flat_stage_id_gate_hook(ctx: &mut skyline::hooks::InlineCtx) {
    translate_end_stage_id(ctx, 9, "flat");
}

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(END_FOUR_PLATE_GATE), inline)]
unsafe fn end_four_plate_gate_hook(ctx: &mut skyline::hooks::InlineCtx) {
    translate_end_stage_id(ctx, 9, "four-plate");
}

#[cfg(feature = "stage_mint_places")]
#[skyline::hook(offset = crate::game::rel(END_BATTLE_DECORATION_DECISION), inline)]
unsafe fn end_battle_decoration_decision_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let stage = ctx.registers[19].x() as usize;
    if stage == 0 {
        return;
    }
    let minted = core::ptr::read_volatile((stage + 8) as *const u32);
    if donor_for(minted) != Some(4) {
        return;
    }
    let native = ctx.registers[8].x() as u32;
    ctx.registers[8].set_x(0);
    skyline::println!(
        "[endform] decorations: keep authored scenery for StageID {}; native predicate was {}",
        minted,
        native,
    );
}

#[cfg(feature = "stage_mint_places")]
pub(crate) fn install() {
    unsafe {
        for (site, expected, label) in [
            (
                STAGE_DATA_FACTORY_DISPATCH,
                STAGE_DATA_FACTORY_DISPATCH_OPCODE,
                "runtime-data class",
            ),
            (
                STAGE_DATA_CACHE_GATE,
                STAGE_DATA_CACHE_GATE_OPCODE,
                "runtime-data cache",
            ),
            (
                STAGE_FACTORY_DISPATCH,
                STAGE_FACTORY_DISPATCH_OPCODE,
                "stage class",
            ),
            (
                STAGE_DATA_DONOR_GATE,
                STAGE_DATA_DONOR_GATE_OPCODE,
                "runtime-data donor predicate",
            ),
            (
                STAGE_STDAT_METADATA_LOOKUP,
                STAGE_STDAT_METADATA_LOOKUP_OPCODE,
                ".stdat metadata lookup",
            ),
            (
                END_FLAT_STAGE_ID_GATE,
                END_FLAT_STAGE_ID_GATE_OPCODE,
                "End flat-stage predicate",
            ),
            (
                END_FOUR_PLATE_GATE,
                END_FOUR_PLATE_GATE_OPCODE,
                "End four-platform predicate",
            ),
            (
                END_BATTLE_DECORATION_DECISION,
                END_BATTLE_DECORATION_DECISION_OPCODE,
                "End Battlefield decoration decision",
            ),
        ] {
            let observed = crate::game::text_word(site);
            if observed != expected {
                skyline::println!(
                    "[stagedisp] REFUSED {label} dispatch at {site:#x}: expected \
                     {expected:#010x}, found {observed:#010x}; no dispatch hooks installed"
                );
                return;
            }
        }
        skyline::install_hooks!(
            stage_data_cache_gate_hook,
            stage_data_factory_dispatch_hook,
            stage_factory_dispatch_hook,
            stage_data_donor_gate_hook,
            stage_stdat_metadata_lookup_hook,
            end_flat_stage_id_gate_hook,
            end_four_plate_gate_hook,
            end_battle_decoration_decision_hook
        );
    }
    skyline::println!(
        "[stagedisp] runtime-data cache/class, stage-class dispatch, donor predicate, .stdat scan, and End form predicates hooked at \
         {STAGE_DATA_CACHE_GATE:#x}/{STAGE_DATA_FACTORY_DISPATCH:#x}/{STAGE_FACTORY_DISPATCH:#x}/{STAGE_DATA_DONOR_GATE:#x}/{STAGE_STDAT_METADATA_LOOKUP:#x}/\
         {END_FLAT_STAGE_ID_GATE:#x}/{END_FOUR_PLATE_GATE:#x}/{END_BATTLE_DECORATION_DECISION:#x}; \
         source data remains minted; {} dispatch site(s) surveyed, none patched",
        STAGE_SWITCH_COUNT + UNEXTENDED_SWITCHES,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: usize = 0x8004000000;

    fn switch_at(cmp_at: usize) -> &'static StageSwitch {
        STAGE_SWITCHES
            .iter()
            .find(|s| s.cmp_at == cmp_at)
            .expect("census site")
    }

    fn image(switch: &StageSwitch) -> (impl Fn(usize) -> i32, impl Fn(usize) -> u32 + '_) {
        let base = TEXT + switch.jump_table;
        let read_entry = move |address: usize| ((address - base) / 4 * 0x10) as i32;
        let read_word = move |address: usize| {
            if address == TEXT + switch.cmp_at {
                switch.cmp_opcode
            } else if address == TEXT + switch.adrp_at {
                switch.adrp_opcode
            } else {
                switch.add_opcode
            }
        };
        (read_entry, read_word)
    }

    #[test]
    fn the_census_holds_only_flat_stage_id_tables() {
        assert_eq!(STAGE_SWITCHES.len(), STAGE_SWITCH_COUNT);
        assert_eq!(STAGE_SWITCH_COUNT, 4);
        assert!(STAGE_SWITCHES.iter().any(|s| s.cmp_at == 0x2633d10));
        for switch in STAGE_SWITCHES {
            assert!(switch.entries <= 364, "{:#x}", switch.cmp_at);
            assert_eq!((switch.new_cmp_opcode >> 10) & 0xFFF, EXPANDED_ENTRIES as u32 - 1);
            assert_eq!(
                switch.cmp_opcode & 0xFF80_03FF,
                switch.new_cmp_opcode & 0xFF80_03FF
            );
        }
    }

    #[test]
    fn factory_hook_intercepts_only_the_dispatch_register() {
        let switch = switch_at(STAGE_FACTORY_DISPATCH);
        assert_eq!(switch.cmp_at, STAGE_FACTORY_DISPATCH);
        assert_eq!(switch.cmp_opcode, STAGE_FACTORY_DISPATCH_OPCODE);
        assert_ne!(STAGE_FACTORY_DISPATCH, 0x2633c90);
    }

    #[test]
    fn data_factory_hook_intercepts_only_the_dispatch_register() {
        let switch = switch_at(STAGE_DATA_FACTORY_DISPATCH);
        assert_eq!(switch.cmp_at, STAGE_DATA_FACTORY_DISPATCH);
        assert_eq!(switch.cmp_opcode, STAGE_DATA_FACTORY_DISPATCH_OPCODE);
        assert_eq!(switch.jump_table, 0x44fd898);
        assert_eq!(switch.default_target, 0x24135dc);
        assert_eq!(STAGE_DATA_FACTORY_DISPATCH, 0x240ef9c);
    }

    #[test]
    fn runtime_data_cache_gate_preserves_native_replacement_lifecycle() {
        assert_eq!(STAGE_DATA_CACHE_GATE, 0x25e3c80);
        assert_eq!(STAGE_DATA_CACHE_GATE_OPCODE, 0xb5000168);
        assert!(STAGE_DATA_CACHE_GATE < 0x25e3c8c);
        assert!(STAGE_DATA_CACHE_GATE < 0x25e3c98);
    }

    #[test]
    fn runtime_data_identity_gate_intercepts_only_the_loaded_id() {
        assert_eq!(STAGE_DATA_DONOR_GATE, 0x245c810);
        assert_eq!(STAGE_DATA_DONOR_GATE_OPCODE, 0x7103451f);
        assert_ne!(STAGE_DATA_DONOR_GATE, 0x245c804);
    }

    #[test]
    fn stdat_metadata_bridge_runs_after_the_widened_bound() {
        assert_eq!(STAGE_STDAT_METADATA_LOOKUP, 0x25ff924);
        assert!(crate::stage_bounds::STAGE_BOUNDS
            .iter()
            .any(|bound| bound.address == 0x25ff954
                && bound.old_value == 26208
                && bound.new_value == 36864));
        assert_eq!(STAGE_STDAT_METADATA_LOOKUP_OPCODE, 0x5280090a);
        assert_ne!(STAGE_STDAT_METADATA_LOOKUP, 0x25ff91c);
    }

    #[test]
    fn entries_are_rebased_so_every_case_lands_where_it_did() {
        let switch = switch_at(0x2633d10);
        let (read_entry, read_word) = image(switch);
        let new_base = aligned_base(TEXT + 0x1000_0000, (TEXT + switch.jump_table) & 0xFFF);
        let plan = plan(switch, new_base, TEXT, read_entry, read_word).unwrap();
        let old_base = TEXT + switch.jump_table;
        for index in 0..switch.entries {
            assert_eq!(
                plan.target(index),
                old_base + index * 0x10,
                "case {index} moved"
            );
        }
    }

    #[test]
    fn everything_past_the_vanilla_cases_takes_the_default() {
        let switch = switch_at(0x2633d10);
        let (read_entry, read_word) = image(switch);
        let new_base = aligned_base(TEXT + 0x1000_0000, (TEXT + switch.jump_table) & 0xFFF);
        let plan = plan(switch, new_base, TEXT, read_entry, read_word).unwrap();
        for index in switch.entries..EXPANDED_ENTRIES {
            assert_eq!(plan.target(index), TEXT + switch.default_target);
        }
    }

    #[test]
    fn a_minted_id_takes_its_donors_case() {
        let switch = switch_at(0x2633d10);
        let (read_entry, read_word) = image(switch);
        let new_base = aligned_base(TEXT + 0x1000_0000, (TEXT + switch.jump_table) & 0xFFF);
        let mut plan = plan(switch, new_base, TEXT, read_entry, read_word).unwrap();
        set_donor(&mut plan, switch, 365, 8).unwrap();
        assert_eq!(plan.target(365), plan.target(8));
        assert_eq!(plan.target(364), TEXT + switch.default_target);
        assert_eq!(plan.target(366), TEXT + switch.default_target);
    }

    #[test]
    fn a_minted_donor_is_refused() {
        let switch = switch_at(0x2633d10);
        let (read_entry, read_word) = image(switch);
        let mut plan = plan(
            switch,
            aligned_base(TEXT + 0x1000_0000, (TEXT + switch.jump_table) & 0xFFF),
            TEXT,
            read_entry,
            read_word,
        )
        .unwrap();
        assert_eq!(
            set_donor(&mut plan, switch, 400, 380),
            Err(DispatchError::DonorOutOfRange {
                donor: 380,
                entries: switch.entries
            })
        );
    }

    #[test]
    fn a_table_further_than_an_i32_offset_is_refused() {
        let switch = switch_at(0x2633d10);
        let (read_entry, read_word) = image(switch);
        let far = aligned_base(TEXT + 0xC000_0000, (TEXT + switch.jump_table) & 0xFFF);
        assert!(matches!(
            plan(switch, far, TEXT, read_entry, read_word),
            Err(DispatchError::TargetOutOfReach { .. })
        ));
    }

    #[test]
    fn a_changed_site_stops_the_pass() {
        let switch = switch_at(0x2633d10);
        let (read_entry, _) = image(switch);
        let read_word = |address: usize| {
            if address == TEXT + switch.cmp_at {
                0xDEAD_BEEF
            } else if address == TEXT + switch.adrp_at {
                switch.adrp_opcode
            } else {
                switch.add_opcode
            }
        };
        assert_eq!(
            plan(
                switch,
                aligned_base(TEXT + 0x1000_0000, (TEXT + switch.jump_table) & 0xFFF),
                TEXT,
                read_entry,
                read_word
            ),
            Err(DispatchError::OpcodeMismatch {
                address: TEXT + switch.cmp_at,
                expected: switch.cmp_opcode,
                actual: 0xDEAD_BEEF,
            })
        );
    }

    #[test]
    fn the_swap_is_one_store_because_the_add_never_changes() {
        for switch in STAGE_SWITCHES {
            let old_base = TEXT + switch.jump_table;
            let base = aligned_base(TEXT + 0x1000_0000, old_base & 0xFFF);
            assert_eq!(base & 0xFFF, old_base & 0xFFF);
            assert!(base >= TEXT + 0x1000_0000);
            assert_eq!(
                encode_add(base, switch.base_register),
                switch.add_opcode,
                "{:#x} would need its ADD rewritten",
                switch.cmp_at
            );
        }
    }

    #[test]
    fn a_table_that_is_not_page_congruent_is_refused() {
        let switch = switch_at(0x2633d10);
        let (read_entry, read_word) = image(switch);
        let skewed = aligned_base(TEXT + 0x1000_0000, (TEXT + switch.jump_table) & 0xFFF) + 4;
        assert!(matches!(
            plan(switch, skewed, TEXT, read_entry, read_word),
            Err(DispatchError::NotPageCongruent { .. })
        ));
    }

    #[test]
    fn every_table_fits_the_reserved_block() {
        assert_eq!(required_bytes(), STAGE_SWITCHES.len() * (0x1000 + EXPANDED_ENTRIES * 4));
        assert_eq!(required_bytes(), 4 * (0x1000 + 2048));
    }
}
