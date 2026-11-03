//! What the editor does by itself in every program: the keys without Ctrl or Cmd (the letters of the tools, Delete, Escape, the
//! arrows, Space for the hand) and the small window of the polygon and spiral tools. The Ctrl and Cmd keys come from the menus.
use super::{select_tool, EditorCommands, EditorHost, Tool};
use crate::i18n::tr;
use crate::theme;
use base_core::bezier::Contour;
use base_core::geom::Xf;
use base_core::scene::Kind;
use egui::{Key, Modifiers, RichText};

/// Space held (and not typing): the hand tool, until it is released. For every frame of the editor, before [`plain_keys`].
pub fn hold_space<H: EditorHost>(h: &mut H, ctx: &egui::Context) {
    let space = !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_down(Key::Space) && i.focused);
    if space {
        // Keep a focused button from reacting to the same key.
        ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Space));
    }
    h.update_space(space);
}

/// The keys without Ctrl or Cmd. `arrows`: the arrows move the selected shapes (not while they move the machine, the Keyboard switch
/// of the Control tab). The program checks first that no text field has the keyboard.
pub fn plain_keys<H: EditorHost>(h: &mut H, ctx: &egui::Context, arrows: bool) {
    let plain = |k: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k));
    if plain(Key::Delete) || plain(Key::Backspace) {
        if h.ed().tool == Tool::Node && !h.ed().node_sel.is_empty() {
            h.delete_nodes();
        } else {
            h.delete_selection();
        }
    }
    for (t, _, k) in Tool::ALL {
        if plain(k) {
            select_tool(h.ed_mut(), t);
        }
    }
    // Escape stays in the input: what the window also has open (a question, picking a place) closes with the same key, later in the frame.
    if ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none()) {
        h.note_escape(ctx.input(|i| i.time));
        h.ed_mut().pen_pts.clear();
        if h.ed().tool == Tool::Node && !h.ed().node_sel.is_empty() {
            h.ed_mut().node_sel.clear();
        } else {
            h.ed_mut().sel.clear();
        }
    }
    if !arrows {
        return;
    }
    let step = if ctx.input(|i| i.modifiers.shift) { 10.0 } else { 1.0 };
    for (k, dx, dy) in [(Key::ArrowLeft, -step, 0.0), (Key::ArrowRight, step, 0.0), (Key::ArrowUp, 0.0, -step), (Key::ArrowDown, 0.0, step)] {
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, k)) || ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, k)) {
            if !h.ed().sel.is_empty() {
                h.checkpoint();
                h.transform_selection(Xf::translate(dx, dy));
            }
        }
    }
}

/// Small window of the polygon and spiral tools: how many sides / turns. It sits in the corner of the work area (not over the bars)
/// and has no OK button: a change applies at once, to the shape drawn last as well as to the next ones. It stays while the tool is active.
pub(super) fn shape_popup<H: EditorHost>(h: &mut H, ctx: &egui::Context) {
    let tool = h.ed().tool;
    if !h.ed().show_shape_popup || !matches!(tool, Tool::Polygon | Tool::Spiral) {
        return;
    }
    let polygon = tool == Tool::Polygon;
    let mut open = true;
    let before = if polygon { h.ed().polygon_sides } else { h.ed().spiral_turns };
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin { left: 8, right: 8, top: 6, bottom: 6 });
    let pos = h.ed().canvas_origin + egui::vec2(12.0, 12.0);
    egui::Window::new("shape_popup").title_bar(false).frame(frame).default_pos(pos).resizable(false).show(ctx, |ui| {
        crate::window::mini_title(ui, tr(if polygon { "Polygon" } else { "Spiral" }), &mut open, |_| {});
        let ed = h.ed_mut();
        let (value, range, label): (&mut u32, std::ops::RangeInclusive<u32>, &'static str) =
            if polygon { (&mut ed.polygon_sides, 3..=360, "Number of sides") } else { (&mut ed.spiral_turns, 1..=30, "Number of turns") };
        ui.horizontal(|ui| {
            ui.label(tr(label));
            ui.add(egui::DragValue::new(value).range(range.clone()));
        });
        // The usual choice sits on the slider (polygon 5 to 36, spiral 1 to 24); the number box takes more.
        let slider = if polygon { 5..=36 } else { 1..=24 };
        ui.spacing_mut().slider_width = 200.0;
        ui.add(egui::Slider::new(value, slider).clamping(egui::SliderClamping::Never).show_value(false));
        ui.label(RichText::new(tr("Drag on the work area to draw it.")).color(theme::text_dim()));
    });
    let ed = h.ed_mut();
    ed.polygon_sides = ed.polygon_sides.clamp(3, 360);
    ed.spiral_turns = ed.spiral_turns.clamp(1, 30);
    let now = if polygon { ed.polygon_sides } else { ed.spiral_turns };
    ed.show_shape_popup = open;
    if now != before {
        h.reshape_last(tool);
    }
}

/// The shape of the polygon or spiral drawn last with the number of sides / turns now set.
pub(super) fn reshaped<H: EditorHost>(h: &mut H, tool: Tool) {
    let Some((t, id, w, hh)) = h.ed().last_param else { return };
    if t != tool || h.scene().shape(id).is_none() {
        return;
    }
    let contour = if tool == Tool::Polygon { Contour::polygon(h.ed().polygon_sides, w, hh) } else { Contour::spiral(h.ed().spiral_turns, w, hh) };
    h.checkpoint();
    if let Some(s) = h.scene_mut().unlocked_mut(id) {
        s.kind = Kind::Bezier(vec![contour]);
        h.touch();
    }
}
