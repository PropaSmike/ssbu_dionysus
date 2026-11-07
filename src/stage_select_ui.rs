use dionysus_core::settings::StageSelectUi;
use dionysus_core::stage_select_layout::{self as layout, LayoutError};
use std::sync::atomic::{AtomicBool, Ordering};

const LAYOUT_PATH: &str = "ui/layout/patch/stage_select2/stage_select2/layout.arc";

static INSTALLED: AtomicBool = AtomicBool::new(false);

fn explain(error: &LayoutError) -> String {
    match error {
        LayoutError::NotAPlainGrid { columns, rows, buttons } => format!(
            "its {buttons} buttons are not a plain grid ({columns} x {rows}); \
             set stage_select_ui to page_turning or none"
        ),
        LayoutError::NoButtonParent => {
            "its buttons do not sit alone under one parent; set stage_select_ui to page_turning or none".to_owned()
        }
        other => format!("{other:?}"),
    }
}

fn edit(choice: StageSelectUi, original: &[u8], total: usize) -> Option<Vec<u8>> {
    let existing = match layout::button_count(original) {
        Ok(existing) => existing,
        Err(error) => {
            skyline::println!("[stageui] layout left alone: {}", explain(&error));
            return None;
        }
    };
    if total <= existing {
        skyline::println!("[stageui] {} stage(s) fit the layout's {existing} buttons", total);
        return None;
    }
    let total = if total > layout::MAX_BUTTONS {
        skyline::println!(
            "[stageui] {total} stages, only the first {} can have a button",
            layout::MAX_BUTTONS
        );
        layout::MAX_BUTTONS
    } else {
        total
    };
    let edited = match choice {
        StageSelectUi::Vanilla => layout::add_shrunk(original, total).map(|shrunk| {
            skyline::println!(
                "[stageui] vanilla: {existing} -> {total} buttons, {} x {} at {:.0}% size",
                shrunk.columns,
                shrunk.rows,
                shrunk.scale * 100.0
            );
            shrunk.archive
        }),
        StageSelectUi::PageTurning => layout::add_parked(original, total).map(|archive| {
            skyline::println!("[stageui] page_turning: {existing} -> {total} buttons, placed by the screen's script");
            archive
        }),
        StageSelectUi::None => return None,
    };
    match edited {
        Ok(archive) => Some(archive),
        Err(error) => {
            skyline::println!("[stageui] layout left alone: {}", explain(&error));
            None
        }
    }
}

extern "C" fn serve(hash: u64, buffer: *mut u8, length: usize, out_size: &mut usize) -> bool {
    if buffer.is_null() || length == 0 {
        return false;
    }
    let Some(total) = (unsafe { crate::stage_db_rows::listed_count() }) else {
        skyline::println!("[stageui] the stage list is not ready; layout left alone");
        return false;
    };
    let mut original = vec![0u8; length];
    let Some(read) = arcropolis_api::load_original_file(hash, original.as_mut_slice()) else {
        skyline::println!("[stageui] could not read the layout; left alone");
        return false;
    };
    original.truncate(read);
    let Some(edited) = edit(crate::settings::get().stage_select_ui, &original, total) else {
        return false;
    };
    if edited.len() > length {
        skyline::println!(
            "[stageui] edited layout is {} bytes but only {length} were reserved; left alone",
            edited.len()
        );
        return false;
    }
    unsafe { core::ptr::copy_nonoverlapping(edited.as_ptr(), buffer, edited.len()) };
    *out_size = edited.len();
    true
}

pub(crate) fn install() {
    let choice = crate::settings::get().stage_select_ui;
    if choice == StageSelectUi::None {
        skyline::println!("[stageui] none: stages past the layout's buttons will not show");
        return;
    }
    if INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }
    let hash = smash_arc_hash(LAYOUT_PATH);
    let Some(size) = crate::arc_files::decompressed_size(hash) else {
        skyline::println!("[stageui] ARCropolis does not know the layout's size; layout left alone");
        return;
    };
    arcropolis_api::register_callback(LAYOUT_PATH, size + layout::MAX_GROWTH, serve);
    skyline::println!("[stageui] {} screen: watching the layout, {size} bytes", choice.name());
}

fn smash_arc_hash(path: &str) -> u64 {
    arcropolis_api::Hash40::from(path).as_u64()
}
