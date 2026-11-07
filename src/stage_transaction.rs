use crate::stage_bounds::{StageTable, STAGE_TABLES};
#[cfg(feature = "stage_mint_places")]
use crate::stage_relocation::{RelocationPlan, TableReference};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

const CANARY_LEN: usize = 0x40;
const CANARY_WORD: u64 = 0x5354_4147_4543_414E;

static DONE: AtomicBool = AtomicBool::new(false);
static BLOCK: AtomicUsize = AtomicUsize::new(0);
static BLOCK_LEN: AtomicUsize = AtomicUsize::new(0);
static FOREIGN_BLOCK: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn foreign_stage_id_block() -> Option<usize> {
    match FOREIGN_BLOCK.load(Ordering::Acquire) {
        0 => None,
        base => Some(base),
    }
}

#[cfg(feature = "stage_mint")]
pub(crate) unsafe fn retake_foreign_sites(table: usize) -> (usize, usize) {
    let mut references = Vec::new();
    let mut skipped = 0usize;
    for recorded in crate::stage_bounds::STAGE_REFERENCES {
        if recorded.table != "stage_id" {
            continue;
        }
        let at = crate::game::addr(recorded.adrp_at);
        let first = crate::game::text_word(recorded.adrp_at);
        let second = crate::game::text_word(recorded.adrp_at + 4);
        let recognised = (first, second) == (recorded.adrp_opcode, recorded.add_opcode)
            || crate::stage_backend::is_foreign_patch(first, second);
        if !recognised {
            skipped += 1;
            continue;
        }
        references.push(TableReference {
            adrp_at: at,
            register: recorded.register,
            delta: recorded.delta,
        });
    }
    if references.is_empty() {
        return (0, skipped);
    }
    match RelocationPlan::build(&references, table) {
        Ok(plan) => {
            if !plan_is_safe("owned", &plan) {
                return (0, skipped);
            }
            report_refused("owned", plan.apply(), plan.patches.len());
            let mut landed = 0usize;
            for patch in &plan.patches {
                let site = patch.adrp_at as *const u32;
                if site.read_volatile() == patch.adrp_word
                    && site.add(1).read_volatile() == patch.add_word
                {
                    landed += 1;
                }
            }
            skyline::println!(
                "[stagereloc] retook {} stage_id site(s) from the foreign owner -> {:#x},                  {} verified{}; {} left alone",
                plan.patches.len(),
                table,
                landed,
                if landed == plan.patches.len() { "" } else { "  <-- WRITES DID NOT LAND" },
                skipped,
            );
            (landed, skipped)
        }
        Err(error) => {
            skyline::println!("[stagereloc] cannot retake the stage_id sites: {error:?}");
            (0, skipped)
        }
    }
}

