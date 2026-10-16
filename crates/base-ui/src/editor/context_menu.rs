//! The menu of the right mouse button on the work area, the same in every program: the standard commands of the Edit and
//! Arrange menus, greyed out when they do not apply to the selection. A program that wants other entries draws its own
//! `canvas_context_menu`.
use super::EditorHost;
use crate::i18n::tr;
use crate::menu::StdAct;
use crate::{icons, theme};
use base_core::scene::Kind;

/// What the selection allows.
struct Sel {
    any: bool,
    unlocked: bool,
    locked: bool,
    grouped: bool,
    several: bool,
    image: bool,
}

fn selection<H: EditorHost>(h: &H) -> Sel {
    let shapes: Vec<_> = h.ed().sel.iter().filter_map(|id| h.scene().shape(*id)).collect();
    Sel {
        any: !shapes.is_empty(),
        unlocked: shapes.iter().any(|s| !s.locked),
        locked: shapes.iter().any(|s| s.locked),
        grouped: shapes.iter().any(|s| s.group.is_some()),
        several: shapes.len() >= 2,
        image: shapes.len() == 1 && matches!(shapes[0].kind, Kind::Image(_)),
    }
}

/// Is the command available for this selection?
fn allowed<H: EditorHost>(h: &H, s: &Sel, act: StdAct) -> bool {
    match act {
        StdAct::Copy | StdAct::Duplicate => s.any,
        StdAct::Paste => !h.ed().clipboard.is_empty(),
        StdAct::Delete | StdAct::FlipH | StdAct::FlipV | StdAct::RotCw | StdAct::RotCcw | StdAct::ToFront | StdAct::ToBack | StdAct::Align(_) | StdAct::CenterOnBed | StdAct::EditNodes | StdAct::ToPath => s.unlocked,
        StdAct::Group | StdAct::CenterEachOther => s.several,
        StdAct::Ungroup => s.grouped,
        StdAct::Lock | StdAct::RoundCorners | StdAct::ToCurves => s.unlocked,
        StdAct::BoolUnion | StdAct::BoolIntersect | StdAct::BoolSubtract | StdAct::BoolXor => s.several,
        StdAct::OffsetShape => s.any,
        StdAct::Unlock => s.locked,
        StdAct::AdjustImage | StdAct::TraceImage => s.image,
        _ => act.enabled(h),
    }
}

/// The entries, with the English label (translated where it is drawn), in the order they are shown. `None` is a line.
fn entries() -> Vec<Option<(StdAct, &'static str)>> {
    use StdAct::*;
    vec![
        Some((Copy, "Copy")),
        Some((Paste, "Paste")),
        Some((Duplicate, "Duplicate")),
        Some((Delete, "Delete")),
        Some((SelectAll, "Select all")),
        None,
        Some((Group, "Group")),
        Some((Ungroup, "Ungroup")),
        Some((Lock, "Lock")),
        Some((Unlock, "Unlock")),
        None,
        Some((FlipH, "Flip horizontal")),
        Some((FlipV, "Flip vertical")),
        Some((RotCw, "Rotate 90° CW")),
        Some((RotCcw, "Rotate 90° CCW")),
    ]
}

/// The submenu "Arrange": order, alignment, centring, nodes.
fn arrange_entries() -> Vec<Option<(StdAct, &'static str)>> {
    use StdAct::*;
    vec![
        Some((ToFront, "Bring to front")),
        Some((ToBack, "Send to back")),
        None,
        Some((Align(0), "Align left")),
        Some((Align(1), "Align centre (H)")),
        Some((Align(2), "Align right")),
        Some((Align(3), "Align top")),
        Some((Align(4), "Align centre (V)")),
        Some((Align(5), "Align bottom")),
        None,
        Some((CenterOnBed, "Centre on bed")),
        Some((CenterEachOther, "Centre on each other")),
        None,
        Some((EditNodes, "Edit nodes")),
        Some((ToPath, "Convert to path")),
    ]
}

/// The submenu "Shape operations".
fn shape_op_entries() -> Vec<Option<(StdAct, &'static str)>> {
    use StdAct::*;
    vec![
        Some((BoolUnion, "Union")),
        Some((BoolIntersect, "Intersection")),
        Some((BoolSubtract, "Subtract")),
        Some((BoolXor, "Exclusive or")),
        None,
        Some((OffsetShape, "Offset shape…")),
        Some((RoundCorners, "Round corners…")),
        Some((ToCurves, "Convert to curves")),
    ]
}

/// Draw the menu on a right click and run the command that was chosen.
pub fn context_menu<H: EditorHost>(h: &mut H, ui: &mut egui::Ui, resp: &egui::Response) {
    let mut chosen: Option<StdAct> = None;
    resp.context_menu(|ui| {
        let sel = selection(h);
        let mut item = |ui: &mut egui::Ui, act: StdAct, label: &'static str| {
            let on = allowed(h, &sel, act);
            let ink = if on { theme::text() } else { theme::text().gamma_multiply(0.4) };
            let btn = egui::Button::image_and_text(icons::slot(Some(act.icon()), ink), tr(label));
            if ui.add_enabled(on, btn).clicked() {
                chosen = Some(act);
                ui.close();
            }
        };
        for e in entries() {
            match e {
                Some((act, label)) => item(ui, act, label),
                None => {
                    ui.separator();
                }
            }
        }
        ui.separator();
        ui.menu_button((icons::slot(Some("shapes"), theme::text()), tr("Arrange")), |ui| {
            for e in arrange_entries() {
                match e {
                    Some((act, label)) => item(ui, act, label),
                    None => {
                        ui.separator();
                    }
                }
            }
        });
        ui.menu_button((icons::slot(Some("squares-unite"), theme::text()), tr("Shape operations")), |ui| {
            for e in shape_op_entries() {
                match e {
                    Some((act, label)) => item(ui, act, label),
                    None => {
                        ui.separator();
                    }
                }
            }
        });
        if sel.image {
            ui.separator();
            item(ui, StdAct::AdjustImage, "Adjust image…");
            item(ui, StdAct::TraceImage, "Trace image…");
        }
    });
    let _ = ui;
    if let Some(act) = chosen {
        act.run(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_has_an_icon_that_exists_and_no_command_is_listed_twice() {
        let have: std::collections::HashSet<&str> = icons::all_names().into_iter().collect();
        assert!(have.contains("shapes"), "icon of the Arrange submenu");
        let mut seen: Vec<StdAct> = vec![];
        for (act, label) in entries().into_iter().chain(arrange_entries()).chain(shape_op_entries()).flatten() {
            assert!(have.contains(act.icon()), "icon of {act:?}");
            assert!(!seen.contains(&act), "{act:?} twice");
            assert!(!label.is_empty());
            seen.push(act);
        }
        // The six alignments are all there.
        assert_eq!(seen.iter().filter(|a| matches!(a, StdAct::Align(_))).count(), 6);
    }
}
