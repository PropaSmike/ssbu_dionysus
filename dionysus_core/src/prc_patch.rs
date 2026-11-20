use crate::hash::hash40;
use crate::prc::{self, Param, PrcError};
use crate::prcxml;

const PATCHES: [(&str, &str); 6] = [
    (".prcxml", ".prc"),
    (".prcx", ".prc"),
    (".stprmxml", ".stprm"),
    (".stprmx", ".stprm"),
    (".stdatxml", ".stdat"),
    (".stdatx", ".stdat"),
];

const ITEM_TOPS: [&str; 4] = ["item", "assist", "pokemon", "boss"];

const STAGE_FILES: [&str; 4] = [
    prcxml::STAGE_FILE,
    prcxml::STAGE_SOUND_FILE,
    prcxml::STAGE_AREA_FILE,
    prcxml::ASSET_FILE,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub at: String,
    pub file: &'static str,
    pub patch: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    Values(usize),
    Mismatch(Mismatch),
}

pub fn is_param_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    PATCHES.iter().any(|(_, base)| lower.ends_with(base))
}

pub fn base_of(patch: &str) -> Option<String> {
    let lower = patch.to_ascii_lowercase();
    let (extension, base) = PATCHES.iter().find(|(extension, _)| lower.ends_with(extension))?;
    let stem = &lower[..lower.len() - extension.len()];
    let name = stem.rsplit('/').next().unwrap_or(stem);
    if name.is_empty() || name.contains('+') {
        return None;
    }
    Some(format!("{stem}{base}"))
}

pub fn patch_names(base_name: &str) -> Vec<String> {
    let lower = base_name.to_ascii_lowercase();
    PATCHES
        .iter()
        .filter_map(|(extension, base)| lower.strip_suffix(base).map(|stem| format!("{stem}{extension}")))
        .collect()
}

pub fn read_by_dionysus(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split('/').collect();
    let Some(stem) = parts.last().and_then(|file| file.strip_suffix(".prcxml")) else {
        return false;
    };
    match parts.as_slice() {
        ["fighter", _, "param", _] => stem == prcxml::VL_FILE || prcxml::FIGHTER_FILES.contains(&stem),
        ["fighter", _, "ai", "param", _] => stem == prcxml::CPU_FILE,
        [top, _, "param", _] if ITEM_TOPS.contains(top) => stem == prcxml::ITEM_FILE || stem == prcxml::ASSET_FILE,
        ["enemy", _, _] => stem == prcxml::ENEMY_FILE,
        ["enemy", _, "param", _] => stem == prcxml::ASSET_FILE,
        ["stage", _, _, "param", _] => STAGE_FILES.contains(&stem),
        ["spirit", _] => true,
        _ => false,
    }
}

pub fn path_hash(path: &str) -> u64 {
    prcxml::key(path)
}

pub fn read_patch(bytes: &[u8]) -> Result<Param, String> {
    if bytes.starts_with(prc::MAGIC) {
        return match prc::parse(bytes) {
            Ok(Param::Struct(members)) => Ok(Param::Struct(members)),
            Ok(_) | Err(PrcError::RootNotStruct) => Err("its top is not a struct".to_string()),
            Err(error) => Err(format!("it is a damaged param file ({error:?})")),
        };
    }
    let text = core::str::from_utf8(bytes).map_err(|_| "it is neither a param file nor UTF-8 text".to_string())?;
    prcxml::parse_param(text).map_err(|error| format!("line {}: {}", error.line, error.message))
}

pub fn apply(patch: &Param, source: &mut Param) -> Result<usize, Mismatch> {
    let mut working = source.clone();
    let mut trail = String::new();
    let count = merge(patch, &mut working, &mut trail)?;
    *source = working;
    Ok(count)
}

pub fn patch_file(source: &[u8], patches: &[Param]) -> Result<(Vec<u8>, Vec<Applied>), PrcError> {
    let mut root = prc::parse(source)?;
    let outcomes: Vec<Applied> = patches
        .iter()
        .map(|patch| match apply(patch, &mut root) {
            Ok(count) => Applied::Values(count),
            Err(mismatch) => Applied::Mismatch(mismatch),
        })
        .collect();
    let changed = outcomes.iter().any(|outcome| matches!(outcome, Applied::Values(count) if *count > 0));
    Ok((if changed { prc::write(&root) } else { source.to_vec() }, outcomes))
}