#[cfg(feature = "stage_mint")]
pub(crate) unsafe fn finish_foreign_sites(sites: &[usize], table: usize) -> usize {
    let mut references = Vec::new();
    for &site in sites {
        let Some(recorded) = crate::stage_bounds::STAGE_REFERENCES
            .iter()
            .find(|reference| reference.adrp_at == site)
        else {
            continue;
        };
        let at = crate::game::addr(site);
        let first = crate::game::text_word(site);
        let second = crate::game::text_word(site + 4);
        if (first, second) != (recorded.adrp_opcode, recorded.add_opcode) {
            skyline::println!(
                "[stagereloc] straggler {site:#x} is no longer vanilla ({first:#010x}/{second:#010x}); skipping it"
            );
            continue;
        }
        references.push(TableReference {
            adrp_at: at,
            register: recorded.register,
            delta: recorded.delta,
        });
    }
    if references.is_empty() {
        return 0;
    }
    match RelocationPlan::build(&references, table) {
        Ok(plan) => {
            if !plan_is_safe("stragglers", &plan) {
                return 0;
            }
            report_refused("stragglers", plan.apply(), plan.patches.len());
            skyline::println!(
                "[stagereloc] finished {} site(s) the foreign owner left unpatched -> {:#x}",
                plan.patches.len(),
                table,
            );
            plan.patches.len()
        }
        Err(error) => {
            skyline::println!("[stagereloc] cannot finish the foreign stragglers: {error:?}");
            0
        }
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct MemoryInfo {
    address: u64,
    size: u64,
    kind: u32,
    attribute: u32,
    permission: u32,
    ipc_refcount: u32,
    device_refcount: u32,
    padding: u32,
}

const MEMORY_FREE: u32 = 0x00;

#[cfg(not(target_arch = "aarch64"))]
unsafe fn query_memory(_info: *mut MemoryInfo, _address: u64) -> u32 {
    u32::MAX
}

#[cfg(target_arch = "aarch64")]
unsafe fn query_memory(info: *mut MemoryInfo, address: u64) -> u32 {
    let result: u64;
    core::arch::asm!(
        "svc 0x6",
        inout("x0") info as u64 => result,
        out("x1") _,
        in("x2") address,
        clobber_abi("C"),
        options(nostack),
    );
    result as u32
}

fn required_bytes(tables: &[&str]) -> usize {
    STAGE_TABLES
        .iter()
        .filter(|t| tables.contains(&t.name))
        .map(|t| CANARY_LEN * 2 + t.element_size * t.expanded_length)
        .sum()
}

unsafe fn find_destination(needed: usize) -> Option<(usize, usize)> {
    let text = crate::game::raw_text_base() as u64;
    let mut info = MemoryInfo::default();
    let mut address = text;
    let mut region: Option<(u64, u64)> = None;

    for _ in 0..64 {
        if query_memory(&mut info, address) != 0 || info.size == 0 {
            return None;
        }
        if info.kind != MEMORY_FREE
            && info.permission == 3
            && info.address >= text
            && info.size as usize >= needed
        {
            region = Some((info.address, info.address + info.size));
            break;
        }
        let next = info.address.wrapping_add(info.size);
        if next <= address {
            return None;
        }
        address = next;
    }
    let (start, end) = region?;

    let mut best = (0u64, 0usize);
    let mut run_start = start;
    let mut run_len = 0usize;
    let mut at = start;
    while at + 0x1000 <= end {
        let mut zero = true;
        let mut offset = 0usize;
        while offset < 0x1000 {
            if core::ptr::read_volatile((at as usize + offset) as *const u64) != 0 {
                zero = false;
                break;
            }
            offset += 8;
        }
        if zero {
            if run_len == 0 {
                run_start = at;
            }
            run_len += 0x1000;
            if run_len > best.1 {
                best = (run_start, run_len);
            }
        } else {
            run_len = 0;
        }
        at += 0x1000;
    }
    if best.1 < needed {
        return None;
    }
    let tail = ((best.0 as usize + best.1 - needed) & !0xFFF, needed);
    Some(tail)
}

unsafe fn destination_is_clean(start: usize, len: usize) -> bool {
    let mut offset = 0usize;
    while offset < len {
        if core::ptr::read_volatile((start + offset) as *const u64) != 0 {
            return false;
        }
        offset += 8;
    }
    true
}

unsafe fn write_canary(at: usize) {
    let mut offset = 0usize;
    while offset < CANARY_LEN {
        core::ptr::write_volatile((at + offset) as *mut u64, CANARY_WORD);
        offset += 8;
    }
}

pub(crate) unsafe fn canary_intact(at: usize) -> bool {
    let mut offset = 0usize;
    while offset < CANARY_LEN {
        if core::ptr::read_volatile((at + offset) as *const u64) != CANARY_WORD {
            return false;
        }
        offset += 8;
    }
    true
}

unsafe fn report_refused(label: &str, refused: usize, total: usize) {
    if refused != 0 {
        skyline::println!(
            "[stagereloc] {} could not write {} of {} site(s); .text stayed read-only",
            label,
            refused,
            total
        );
    }
}

unsafe fn text_span() -> (usize, usize) {
    let base = crate::game::raw_text_base();
    let end = skyline::hooks::getRegionAddress(skyline::hooks::Region::Rodata) as usize;
    (base, end)
}

unsafe fn plan_is_safe(label: &str, plan: &RelocationPlan) -> bool {
    let (base, end) = text_span();
    if base == 0 || end <= base {
        skyline::println!(
            "[stagereloc] REFUSED {}: .text reads {:#x}..{:#x}; nothing written",
            label,
            base,
            end
        );
        return false;
    }
    for (index, patch) in plan.patches.iter().enumerate() {
        if patch.adrp_at < base || patch.adrp_at.saturating_add(8) > end {
            skyline::println!(
                "[stagereloc] REFUSED {}: site {} of {} is {:#x}, outside .text {:#x}..{:#x}; nothing written",
                label,
                index,
                plan.patches.len(),
                patch.adrp_at,
                base,
                end
            );
            return false;
        }
    }
    skyline::println!(
        "[stagereloc] {} plan: {} site(s), first {:#x}, table {:#x}, .text {:#x}..{:#x}",
        label,
        plan.patches.len(),
        plan.patches.first().map(|patch| patch.adrp_at).unwrap_or(0),
        plan.table_pointer,
        base,
        end
    );
    true
}

unsafe fn install_table(table: &StageTable, cursor: usize) -> (usize, usize) {
    write_canary(cursor);
    let base = cursor + CANARY_LEN;
    let native = table.element_size * table.native_length;
    core::ptr::copy_nonoverlapping(
        crate::game::addr(table.address) as *const u8,
        base as *mut u8,
        native,
    );
    let end = base + table.element_size * table.expanded_length;
    write_canary(end);
    (base, end + CANARY_LEN)
}

pub(crate) unsafe fn try_relocate() {
    if DONE.swap(true, Ordering::AcqRel) {
        return;
    }

    let plan = crate::stage_backend::arm();
    crate::stage_backend::report(plan);

    #[cfg(feature = "stage_mint_places")]
    relocate_owned(plan);

    #[cfg(feature = "stage_mint")]
    crate::stage_registry::install_stage_id_backend();

    #[cfg(not(feature = "stage_mint_places"))]
    skyline::println!(
        "[stagereloc] observed only: this build relocates no table and mints no stage. \
         Rebuild with --features stage_mint_places (places) or stage_mint (everything)."
    );
    #[cfg(all(feature = "stage_mint_places", not(feature = "stage_mint")))]
    skyline::println!(
        "[stagereloc] places only: this build does not touch the stage_id table, so the \
         registry has no stage ids to hand out and nothing will be minted"
    );

    apply_select_cap("late");
}

static RELOCATED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

pub(crate) fn relocated_tables() -> Vec<&'static str> {
    RELOCATED
        .lock()
        .map(|names| names.clone())
        .unwrap_or_default()
}

pub(crate) fn table_base(name: &str) -> Option<usize> {
    let block = BLOCK.load(Ordering::Acquire);
    if block == 0 {
        return None;
    }
    let relocated = relocated_tables();
    let mut cursor = block;
    for table in STAGE_TABLES {
        if !relocated.contains(&table.name) {
            continue;
        }
        let base = cursor + CANARY_LEN;
        if table.name == name {
            return Some(base);
        }
        cursor = base + table.element_size * table.expanded_length + CANARY_LEN;
    }
    None
}

#[cfg(feature = "stage_mint_places")]
unsafe fn relocate_owned(plan: &crate::stage_backend::Plan) {
    use crate::stage_backend::Ownership;

    let owned: Vec<&'static str> = plan
        .verdicts
        .iter()
        .filter(|verdict| verdict.ownership == Ownership::Vanilla)
        .map(|verdict| verdict.table)
        .collect();

    let extend_foreign = plan.stage_ids == crate::stage_backend::StageIdBackend::ExtendForeign;
    if owned.is_empty() && !extend_foreign {
        skyline::println!("[stagereloc] no stage table is ours to relocate; nothing written");
        return;
    }

    let foreign_bytes = if extend_foreign {
        STAGE_TABLES
            .iter()
            .find(|table| table.name == "stage_id")
            .map(|table| CANARY_LEN * 2 + table.element_size * table.expanded_length)
            .unwrap_or(0)
    } else {
        0
    };
    let needed = required_bytes(&owned) + foreign_bytes;
    let Some((block, length)) = find_destination(needed) else {
        skyline::println!(
            "[stagereloc] no {needed}-byte zero run inside ADRP reach of .text; not relocating"
        );
        return;
    };
    if !destination_is_clean(block, length) {
        skyline::println!("[stagereloc] destination {block:#x} is not zero; not relocating");
        return;
    }

    for reference in crate::stage_bounds::STAGE_REFERENCES {
        if !owned.contains(&reference.table) {
            continue;
        }
        let first = crate::game::text_word(reference.adrp_at);
        let second = crate::game::text_word(reference.adrp_at + 4);
        if (first, second) != (reference.adrp_opcode, reference.add_opcode) {
            skyline::println!(
                "[stagereloc] {} site {:#x} changed under us ({:#010x}/{:#010x}); aborting with \
                 nothing written",
                reference.table,
                reference.adrp_at,
                first,
                second
            );
            return;
        }
    }

    let mut cursor = block;
    let mut bases = Vec::new();
    for table in STAGE_TABLES {
        if !owned.contains(&table.name) {
            continue;
        }
        let (base, next) = install_table(table, cursor);
        bases.push((table.name, base));
        cursor = next;
    }

    let mut plans = Vec::new();
    for (name, base) in &bases {
        let references: Vec<TableReference> = crate::stage_bounds::STAGE_REFERENCES
            .iter()
            .filter(|reference| reference.table == *name)
            .map(|reference| TableReference {
                adrp_at: crate::game::addr(reference.adrp_at),
                register: reference.register,
                delta: reference.delta,
            })
            .collect();
        match RelocationPlan::build(&references, *base) {
            Ok(built) => plans.push((*name, built)),
            Err(error) => {
                skyline::println!(
                    "[stagereloc] cannot plan {name}: {error:?}; aborting with nothing written"
                );
                return;
            }
        }
    }

    for (name, built) in &plans {
        if !plan_is_safe(name, built) {
            return;
        }
    }

    for (name, built) in &plans {
        report_refused(name, built.apply(), built.patches.len());
        let mut landed = 0usize;
        for patch in &built.patches {
            let site = patch.adrp_at as *const u32;
            if site.read_volatile() == patch.adrp_word
                && site.add(1).read_volatile() == patch.add_word
            {
                landed += 1;
            }
        }
        skyline::println!(
            "[stagereloc] {} -> {:#x}, {} sites rewritten, {} verified{}",
            name,
            built.table_pointer,
            built.patches.len(),
            landed,
            if landed == built.patches.len() {
                ""
            } else {
                "  <-- WRITES DID NOT LAND"
            },
        );
    }

    if foreign_bytes != 0 {
        if let Some(table) = STAGE_TABLES.iter().find(|table| table.name == "stage_id") {
            let (base, _end) = install_table(table, cursor);
            FOREIGN_BLOCK.store(base, Ordering::Release);
            skyline::println!(
                "[stagereloc] reserved {:#x} for the foreign stage_id table ({} rows)",
                base,
                table.expanded_length,
            );
        }
    }

    BLOCK.store(block, Ordering::Release);
    BLOCK_LEN.store(length, Ordering::Release);
    if let Ok(mut names) = RELOCATED.lock() {
        *names = owned.clone();
    }

    if let Some((_, base)) = bases.iter().find(|(name, _)| *name == "stage_place") {
        let name_hash = core::ptr::read_volatile((base + 0x08) as *const u64);
        skyline::println!(
            "[stagereloc] readback: stage_place[0] name hash {:#x} (expect {:#x})",
            name_hash,
            crate::stage_ledger::hash40("battlefield"),
        );
    }

    widen_bounds(&owned);
}

#[cfg(feature = "stage_mint_places")]
pub(crate) unsafe fn widen_bounds(tables: &[&str]) {
    let read = |offset: usize| crate::game::text_word(offset);
    let bounds = crate::stage_bounds::bounds_for(tables);
    if bounds.is_empty() {
        return;
    }
    match crate::stage_bounds::plan_widening(&bounds, tables, read) {
        Ok(patches) => {
            let mut landed = 0usize;
            for patch in &patches {
                crate::text_patch::write_word(crate::game::addr(patch.address), patch.word);
            }
            for patch in &patches {
                if core::ptr::read_volatile((crate::game::addr(patch.address)) as *const u32) == patch.word {
                    landed += 1;
                }
            }
            skyline::println!(
                "[stagereloc] widened {} bound(s) for {:?}, {} verified{}; {} sites in the census stay narrow",
                patches.len(),
                tables,
                landed,
                if landed == patches.len() { "" } else { "  <-- WRITES DID NOT LAND" },
                crate::stage_bounds::unwidened_count(),
            );
            if landed != patches.len() {
                for patch in patches
                    .iter()
                    .filter(|patch| {
                        core::ptr::read_volatile((crate::game::addr(patch.address)) as *const u32) != patch.word
                    })
                    .take(8)
                {
                    skyline::println!(
                        "[stagereloc]   {:#x} still holds {:#010x}, wanted {:#010x}",
                        patch.address,
                        core::ptr::read_volatile((crate::game::addr(patch.address)) as *const u32),
                        patch.word,
                    );
                }
            }
        }
        Err(error) => skyline::println!("[stagereloc] bounds NOT widened: {error:?}"),
    }
}

pub(crate) fn apply_select_cap(phase: &str) {
    use crate::stage_select_cap::{plan, TARGET_CAP, VANILLA_CAP};
    let read = |offset: usize| unsafe { crate::game::text_word(offset) };

    match plan(TARGET_CAP, read) {
        Ok(patches) => {
            for (offset, word) in &patches {
                unsafe {
                    crate::text_patch::write_word(crate::game::addr(*offset), *word);
                }
            }
            let live = patches
                .iter()
                .filter(|(offset, word)| read(*offset) == *word)
                .count();
            skyline::println!(
                "[stagecap] {phase}: raised the stage-select cap {} -> {} at {} site(s); \
                 {live} of {} verified live by read-back",
                VANILLA_CAP,
                TARGET_CAP,
                patches.len(),
                patches.len()
            );
            if live != patches.len() {
                for (offset, word) in &patches {
                    let actual = read(*offset);
                    if actual != *word {
                        skyline::println!(
                            "[stagecap] site {offset:#x} did NOT take: wanted {word:#010x}, \
                             memory holds {actual:#010x}"
                        );
                    }
                }
            }
        }
        Err(error) => skyline::println!("[stagecap] NOT applied: {:?}", error),
    }
}

pub(crate) unsafe fn verify_canaries() {
    let block = BLOCK.load(Ordering::Acquire);
    if block == 0 {
        return;
    }
    let relocated = relocated_tables();
    let mut cursor = block;
    for table in STAGE_TABLES {
        if !relocated.contains(&table.name) {
            continue;
        }
        let head = cursor;
        let tail = cursor + CANARY_LEN + table.element_size * table.expanded_length;
        if !canary_intact(head) || !canary_intact(tail) {
            skyline::println!(
                "[stagereloc] CANARY BROKEN around {} (head {:#x} tail {:#x}) - something else owns this memory",
                table.name,
                head,
                tail
            );
            return;
        }
        cursor = tail + CANARY_LEN;
    }
}
