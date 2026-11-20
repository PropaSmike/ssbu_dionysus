use crate::game_reloc_13_0_5::{OPCODES, TEXT_RANGES};
use core::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

const CONFIG_DIRECTORY: &str = "sd:/ultimate/dionysus";
const CONFIG_PATH: &str = "sd:/ultimate/dionysus/config.toml";
const DEFAULT_CONFIG: &str = "game_version = \"auto\"\nmodes_button = \"x\"\n";
const OLD_TEXT_END: usize = 0x39c_7e90;
const OLD_DATA_START: usize = 0x39c_8000;
const DATA_DELTA_13_0_5: usize = 0x1000;
const TEXT_TAIL_DELTA_13_0_5: usize = 0x5b0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GameVersion {
    V13_0_4,
    V13_0_5,
}

impl GameVersion {
    pub(crate) fn name(self) -> &'static str {
        match self {
            GameVersion::V13_0_4 => "13.0.4",
            GameVersion::V13_0_5 => "13.0.5",
        }
    }

    fn rodata_offset(self) -> usize {
        match self {
            GameVersion::V13_0_4 => 0x39c_8000,
            GameVersion::V13_0_5 => 0x39c_9000,
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "13.0.4" => Some(GameVersion::V13_0_4),
            "13.0.5" => Some(GameVersion::V13_0_5),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Setting {
    Auto,
    Fixed(GameVersion),
}

static VERSION: OnceLock<GameVersion> = OnceLock::new();
static TEXT_BASE: OnceLock<usize> = OnceLock::new();
static UNMAPPED_REPORTS: AtomicU32 = AtomicU32::new(0);

pub(crate) fn raw_text_base() -> usize {
    *TEXT_BASE.get_or_init(|| unsafe {
        skyline::hooks::getRegionAddress(skyline::hooks::Region::Text) as usize
    })
}

pub(crate) fn version() -> GameVersion {
    *VERSION.get_or_init(resolve)
}

pub(crate) fn detected() -> Option<GameVersion> {
    let text = raw_text_base();
    let rodata = unsafe { skyline::hooks::getRegionAddress(skyline::hooks::Region::Rodata) as usize };
    let layout = rodata.wrapping_sub(text);
    [GameVersion::V13_0_4, GameVersion::V13_0_5]
        .into_iter()
        .find(|version| version.rodata_offset() == layout)
}

fn parse_setting(text: &str) -> Option<Setting> {
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "game_version" {
            continue;
        }
        let value = value.trim().trim_matches('"').trim();
        if value.eq_ignore_ascii_case("auto") {
            return Some(Setting::Auto);
        }
        return GameVersion::parse(value).map(Setting::Fixed);
    }
    Some(Setting::Auto)
}

fn configured() -> Setting {
    crate::upgrade::carry_old_folder();
    match std::fs::read_to_string(CONFIG_PATH) {
        Ok(text) => parse_setting(&text).unwrap_or_else(|| {
            skyline::println!(
                "[game] {CONFIG_PATH}: game_version must be \"auto\", \"13.0.4\" or \"13.0.5\"; using auto"
            );
            Setting::Auto
        }),
        Err(_) => {
            let _ = std::fs::create_dir_all(CONFIG_DIRECTORY);
            let _ = std::fs::write(CONFIG_PATH, DEFAULT_CONFIG);
            Setting::Auto
        }
    }
}

fn resolve() -> GameVersion {
    let setting = configured();
    let detected = detected();
    let chosen = match (setting, detected) {
        (Setting::Fixed(wanted), Some(running)) if wanted != running => {
            skyline::println!(
                "[game] config.toml asks for SSBU {} but the running game is {}; using {}",
                wanted.name(),
                running.name(),
                running.name()
            );
            running
        }
        (Setting::Fixed(wanted), _) => wanted,
        (Setting::Auto, Some(running)) => running,
        (Setting::Auto, None) => {
            skyline::println!(
                "[game] unrecognised main layout and game_version = \"auto\"; assuming SSBU 13.0.4, set game_version in {CONFIG_PATH} to choose"
            );
            GameVersion::V13_0_4
        }
    };
    skyline::println!(
        "[game] SSBU {} (config {}, detected {})",
        chosen.name(),
        match setting {
            Setting::Auto => "auto",
            Setting::Fixed(version) => version.name(),
        },
        detected.map(GameVersion::name).unwrap_or("unknown")
    );
    chosen
}

fn report_unmapped(offset: usize) {
    if UNMAPPED_REPORTS.fetch_add(1, Ordering::Relaxed) < 16 {
        skyline::println!(
            "[game] 13.0.4 offset {offset:#x} has no 13.0.5 equivalent (changed code); using the nearest range"
        );
    }
}

fn rel_13_0_5(offset: usize) -> usize {
    if offset >= OLD_DATA_START {
        return offset + DATA_DELTA_13_0_5;
    }
    if offset >= OLD_TEXT_END {
        return offset + TEXT_TAIL_DELTA_13_0_5;
    }
    let index = TEXT_RANGES.partition_point(|range| range.0 as usize <= offset);
    if index == 0 {
        report_unmapped(offset);
        return offset;
    }
    let (_, end, delta) = TEXT_RANGES[index - 1];
    if offset >= end as usize {
        report_unmapped(offset);
    }
    (offset as isize + delta as isize) as usize
}

fn unrel_13_0_5(offset: usize) -> usize {
    if offset >= OLD_DATA_START + DATA_DELTA_13_0_5 {
        return offset - DATA_DELTA_13_0_5;
    }
    if offset >= OLD_TEXT_END + TEXT_TAIL_DELTA_13_0_5 {
        return offset - TEXT_TAIL_DELTA_13_0_5;
    }
    let index = TEXT_RANGES
        .partition_point(|range| (range.0 as isize + range.2 as isize) as usize <= offset);
    if index == 0 {
        return offset;
    }
    let delta = TEXT_RANGES[index - 1].2;
    (offset as isize - delta as isize) as usize
}

pub(crate) fn rel(offset: usize) -> usize {
    match version() {
        GameVersion::V13_0_4 => offset,
        GameVersion::V13_0_5 => rel_13_0_5(offset),
    }
}

pub(crate) fn unrel(offset: usize) -> usize {
    match version() {
        GameVersion::V13_0_4 => offset,
        GameVersion::V13_0_5 => unrel_13_0_5(offset),
    }
}

pub(crate) fn addr(offset: usize) -> usize {
    raw_text_base() + rel(offset)
}

pub(crate) fn offset_of(address: usize) -> usize {
    unrel(address.wrapping_sub(raw_text_base()))
}

fn opcode_entry(offset: usize) -> Option<(u32, u32)> {
    let index = OPCODES.partition_point(|entry| (entry.0 as usize) < offset);
    match OPCODES.get(index) {
        Some(&(at, old, new)) if at as usize == offset => Some((old, new)),
        _ => None,
    }
}

fn sign_extend(value: u32, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((value as i64) << shift) >> shift
}

fn branch_field(op: u32) -> Option<(u32, u32, i64)> {
    if op & 0x7C00_0000 == 0x1400_0000 {
        return Some((0, 26, 4));
    }
    if op & 0xFF00_0010 == 0x5400_0000
        || op & 0x7E00_0000 == 0x3400_0000
        || op & 0x3B00_0000 == 0x1800_0000
    {
        return Some((5, 19, 4));
    }
    if op & 0x7E00_0000 == 0x3600_0000 {
        return Some((5, 14, 4));
    }
    None
}

fn adr_target(op: u32, pc: usize) -> Option<(bool, i64)> {
    if op & 0x1F00_0000 != 0x1000_0000 {
        return None;
    }
    let page = op & 0x8000_0000 != 0;
    let imm = sign_extend(((op >> 5) & 0x7_FFFF) << 2 | ((op >> 29) & 0b11), 21);
    let target = if page {
        ((pc as i64) & !0xFFF) + (imm << 12)
    } else {
        pc as i64 + imm
    };
    Some((page, target))
}

fn encode_adr(op: u32, pc: usize, target: i64, page: bool) -> Option<u32> {
    let imm = if page {
        (target >> 12) - ((pc as i64) >> 12)
    } else {
        target - pc as i64
    };
    if !(-(1 << 20)..(1 << 20)).contains(&imm) {
        return None;
    }
    let imm = imm as u32 & 0x1F_FFFF;
    Some(op & 0x9F00_001F | (imm & 0b11) << 29 | (imm >> 2) << 5)
}

fn translate(op: u32, from_pc: usize, to_pc: usize, map: fn(usize) -> usize) -> Option<u32> {
    if let Some((shift, bits, scale)) = branch_field(op) {
        let field = (op >> shift) & ((1 << bits) - 1);
        let target = from_pc as i64 + sign_extend(field, bits) * scale;
        let moved = map(target as usize) as i64;
        let imm = (moved - to_pc as i64) / scale;
        if (moved - to_pc as i64) % scale != 0 || !(-(1 << (bits - 1))..(1 << (bits - 1))).contains(&imm) {
            return None;
        }
        let mask = ((1u32 << bits) - 1) << shift;
        return Some(op & !mask | ((imm as u32) << shift & mask));
    }
    let (page, target) = adr_target(op, from_pc)?;
    let moved = map(target as usize) as i64;
    encode_adr(op, to_pc, moved, page)
}

pub(crate) fn opcode(offset: usize, opcode: u32) -> u32 {
    match version() {
        GameVersion::V13_0_4 => opcode,
        GameVersion::V13_0_5 => match opcode_entry(offset) {
            Some((old, new)) if old == opcode => new,
            _ => translate(opcode, offset, rel_13_0_5(offset), rel_13_0_5).unwrap_or(opcode),
        },
    }
}

const NRO_MAGIC: u32 = 0x304F_524E;
const NRO_MAGIC_AT: usize = 0x10;
const NRO_SIZE_AT: usize = 0x18;
const MODULE_PAGE: usize = 0x1000;
const MAX_MODULE_PAGES: usize = 0x2000;

pub(crate) unsafe fn module_range(address: usize) -> (usize, usize) {
    let mut page = address & !(MODULE_PAGE - 1);
    for _ in 0..MAX_MODULE_PAGES {
        if core::ptr::read_volatile((page + NRO_MAGIC_AT) as *const u32) == NRO_MAGIC {
            let size = core::ptr::read_volatile((page + NRO_SIZE_AT) as *const u32) as usize;
            return (page, page + size);
        }
        page -= MODULE_PAGE;
    }
    (0, 0)
}

pub(crate) unsafe fn text_word(offset: usize) -> u32 {
    let word = core::ptr::read_volatile(addr(offset) as *const u32);
    match version() {
        GameVersion::V13_0_4 => word,
        GameVersion::V13_0_5 => match opcode_entry(offset) {
            Some((old, new)) if new == word => old,
            _ => translate(word, rel_13_0_5(offset), offset, unrel_13_0_5).unwrap_or(word),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_parses() {
        assert_eq!(parse_setting("game_version = \"13.0.5\"\n"), Some(Setting::Fixed(GameVersion::V13_0_5)));
        assert_eq!(parse_setting("game_version = \"auto\""), Some(Setting::Auto));
        assert_eq!(parse_setting("other = 1"), Some(Setting::Auto));
        assert_eq!(parse_setting("game_version = \"13.0.3\""), None);
    }

    #[test]
    fn ranges_are_sorted_and_round_trip() {
        for pair in TEXT_RANGES.windows(2) {
            assert!(pair[0].1 <= pair[1].0);
            assert!(pair[0].1 as i64 + pair[0].2 as i64 <= pair[1].0 as i64 + pair[1].2 as i64);
        }
        for &(start, end, _) in TEXT_RANGES.iter() {
            for offset in [start as usize, end as usize - 4] {
                assert_eq!(unrel_13_0_5(rel_13_0_5(offset)), offset);
            }
        }
        assert_eq!(rel_13_0_5(0x52c45d0), 0x52c55d0);
        assert_eq!(unrel_13_0_5(0x52c55d0), 0x52c45d0);
    }
}
