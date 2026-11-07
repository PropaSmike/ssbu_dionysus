pub const LAYOUT_FILE: &str = "blyt/stage_select2.bflyt";
pub const MAX_BUTTONS: usize = 255;
pub const SARC_GRANULE: usize = 0x2000;

const BUTTON_PREFIX: &[u8] = b"set_parts_n_stage_";
const BUTTON_SECTION: usize = 160;
const NAME_AT: usize = 0x0c;
const NAME_LEN: usize = 24;
const TRANSLATE_AT: usize = 0x2c;
const SCALE_AT: usize = 0x44;

pub const MAX_GROWTH: usize = (MAX_BUTTONS * BUTTON_SECTION).div_ceil(SARC_GRANULE) * SARC_GRANULE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutError {
    NotAnArchive,
    NoLayoutFile,
    NotALayout,
    NoButtons,
    ButtonSize(usize),
    ButtonsNotTogether,
    ButtonsMisnumbered,
    NoButtonParent,
    NotAPlainGrid { columns: usize, rows: usize, buttons: usize },
    TooManyButtons(usize),
    NothingToAdd { buttons: usize, wanted: usize },
    CheckFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shrunk {
    pub archive: Vec<u8>,
    pub columns: usize,
    pub rows: usize,
    pub scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Grid {
    columns: usize,
    rows: usize,
    x0: f32,
    y0: f32,
    x_pitch: f32,
    y_pitch: f32,
}

#[derive(Clone, Copy, Debug)]
struct Section {
    offset: usize,
    magic: [u8; 4],
    size: usize,
}

#[derive(Clone, Copy, Debug)]
struct Button {
    index: usize,
    offset: usize,
    number: usize,
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    start: usize,
    end: usize,
    node: usize,
    name: usize,
}

fn u16_at(blob: &[u8], at: usize) -> Option<usize> {
    let bytes = blob.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]) as usize)
}

fn u32_at(blob: &[u8], at: usize) -> Option<usize> {
    let bytes = blob.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
}