pub fn kind_name(param: &Param) -> &'static str {
    match param {
        Param::Bool(_) => "bool",
        Param::I8(_) => "sbyte",
        Param::U8(_) => "byte",
        Param::I16(_) => "short",
        Param::U16(_) => "ushort",
        Param::I32(_) => "int",
        Param::U32(_) => "uint",
        Param::Float(_) => "float",
        Param::Hash(_) => "hash40",
        Param::Str(_) => "string",
        Param::List(_) => "list",
        Param::Struct(_) => "struct",
    }
}

fn is_dummy(param: &Param) -> bool {
    matches!(param, Param::Hash(hash) if *hash == hash40("dummy"))
}

fn merge(patch: &Param, source: &mut Param, trail: &mut String) -> Result<usize, Mismatch> {
    match (patch, source) {
        (Param::Struct(members), Param::Struct(fields)) => {
            let mut seen: Vec<(u64, usize)> = Vec::new();
            let mut count = 0;
            for (key, value) in members {
                let nth = match seen.iter_mut().find(|(known, _)| known == key) {
                    Some((_, times)) => {
                        *times += 1;
                        *times
                    }
                    None => {
                        seen.push((*key, 0));
                        0
                    }
                };
                let mark = trail.len();
                trail.push_str(&format!("/{key:#x}"));
                match fields.iter_mut().filter(|(known, _)| known == key).nth(nth) {
                    Some((_, field)) => count += merge(value, field, trail)?,
                    None => {
                        fields.push((*key, value.clone()));
                        count += 1;
                    }
                }
                trail.truncate(mark);
            }
            Ok(count)
        }
        (Param::List(entries), Param::List(items)) => {
            let mut count = 0;
            for (index, value) in entries.iter().enumerate() {
                if is_dummy(value) {
                    continue;
                }
                let mark = trail.len();
                trail.push_str(&format!("[{index}]"));
                match items.get_mut(index) {
                    Some(item) => count += merge(value, item, trail)?,
                    None => {
                        items.push(value.clone());
                        count += 1;
                    }
                }
                trail.truncate(mark);
            }
            Ok(count)
        }
        (patch, source) if core::mem::discriminant(patch) == core::mem::discriminant(source) => {
            *source = patch.clone();
            Ok(1)
        }
        (patch, source) => Err(Mismatch {
            at: if trail.is_empty() { "/".to_string() } else { trail.clone() },
            file: kind_name(source),
            patch: kind_name(patch),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(name: &str) -> u64 {
        hash40(name)
    }

    fn sample() -> Param {
        Param::Struct(vec![
            (h("camera_fov"), Param::Float(30.0)),
            (h("dead_range"), Param::List(vec![Param::Float(-200.0), Param::Float(200.0)])),
            (h("twice"), Param::I32(1)),
            (h("twice"), Param::I32(2)),
            (
                h("rows"),
                Param::List(vec![
                    Param::Struct(vec![(h("x"), Param::Float(1.0)), (h("kind"), Param::Hash(h("a")))]),
                    Param::Struct(vec![(h("x"), Param::Float(2.0)), (h("kind"), Param::Hash(h("b")))]),
                ]),
            ),
            (h("can_futtobi_back"), Param::Bool(true)),
        ])
    }

    fn field<'a>(root: &'a Param, name: &str) -> &'a Param {
        match root {
            Param::Struct(fields) => &fields.iter().find(|(key, _)| *key == h(name)).unwrap().1,
            _ => panic!("not a struct"),
        }
    }

    #[test]
    fn a_patch_name_maps_to_its_base_like_arcropolis() {
        assert_eq!(
            base_of("stage/x/normal/param/stageparam_x.stprmxml").as_deref(),
            Some("stage/x/normal/param/stageparam_x.stprm")
        );
        assert_eq!(base_of("stage/x/normal/param/end.STDATX").as_deref(), Some("stage/x/normal/param/end.stdat"));
        assert_eq!(base_of("fighter/x/motion/body/c00/update.prcxml").as_deref(), Some("fighter/x/motion/body/c00/update.prc"));
        assert_eq!(base_of("fighter/x/motion/body/c00/update.prcx").as_deref(), Some("fighter/x/motion/body/c00/update.prc"));
        assert_eq!(base_of("ui/message/msg_name+us_en.prcxml"), None);
        assert_eq!(base_of("stage/x/normal/param/x.stprm"), None);
        assert_eq!(patch_names("stageparam_x.stprm"), vec!["stageparam_x.stprmxml", "stageparam_x.stprmx"]);
        assert_eq!(patch_names("end.stdat"), vec!["end.stdatxml", "end.stdatx"]);
        assert_eq!(patch_names("update.prc"), vec!["update.prcxml", "update.prcx"]);
        assert!(is_param_file("Stageparam_X.STPRM"));
        assert!(!is_param_file("x_00.lvd"));
    }

    #[test]
    fn files_dionysus_reads_itself_are_left_to_it() {
        assert!(read_by_dionysus("fighter/bluster/param/vl.prcxml"));
        assert!(read_by_dionysus("fighter/bluster/param/fighter_param.prcxml"));
        assert!(read_by_dionysus("fighter/bluster/ai/param/fighter_param.prcxml"));
        assert!(read_by_dionysus("assist/zeroclone/param/common_param.prcxml"));
        assert!(read_by_dionysus("item/obv/param/param.prcxml"));
        assert!(read_by_dionysus("enemy/primidx/common.prcxml"));
        assert!(read_by_dionysus("stage/hanenbow_live/normal/param/stage_sound_settings.prcxml"));
        assert!(read_by_dionysus("stage/hanenbow_live/battle/param/param.prcxml"));
        assert!(read_by_dionysus("spirit/ui_spirit_db.prcxml"));
        assert!(!read_by_dionysus("fighter/bluster/param/expression_vl.prcxml"));
        assert!(!read_by_dionysus("fighter/bluster/motion/body/c00/update.prcxml"));
        assert!(!read_by_dionysus("stage/hanenbow_live/normal/param/stageparam_hanenbow_live.stprmxml"));
        assert!(!read_by_dionysus("stage/hanenbow_live/normal/render/render_param.prcxml"));
    }

    #[test]
    fn a_path_hashes_as_written_or_from_hex() {
        assert_eq!(path_hash("0x1234"), 0x1234);
        assert_eq!(path_hash("stage/x/a.prc"), hash40("stage/x/a.prc"));
    }

    #[test]
    fn the_xml_keeps_each_value_type() {
        let patch = read_patch(
            br#"<?xml version="1.0" encoding="utf-8"?>
<struct>
  <float hash="camera_fov">45</float>
  <byte hash="b">255</byte>
  <sbyte hash="s">-3</sbyte>
  <ushort hash="u">0xffff</ushort>
  <uint hash="0x1234567890">4000000000</uint>
  <hash40 hash="k">dummy</hash40>
  <string hash="t">a &amp; b</string>
  <bool hash="on">True</bool>
  <list hash="l"><int index="5">7</int><struct index="0"/></list>
</struct>"#,
        )
        .unwrap();
        assert_eq!(
            patch,
            Param::Struct(vec![
                (h("camera_fov"), Param::Float(45.0)),
                (h("b"), Param::U8(255)),
                (h("s"), Param::I8(-3)),
                (h("u"), Param::U16(0xffff)),
                (0x1234567890, Param::U32(4_000_000_000)),
                (h("k"), Param::Hash(h("dummy"))),
                (h("t"), Param::Str("a & b".to_string())),
                (h("on"), Param::Bool(true)),
                (h("l"), Param::List(vec![Param::I32(7), Param::Struct(Vec::new())])),
            ])
        );
        assert!(read_patch(b"<struct><byte hash=\"b\">256</byte></struct>").is_err());
        assert!(read_patch(b"<struct><float hash=\"f\">nan</float></struct>").is_err());
        assert!(read_patch(b"<list></list>").is_err());
        assert!(read_patch(&[0xff, 0xfe, 0]).is_err());
    }

    #[test]
    fn a_patch_sets_values_and_adds_what_is_missing() {
        let mut root = sample();
        let patch = read_patch(
            br#"<struct>
  <float hash="camera_fov">25</float>
  <list hash="dead_range"><float>-250</float></list>
  <int hash="twice">5</int>
  <int hash="twice">6</int>
  <int hash="twice">7</int>
  <bool hash="new_flag">false</bool>
</struct>"#,
        )
        .unwrap();
        assert_eq!(apply(&patch, &mut root), Ok(6));
        assert_eq!(field(&root, "camera_fov"), &Param::Float(25.0));
        assert_eq!(field(&root, "dead_range"), &Param::List(vec![Param::Float(-250.0), Param::Float(200.0)]));
        let Param::Struct(fields) = &root else { unreachable!() };
        let twice: Vec<&Param> = fields.iter().filter(|(key, _)| *key == h("twice")).map(|(_, value)| value).collect();
        assert_eq!(twice, vec![&Param::I32(5), &Param::I32(6), &Param::I32(7)]);
        assert_eq!(field(&root, "new_flag"), &Param::Bool(false));
        assert_eq!(field(&root, "can_futtobi_back"), &Param::Bool(true));
    }

    #[test]
    fn lists_are_positional_skip_dummy_and_push_past_the_end() {
        let mut root = sample();
        let patch = read_patch(
            br#"<struct>
  <list hash="rows">
    <hash40 index="0">dummy</hash40>
    <struct index="1"><float hash="x">9</float></struct>
    <struct index="2"><float hash="x">3</float></struct>
  </list>
</struct>"#,
        )
        .unwrap();
        assert_eq!(apply(&patch, &mut root), Ok(2));
        let Param::List(rows) = field(&root, "rows") else { panic!() };
        assert_eq!(rows.len(), 3);
        assert_eq!(field(&rows[0], "x"), &Param::Float(1.0));
        assert_eq!(field(&rows[1], "x"), &Param::Float(9.0));
        assert_eq!(field(&rows[1], "kind"), &Param::Hash(h("b")));
        assert_eq!(rows[2], Param::Struct(vec![(h("x"), Param::Float(3.0))]));
    }

    #[test]
    fn a_wrong_type_changes_nothing_and_says_where() {
        let mut root = sample();
        let patch = read_patch(
            br#"<struct>
  <float hash="camera_fov">10</float>
  <list hash="rows"><struct><int hash="x">3</int></struct></list>
</struct>"#,
        )
        .unwrap();
        let mismatch = apply(&patch, &mut root).unwrap_err();
        assert_eq!(mismatch.file, "float");
        assert_eq!(mismatch.patch, "int");
        assert_eq!(mismatch.at, format!("/{:#x}[0]/{:#x}", h("rows"), h("x")));
        assert_eq!(root, sample());
    }

    #[test]
    fn a_file_is_patched_in_order_and_one_bad_patch_is_left_out() {
        let source = prc::write(&sample());
        let first = read_patch(br#"<struct><float hash="camera_fov">40</float></struct>"#).unwrap();
        let bad = read_patch(br#"<struct><int hash="camera_fov">1</int></struct>"#).unwrap();
        let second = read_patch(br#"<struct><float hash="camera_fov">50</float><bool hash="can_futtobi_back">false</bool></struct>"#).unwrap();
        let (bytes, outcomes) = patch_file(&source, &[first, bad, second]).unwrap();
        assert!(matches!(outcomes[0], Applied::Values(1)));
        assert!(matches!(outcomes[1], Applied::Mismatch(_)));
        assert!(matches!(outcomes[2], Applied::Values(2)));
        let root = prc::parse(&bytes).unwrap();
        assert_eq!(field(&root, "camera_fov"), &Param::Float(50.0));
        assert_eq!(field(&root, "can_futtobi_back"), &Param::Bool(false));
        assert_eq!(field(&root, "rows"), field(&sample(), "rows"));
    }

    #[test]
    fn a_binary_patch_reads_like_the_xml_one() {
        let binary = prc::write(&Param::Struct(vec![(h("camera_fov"), Param::Float(33.0))]));
        assert_eq!(read_patch(&binary).unwrap(), read_patch(br#"<struct><float hash="camera_fov">33</float></struct>"#).unwrap());
        let source = prc::write(&sample());
        let nothing = read_patch(b"<struct/>").unwrap();
        let (bytes, outcomes) = patch_file(&source, &[nothing]).unwrap();
        assert_eq!(bytes, source);
        assert_eq!(outcomes, vec![Applied::Values(0)]);
    }

    #[test]
    #[ignore]
    fn patches_a_real_file() {
        let source = std::fs::read(std::env::var("PRC_PATCH_SOURCE").unwrap()).unwrap();
        let patch = read_patch(&std::fs::read(std::env::var("PRC_PATCH_FILE").unwrap()).unwrap()).unwrap();
        let (bytes, outcomes) = patch_file(&source, &[patch]).unwrap();
        println!("{outcomes:?}");
        std::fs::write(std::env::var("PRC_PATCH_OUT").unwrap(), bytes).unwrap();
    }

    #[test]
    #[ignore]
    fn real_files_survive_a_rewrite() {
        fn walk(folder: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(folder).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, found);
                } else if is_param_file(&path.to_string_lossy()) {
                    found.push(path);
                }
            }
        }
        let mut found = Vec::new();
        walk(std::path::Path::new(&std::env::var("PRC_PATCH_TREE").unwrap()), &mut found);
        for path in &found {
            let bytes = std::fs::read(path).unwrap();
            let root = prc::parse(&bytes).unwrap();
            assert_eq!(prc::parse(&prc::write(&root)).unwrap(), root, "{}", path.display());
        }
        println!("{} files", found.len());
    }
}