fn f32_at(blob: &[u8], at: usize) -> Option<f32> {
    let bytes = blob.get(at..at.checked_add(4)?)?;
    Some(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn put_u32(blob: &mut [u8], at: usize, value: usize) {
    blob[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
}

fn put_f32(blob: &mut [u8], at: usize, value: f32) {
    blob[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn archive_entries(archive: &[u8]) -> Option<(usize, Vec<Entry>)> {
    if archive.get(..4)? != b"SARC" || u16_at(archive, 6)? != 0xFEFF {
        return None;
    }
    if u32_at(archive, 8)? != archive.len() {
        return None;
    }
    let sfat = u16_at(archive, 4)?;
    let data = u32_at(archive, 12)?;
    let header = u16_at(archive, sfat + 4)?;
    let count = u16_at(archive, sfat + 6)?;
    let nodes = sfat + header;
    let sfnt = nodes + count * 16;
    if archive.get(sfnt..sfnt + 4)? != b"SFNT" {
        return None;
    }
    let names = sfnt + u16_at(archive, sfnt + 4)?;
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let node = nodes + index * 16;
        let attributes = u32_at(archive, node + 4)?;
        let name = names + (attributes & 0xFF_FFFF) * 4;
        let (start, end) = (u32_at(archive, node + 8)?, u32_at(archive, node + 12)?);
        if start > end || data + end > archive.len() {
            return None;
        }
        entries.push(Entry { start, end, node, name });
    }
    if entries.windows(2).any(|pair| pair[0].start > pair[1].start) {
        return None;
    }
    Some((data, entries))
}

fn entry_named(archive: &[u8], entries: &[Entry], wanted: &str) -> Option<Entry> {
    entries.iter().copied().find(|entry| {
        archive
            .get(entry.name..entry.name + wanted.len() + 1)
            .map_or(false, |bytes| &bytes[..wanted.len()] == wanted.as_bytes() && bytes[wanted.len()] == 0)
    })
}

fn layout_of(archive: &[u8]) -> Result<(usize, Vec<Entry>, Entry), LayoutError> {
    let (data, entries) = archive_entries(archive).ok_or(LayoutError::NotAnArchive)?;
    let target = entry_named(archive, &entries, LAYOUT_FILE).ok_or(LayoutError::NoLayoutFile)?;
    Ok((data, entries, target))
}

fn replace_file(
    archive: &[u8],
    data: usize,
    entries: &[Entry],
    target: Entry,
    payload: &[u8],
) -> Result<Vec<u8>, LayoutError> {
    let old = target.end - target.start;
    if payload.len() < old {
        return Err(LayoutError::CheckFailed);
    }
    let grew = payload.len() - old;
    let shift = grew.div_ceil(SARC_GRANULE) * SARC_GRANULE;
    let mut out = Vec::with_capacity(archive.len() + shift);
    out.extend_from_slice(&archive[..data + target.start]);
    out.extend_from_slice(payload);
    out.resize(out.len() + (shift - grew), 0);
    out.extend_from_slice(&archive[data + target.end..]);
    for entry in entries {
        if entry.node == target.node {
            put_u32(&mut out, entry.node + 12, target.start + payload.len());
        } else if entry.start > target.start {
            put_u32(&mut out, entry.node + 8, entry.start + shift);
            put_u32(&mut out, entry.node + 12, entry.end + shift);
        }
    }
    let size = out.len();
    put_u32(&mut out, 8, size);
    Ok(out)
}

fn sections_of(layout: &[u8]) -> Option<Vec<Section>> {
    if layout.get(..4)? != b"FLYT" || u32_at(layout, 12)? != layout.len() {
        return None;
    }
    let count = u32_at(layout, 16)?;
    let mut offset = u16_at(layout, 6)?;
    let mut sections = Vec::new();
    for _ in 0..count {
        let magic: [u8; 4] = layout.get(offset..offset + 4)?.try_into().ok()?;
        let size = u32_at(layout, offset + 4)?;
        if size < 8 {
            return None;
        }
        sections.push(Section { offset, magic, size });
        offset = offset.checked_add(size)?;
    }
    (offset == layout.len()).then_some(sections)
}

fn button_number(layout: &[u8], offset: usize) -> Option<usize> {
    let name = layout.get(offset + NAME_AT..offset + NAME_AT + NAME_LEN)?;
    let digits = name.strip_prefix(BUTTON_PREFIX)?;
    let end = digits.iter().position(|byte| *byte == 0).unwrap_or(digits.len());
    let text = core::str::from_utf8(&digits[..end]).ok()?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn buttons_of(layout: &[u8], sections: &[Section]) -> Result<Vec<Button>, LayoutError> {
    let mut buttons = Vec::new();
    for (index, section) in sections.iter().enumerate() {
        if &section.magic != b"prt1" {
            continue;
        }
        let Some(number) = button_number(layout, section.offset) else {
            continue;
        };
        if section.size != BUTTON_SECTION {
            return Err(LayoutError::ButtonSize(section.size));
        }
        buttons.push(Button { index, offset: section.offset, number });
    }
    if buttons.is_empty() {
        return Err(LayoutError::NoButtons);
    }
    if buttons.windows(2).any(|pair| pair[1].index != pair[0].index + 1) {
        return Err(LayoutError::ButtonsNotTogether);
    }
    let mut numbers: Vec<usize> = buttons.iter().map(|button| button.number).collect();
    numbers.sort_unstable();
    if numbers.iter().enumerate().any(|(want, number)| want != *number) {
        return Err(LayoutError::ButtonsMisnumbered);
    }
    Ok(buttons)
}

fn parent_of(sections: &[Section], buttons: &[Button]) -> Result<Section, LayoutError> {
    let first = buttons[0].index;
    let last = buttons[buttons.len() - 1].index;
    let opens = first.checked_sub(1).and_then(|index| sections.get(index));
    let closes = sections.get(last + 1);
    let (Some(opens), Some(closes)) = (opens, closes) else {
        return Err(LayoutError::NoButtonParent);
    };
    if &opens.magic != b"pas1" || &closes.magic != b"pae1" {
        return Err(LayoutError::NoButtonParent);
    }
    let parent = sections[..first - 1]
        .iter()
        .rev()
        .find(|section| &section.magic != b"usd1")
        .ok_or(LayoutError::NoButtonParent)?;
    if &parent.magic != b"pan1" {
        return Err(LayoutError::NoButtonParent);
    }
    Ok(*parent)
}

fn distinct(mut values: Vec<f32>) -> Vec<f32> {
    values.sort_by(|left, right| left.total_cmp(right));
    let mut out: Vec<f32> = Vec::new();
    for value in values {
        if out.last().map_or(true, |last| (value - *last).abs() > 0.01) {
            out.push(value);
        }
    }
    out
}

fn regular(values: &[f32]) -> Option<f32> {
    let pitch = (values[values.len() - 1] - values[0]) / (values.len() - 1) as f32;
    if pitch <= 0.0 {
        return None;
    }
    values
        .iter()
        .enumerate()
        .all(|(index, value)| {
            let want = values[0] + index as f32 * pitch;
            (value - want).abs() <= 0.01_f32.max(want.abs() * 1e-4)
        })
        .then_some(pitch)
}

fn grid_of(layout: &[u8], buttons: &[Button]) -> Result<Grid, LayoutError> {
    let mut xs = Vec::with_capacity(buttons.len());
    let mut ys = Vec::with_capacity(buttons.len());
    for button in buttons {
        let x = f32_at(layout, button.offset + TRANSLATE_AT).ok_or(LayoutError::NotALayout)?;
        let y = f32_at(layout, button.offset + TRANSLATE_AT + 4).ok_or(LayoutError::NotALayout)?;
        xs.push(x);
        ys.push(y);
    }
    let (xs, ys) = (distinct(xs), distinct(ys));
    let not_plain = LayoutError::NotAPlainGrid {
        columns: xs.len(),
        rows: ys.len(),
        buttons: buttons.len(),
    };
    if xs.len() < 2 || ys.len() < 2 || xs.len() * ys.len() != buttons.len() {
        return Err(not_plain);
    }
    let (Some(x_pitch), Some(y_pitch)) = (regular(&xs), regular(&ys)) else {
        return Err(not_plain);
    };
    Ok(Grid {
        columns: xs.len(),
        rows: ys.len(),
        x0: xs[0],
        y0: ys[ys.len() - 1],
        x_pitch,
        y_pitch,
    })
}

pub fn fit(columns: usize, rows: usize, total: usize) -> (usize, usize, f32) {
    let mut best = (columns, total.div_ceil(columns), 0.0_f32, usize::MAX);
    for wide in columns..=total.max(columns) {
        let tall = total.div_ceil(wide);
        if tall < 2 {
            break;
        }
        let across = (columns - 1) as f32 / (wide - 1) as f32;
        let down = (rows - 1) as f32 / (tall - 1) as f32;
        let scale = across.min(down).min(1.0);
        let waste = wide * tall - total;
        if scale > best.2 + 1e-4 || ((scale - best.2).abs() <= 1e-4 && waste < best.3) {
            best = (wide, tall, scale, waste);
        }
    }
    (best.0, best.1, best.2)
}

fn splice_buttons(layout: &[u8], sections: &[Section], buttons: &[Button], added: &[u8], count: usize) -> Vec<u8> {
    let seam = buttons[buttons.len() - 1].offset + BUTTON_SECTION;
    let mut grown = Vec::with_capacity(layout.len() + added.len());
    grown.extend_from_slice(&layout[..seam]);
    grown.extend_from_slice(added);
    grown.extend_from_slice(&layout[seam..]);
    let size = grown.len();
    put_u32(&mut grown, 12, size);
    put_u32(&mut grown, 16, sections.len() + count);
    grown
}

fn clone_button(template: &[u8], number: usize) -> Vec<u8> {
    let mut button = template.to_vec();
    let mut name = [0u8; NAME_LEN];
    let text = format!("set_parts_n_stage_{number:03}");
    name[..text.len()].copy_from_slice(text.as_bytes());
    button[NAME_AT..NAME_AT + NAME_LEN].copy_from_slice(&name);
    button
}

fn checked(total: usize, buttons: usize) -> Result<(), LayoutError> {
    if total > MAX_BUTTONS {
        return Err(LayoutError::TooManyButtons(total));
    }
    if total <= buttons {
        return Err(LayoutError::NothingToAdd { buttons, wanted: total });
    }
    Ok(())
}

struct Opened {
    data: usize,
    entries: Vec<Entry>,
    target: Entry,
    layout: Vec<u8>,
    sections: Vec<Section>,
    buttons: Vec<Button>,
}

fn open(archive: &[u8]) -> Result<Opened, LayoutError> {
    let (data, entries, target) = layout_of(archive)?;
    let layout = archive[data + target.start..data + target.end].to_vec();
    let sections = sections_of(&layout).ok_or(LayoutError::NotALayout)?;
    let buttons = buttons_of(&layout, &sections)?;
    Ok(Opened { data, entries, target, layout, sections, buttons })
}

pub fn button_count(archive: &[u8]) -> Result<usize, LayoutError> {
    Ok(open(archive)?.buttons.len())
}

pub fn add_shrunk(archive: &[u8], total: usize) -> Result<Shrunk, LayoutError> {
    let Opened { data, entries, target, layout, sections, buttons } = open(archive)?;
    checked(total, buttons.len())?;
    let grid = grid_of(&layout, &buttons)?;
    let parent = parent_of(&sections, &buttons)?;
    let (columns, rows, scale) = fit(grid.columns, grid.rows, total);

    let template_at = buttons[0].offset;
    let template = &layout[template_at..template_at + BUTTON_SECTION];
    let mut added = Vec::with_capacity((total - buttons.len()) * BUTTON_SECTION);
    for number in buttons.len()..total {
        added.extend_from_slice(&clone_button(template, number));
    }
    let mut grown = splice_buttons(&layout, &sections, &buttons, &added, total - buttons.len());

    let new_sections = sections_of(&grown).ok_or(LayoutError::CheckFailed)?;
    let new_buttons = buttons_of(&grown, &new_sections)?;
    let width = (grid.columns - 1) as f32 * grid.x_pitch;
    let height = (grid.rows - 1) as f32 * grid.y_pitch;
    let used_width = (columns - 1) as f32 * grid.x_pitch * scale;
    let used_height = (rows - 1) as f32 * grid.y_pitch * scale;
    let left = grid.x0 + (width - used_width) / 2.0;
    let top = grid.y0 - (height - used_height) / 2.0;
    for button in &new_buttons {
        let (row, column) = (button.number / columns, button.number % columns);
        let x = left / scale + column as f32 * grid.x_pitch;
        let y = top / scale - row as f32 * grid.y_pitch;
        put_f32(&mut grown, button.offset + TRANSLATE_AT, x);
        put_f32(&mut grown, button.offset + TRANSLATE_AT + 4, y);
    }
    let scale_x = f32_at(&layout, parent.offset + SCALE_AT).ok_or(LayoutError::NotALayout)?;
    let scale_y = f32_at(&layout, parent.offset + SCALE_AT + 4).ok_or(LayoutError::NotALayout)?;
    put_f32(&mut grown, parent.offset + SCALE_AT, scale_x * scale);
    put_f32(&mut grown, parent.offset + SCALE_AT + 4, scale_y * scale);

    let archive = replace_file(archive, data, &entries, target, &grown)?;
    if button_count(&archive)? != total {
        return Err(LayoutError::CheckFailed);
    }
    Ok(Shrunk { archive, columns, rows, scale })
}

pub fn add_parked(archive: &[u8], total: usize) -> Result<Vec<u8>, LayoutError> {
    let Opened { data, entries, target, layout, sections, buttons } = open(archive)?;
    checked(total, buttons.len())?;
    let highest = buttons
        .iter()
        .max_by_key(|button| button.number)
        .ok_or(LayoutError::NoButtons)?;
    let template = &layout[highest.offset..highest.offset + BUTTON_SECTION];
    let mut added = Vec::with_capacity((total - buttons.len()) * BUTTON_SECTION);
    for number in buttons.len()..total {
        added.extend_from_slice(&clone_button(template, number));
    }
    let grown = splice_buttons(&layout, &sections, &buttons, &added, total - buttons.len());
    let archive = replace_file(archive, data, &entries, target, &grown)?;
    if button_count(&archive)? != total {
        return Err(LayoutError::CheckFailed);
    }
    Ok(archive)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLUMNS: usize = 11;
    const ROWS: usize = 11;
    const PITCH: (f32, f32) = (112.0, 86.0);
    const ORIGIN: (f32, f32) = (-238.0, 394.0);

    fn section(magic: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = magic.to_vec();
        out.extend_from_slice(&((body.len() + 8) as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn pane(magic: &[u8; 4], name: &str, x: f32, y: f32, size: usize) -> Vec<u8> {
        let mut out = vec![0u8; size];
        out[..4].copy_from_slice(magic);
        out[4..8].copy_from_slice(&(size as u32).to_le_bytes());
        out[NAME_AT..NAME_AT + name.len()].copy_from_slice(name.as_bytes());
        out[TRANSLATE_AT..TRANSLATE_AT + 4].copy_from_slice(&x.to_le_bytes());
        out[TRANSLATE_AT + 4..TRANSLATE_AT + 8].copy_from_slice(&y.to_le_bytes());
        out[SCALE_AT..SCALE_AT + 4].copy_from_slice(&1.0_f32.to_le_bytes());
        out[SCALE_AT + 4..SCALE_AT + 8].copy_from_slice(&1.0_f32.to_le_bytes());
        out
    }

    fn layout_with(positions: &[(f32, f32)]) -> Vec<u8> {
        let mut sections: Vec<Vec<u8>> = vec![
            section(b"lyt1", &[0u8; 16]),
            pane(b"pan1", "stage_normal_btns", 0.0, 0.0, 0x54),
            section(b"pas1", &[]),
        ];
        for (number, (x, y)) in positions.iter().enumerate() {
            sections.push(pane(b"prt1", &format!("set_parts_n_stage_{number:03}"), *x, *y, BUTTON_SECTION));
        }
        sections.push(section(b"pae1", &[]));
        sections.push(pane(b"pan1", "stage_make", 7.0, 9.0, 0x54));
        let body: Vec<u8> = sections.concat();
        let mut out = b"FLYT".to_vec();
        out.extend_from_slice(&0xFEFFu16.to_le_bytes());
        out.extend_from_slice(&0x14u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&((0x14 + body.len()) as u32).to_le_bytes());
        out.extend_from_slice(&(sections.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    fn plain_positions() -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                out.push((
                    ORIGIN.0 + column as f32 * PITCH.0,
                    ORIGIN.1 - row as f32 * PITCH.1,
                ));
            }
        }
        out
    }

    fn archive_with(files: &[(&str, Vec<u8>, usize)]) -> Vec<u8> {
        let mut names = Vec::new();
        let mut name_offsets = Vec::new();
        for (name, _, _) in files {
            name_offsets.push(names.len() / 4);
            names.extend_from_slice(name.as_bytes());
            names.push(0);
            while names.len() % 4 != 0 {
                names.push(0);
            }
        }
        let header = 0x14usize;
        let sfat = 0x0c + files.len() * 16;
        let sfnt = 8 + names.len();
        let mut data_start = header + sfat + sfnt;
        data_start = data_start.div_ceil(0x2000) * 0x2000;
        let mut data = Vec::new();
        let mut spans = Vec::new();
        for (_, bytes, align) in files {
            while (data_start + data.len()) % align != 0 {
                data.push(0xEE);
            }
            let start = data.len();
            data.extend_from_slice(bytes);
            spans.push((start, data.len()));
        }
        let total = data_start + data.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"SARC");
        out.extend_from_slice(&(header as u16).to_le_bytes());
        out.extend_from_slice(&0xFEFFu16.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(data_start as u32).to_le_bytes());
        out.extend_from_slice(&0x0100u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(b"SFAT");
        out.extend_from_slice(&0x0cu16.to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&0x65u32.to_le_bytes());
        for (index, (start, end)) in spans.iter().enumerate() {
            out.extend_from_slice(&(index as u32).to_le_bytes());
            out.extend_from_slice(&(0x0100_0000u32 | name_offsets[index] as u32).to_le_bytes());
            out.extend_from_slice(&(*start as u32).to_le_bytes());
            out.extend_from_slice(&(*end as u32).to_le_bytes());
        }
        out.extend_from_slice(b"SFNT");
        out.extend_from_slice(&8u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&names);
        out.resize(data_start, 0);
        out.extend_from_slice(&data);
        out
    }

    fn vanilla_like() -> Vec<u8> {
        archive_with(&[
            ("anim/a.bflan", vec![1u8; 0x123], 16),
            (LAYOUT_FILE, layout_with(&plain_positions()), 16),
            ("blyt/z.bflyt", vec![2u8; 0x40], 16),
            ("timg/t.bntx", vec![3u8; 0x3000], 0x2000),
        ])
    }

    fn file(archive: &[u8], name: &str) -> Vec<u8> {
        let (data, entries) = archive_entries(archive).unwrap();
        let entry = entry_named(archive, &entries, name).unwrap();
        archive[data + entry.start..data + entry.end].to_vec()
    }

    fn position(layout: &[u8], number: usize) -> (f32, f32) {
        let sections = sections_of(layout).unwrap();
        let button = buttons_of(layout, &sections)
            .unwrap()
            .into_iter()
            .find(|button| button.number == number)
            .unwrap();
        (
            f32_at(layout, button.offset + TRANSLATE_AT).unwrap(),
            f32_at(layout, button.offset + TRANSLATE_AT + 4).unwrap(),
        )
    }

    fn parent_scale(layout: &[u8]) -> f32 {
        let at = sections_of(layout).unwrap()[1].offset;
        f32_at(layout, at + SCALE_AT).unwrap()
    }

    #[test]
    fn it_reads_the_grid_of_a_vanilla_layout() {
        let archive = vanilla_like();
        assert_eq!(button_count(&archive), Ok(121));
        let layout = file(&archive, LAYOUT_FILE);
        let sections = sections_of(&layout).unwrap();
        let grid = grid_of(&layout, &buttons_of(&layout, &sections).unwrap()).unwrap();
        assert_eq!((grid.columns, grid.rows), (11, 11));
        assert_eq!((grid.x_pitch, grid.y_pitch), PITCH);
    }

    #[test]
    fn fit_keeps_the_icons_as_large_as_possible() {
        assert_eq!(fit(11, 11, 122).0 * fit(11, 11, 122).1 >= 122, true);
        let (columns, rows, scale) = fit(11, 11, 130);
        assert!(columns * rows >= 130);
        assert!((scale - 10.0 / 11.0).abs() < 1e-3, "{scale}");
        let (columns, rows, scale) = fit(11, 11, 255);
        assert_eq!((columns, rows), (16, 16));
        assert!((scale - 10.0 / 15.0).abs() < 1e-3, "{scale}");
        for total in 122..=255 {
            let (columns, rows, scale) = fit(11, 11, total);
            assert!(columns * rows >= total, "{total}");
            assert!(scale > 0.6 && scale <= 1.0, "{total} {scale}");
        }
    }

    #[test]
    fn shrinking_adds_the_buttons_and_scales_only_their_parent() {
        let source = vanilla_like();
        let shrunk = add_shrunk(&source, 130).unwrap();
        assert_eq!(button_count(&shrunk.archive), Ok(130));
        let layout = file(&shrunk.archive, LAYOUT_FILE);
        assert!((parent_scale(&layout) - shrunk.scale).abs() < 1e-5);
        let sections = sections_of(&layout).unwrap();
        for button in buttons_of(&layout, &sections).unwrap() {
            assert_eq!(f32_at(&layout, button.offset + SCALE_AT), Some(1.0));
        }
        let stage_make = sections[sections.len() - 1].offset;
        assert_eq!(f32_at(&layout, stage_make + TRANSLATE_AT), Some(7.0));
        assert_eq!(f32_at(&layout, stage_make + SCALE_AT), Some(1.0));
    }

    #[test]
    fn the_shrunk_grid_stays_inside_the_original_rectangle() {
        let source = vanilla_like();
        for total in [122, 130, 144, 200, 255] {
            let shrunk = add_shrunk(&source, total).unwrap();
            let layout = file(&shrunk.archive, LAYOUT_FILE);
            let right = ORIGIN.0 + 10.0 * PITCH.0;
            let bottom = ORIGIN.1 - 10.0 * PITCH.1;
            for number in 0..total {
                let (x, y) = position(&layout, number);
                let (x, y) = (x * shrunk.scale, y * shrunk.scale);
                assert!(x >= ORIGIN.0 - 0.1 && x <= right + 0.1, "{total} {number} x {x}");
                assert!(y <= ORIGIN.1 + 0.1 && y >= bottom - 0.1, "{total} {number} y {y}");
            }
            let (first_x, first_y) = position(&layout, 0);
            let (next_x, next_y) = position(&layout, 1);
            let (below_x, below_y) = position(&layout, shrunk.columns);
            assert!((next_x - first_x - PITCH.0).abs() < 0.01);
            assert!((next_y - first_y).abs() < 0.01);
            assert!((below_x - first_x).abs() < 0.01);
            assert!((first_y - below_y - PITCH.1).abs() < 0.01);
        }
    }

    #[test]
    fn parking_clones_the_last_button_and_moves_nothing() {
        let mut positions = plain_positions();
        for (index, position) in positions.iter_mut().enumerate().skip(39) {
            *position = (index as f32 % 7.0, -2000.0);
        }
        let source = archive_with(&[(LAYOUT_FILE, layout_with(&positions), 16)]);
        let out = add_parked(&source, 130).unwrap();
        assert_eq!(button_count(&out), Ok(130));
        let layout = file(&out, LAYOUT_FILE);
        for (number, want) in positions.iter().enumerate() {
            assert_eq!(position(&layout, number), *want);
        }
        for number in 121..130 {
            assert_eq!(position(&layout, number), positions[120]);
        }
        assert_eq!(parent_scale(&layout), 1.0);
    }

    #[test]
    fn every_other_file_keeps_its_bytes_and_its_alignment() {
        let source = vanilla_like();
        for out in [add_shrunk(&source, 200).unwrap().archive, add_parked(&source, 200).unwrap()] {
            for name in ["anim/a.bflan", "blyt/z.bflyt", "timg/t.bntx"] {
                assert_eq!(file(&out, name), file(&source, name), "{name}");
            }
            let (data, entries) = archive_entries(&out).unwrap();
            let texture = entry_named(&out, &entries, "timg/t.bntx").unwrap();
            assert_eq!((data + texture.start) % 0x2000, 0);
            assert_eq!(u32_at(&out, 8), Some(out.len()));
            assert!(out.len() - source.len() <= MAX_GROWTH);
        }
    }

    #[test]
    fn a_layout_that_is_not_a_plain_grid_is_refused_for_shrinking() {
        let mut positions = plain_positions();
        for (index, position) in positions.iter_mut().enumerate().skip(39) {
            *position = (index as f32 % 7.0, -2000.0);
        }
        let source = archive_with(&[(LAYOUT_FILE, layout_with(&positions), 16)]);
        assert!(matches!(add_shrunk(&source, 130), Err(LayoutError::NotAPlainGrid { buttons: 121, .. })));
    }

    #[test]
    fn it_refuses_what_it_cannot_do() {
        let source = vanilla_like();
        assert_eq!(
            add_shrunk(&source, 121),
            Err(LayoutError::NothingToAdd { buttons: 121, wanted: 121 })
        );
        assert_eq!(add_parked(&source, 100), Err(LayoutError::NothingToAdd { buttons: 121, wanted: 100 }));
        assert_eq!(add_shrunk(&source, 256), Err(LayoutError::TooManyButtons(256)));
        assert_eq!(button_count(b"nope"), Err(LayoutError::NotAnArchive));
        let other = archive_with(&[("blyt/z.bflyt", vec![0u8; 8], 16)]);
        assert_eq!(button_count(&other), Err(LayoutError::NoLayoutFile));
    }

    #[test]
    fn broken_files_never_panic() {
        let source = vanilla_like();
        for cut in [0, 3, 8, 16, 40, 200, source.len() / 2, source.len() - 1] {
            let _ = add_shrunk(&source[..cut], 130);
            let _ = add_parked(&source[..cut], 130);
        }
        let mut bad = source.clone();
        let (data, entries) = archive_entries(&bad).unwrap();
        let target = entry_named(&bad, &entries, LAYOUT_FILE).unwrap();
        put_u32(&mut bad, data + target.start + 16, 0xFFFF);
        assert_eq!(add_shrunk(&bad, 130), Err(LayoutError::NotALayout));
    }
}
