//! One check of the tools for every program: it drives the real window of a program (real pointer and key events through the whole
//! frame) and looks at what the tools did. The text tool once worked in one program only, because the others had not filled in a hook;
//! a test that only draws the window with every tool does not notice that, this one does.
//!
//! Public behind the `tool-checks` feature. A program calls [`check_tools`] and [`check_menu_texts`] from its own test with
//! `base-ui = { …, features = ["tool-checks"] }` in `[dev-dependencies]`.
use crate::editor::{EditorHost, Tool};
use crate::i18n::{tr_in, Lang};
use crate::menu::{Entry, MenuHost, Menus, StdAct};
use base_core::geom::Pt;
use base_core::scene::Kind;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};

/// What the check needs from the window of a program (in the tests of the program, an `egui_kittest` harness).
pub trait Drive {
    type App: EditorHost + MenuHost;
    fn app(&mut self) -> &mut Self::App;
    /// Send an event; it takes effect in the next frames.
    fn send(&mut self, e: Event);
    /// Run a few frames.
    fn frames(&mut self);
    /// Does the window show this text?
    fn shows(&mut self, text: &str) -> bool;
    /// The context of the window.
    fn ctx(&mut self) -> egui::Context;
    /// Drop what a former step left open or focused (the window with the text being typed, a menu).
    fn settle(&mut self) {
        let ctx = self.ctx();
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        self.frames();
    }
}

/// Does the window show this text, in any language? The tests of a program run side by side and another may change the language.
fn shows<D: Drive>(d: &mut D, english: &'static str) -> bool {
    Lang::ALL.iter().any(|l| d.shows(tr_in(*l, english).trim_end_matches('…')))
}

fn key<D: Drive>(d: &mut D, key: Key, pressed: bool) {
    d.send(Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE });
    d.frames();
}

fn press<D: Drive>(d: &mut D, k: Key) {
    key(d, k, true);
    key(d, k, false);
}

fn click<D: Drive>(d: &mut D, at: Pos2, button: PointerButton) {
    d.send(Event::PointerMoved(at));
    d.frames();
    for pressed in [true, false] {
        d.send(Event::PointerButton { pos: at, button, pressed, modifiers: Modifiers::NONE });
        d.frames();
    }
}

fn drag<D: Drive>(d: &mut D, from: Pos2, to: Pos2) {
    d.send(Event::PointerMoved(from));
    d.frames();
    d.send(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    d.frames();
    d.send(Event::PointerMoved(from.lerp(to, 0.5)));
    d.frames();
    d.send(Event::PointerMoved(to));
    d.frames();
    d.send(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    d.frames();
}

/// A place on the screen: `fx`, `fy` as parts of the work area.
fn place<D: Drive>(d: &mut D, fx: f64, fy: f64) -> Pos2 {
    let a = d.app();
    let (w, h) = a.work_size();
    let o = a.ed().canvas_origin;
    a.w2s(o, Pt::new(w * fx, h * fy))
}

fn clear<D: Drive>(d: &mut D) {
    let a = d.app();
    a.scene_mut().shapes.clear();
    a.ed_mut().sel.clear();
    a.ed_mut().node_sel.clear();
    a.ed_mut().pen_pts.clear();
    a.ed_mut().show_shape_popup = false;
    a.ed_mut().last_param = None;
    a.ed_mut().tool = Tool::Select;
    d.settle();
}

/// Make one rectangle with its tool and leave it selected, with the select tool in hand.
fn one_rect<D: Drive>(d: &mut D) -> u64 {
    clear(d);
    press(d, Key::R);
    let (from, to) = (place(d, 0.3, 0.3), place(d, 0.6, 0.6));
    drag(d, from, to);
    assert_eq!(d.app().scene().shapes.len(), 1, "the rectangle tool made a shape");
    let id = d.app().scene().shapes[0].id;
    d.app().ed_mut().tool = Tool::Select;
    d.app().ed_mut().sel = vec![id];
    d.frames();
    id
}

/// Every tool with a real click or drag on the work area, its key, and the keys of the editor. The window must be in the editor, with a
/// new drawing.
pub fn check_tools<D: Drive>(d: &mut D)
where
    <D::App as MenuHost>::Act: From<StdAct>,
{
    d.frames();
    // The tools that draw with a drag, each with its letter.
    for (tool, _, k) in Tool::ALL {
        clear(d);
        press(d, k);
        assert_eq!(d.app().ed().tool, tool, "the key {k:?} chooses {tool:?}");
        if !matches!(tool, Tool::Rect | Tool::Ellipse | Tool::Triangle | Tool::Star | Tool::Polygon | Tool::Heart | Tool::Spiral | Tool::Line) {
            continue;
        }
        let (from, to) = (place(d, 0.3, 0.3), place(d, 0.6, 0.6));
        drag(d, from, to);
        assert_eq!(d.app().scene().shapes.len(), 1, "a drag with {tool:?} makes one shape");
        assert!(d.app().can_undo(), "{tool:?} can be undone");
    }
    // The small window of the polygon and the spiral: opened by the key, and it changes the shape drawn last.
    for (k, tool, label) in [(Key::G, Tool::Polygon, "Number of sides"), (Key::I, Tool::Spiral, "Number of turns")] {
        clear(d);
        press(d, k);
        assert!(shows(d, label), "the {tool:?} key opens its window with {label:?}");
        let (from, to) = (place(d, 0.3, 0.3), place(d, 0.6, 0.6));
        drag(d, from, to);
        let before = d.app().scene().shapes[0].polys();
        if tool == Tool::Polygon {
            d.app().ed_mut().polygon_sides = 11;
        } else {
            d.app().ed_mut().spiral_turns = 9;
        }
        d.app().reshape_last(tool);
        assert_ne!(d.app().scene().shapes[0].polys(), before, "{tool:?}: the number changes the shape drawn last");
    }
    // The text tool: a click places a text, and the field to type in takes the keyboard.
    clear(d);
    press(d, Key::T);
    let at = place(d, 0.5, 0.5);
    click(d, at, PointerButton::Primary);
    assert_eq!(d.app().scene().shapes.len(), 1, "a click with the text tool makes a shape");
    assert!(matches!(d.app().scene().shapes[0].kind, Kind::Text(_)), "and it is a text");
    assert!(d.ctx().egui_wants_keyboard_input(), "the field of the text has the keyboard");
    // The pen: three clicks and a click on the first point make a closed line.
    clear(d);
    press(d, Key::P);
    let pts = [place(d, 0.3, 0.3), place(d, 0.6, 0.3), place(d, 0.6, 0.6), place(d, 0.3, 0.3)];
    for p in pts {
        click(d, p, PointerButton::Primary);
        d.send(Event::PointerMoved(Pos2::new(p.x + 40.0, p.y + 40.0)));
        d.frames();
    }
    assert_eq!(d.app().scene().shapes.len(), 1, "the pen closes on its first point");
    // Select: a click on a shape and a frame round it.
    let id = one_rect(d);
    d.app().ed_mut().sel.clear();
    d.frames();
    let (from, to) = (place(d, 0.1, 0.1), place(d, 0.8, 0.8));
    drag(d, from, to);
    assert_eq!(d.app().ed().sel, vec![id], "a frame round the shape selects it");
    // Node editing: the key with a shape selected, Delete takes the shape when no node is chosen.
    // The hand moves the view, the zoom tools change its scale.
    d.app().ed_mut().sel.clear();
    press(d, Key::H);
    let pan = d.app().ed().view.pan;
    let (from, to) = (place(d, 0.7, 0.8), place(d, 0.75, 0.85));
    drag(d, from, to);
    assert_ne!(d.app().ed().view.pan, pan, "the hand moves the view");
    for (k, grows) in [(Key::Z, true), (Key::X, false)] {
        press(d, k);
        let zoom = d.app().ed().view.zoom;
        let at = place(d, 0.5, 0.5);
        click(d, at, PointerButton::Primary);
        assert_eq!(d.app().ed().view.zoom > zoom, grows, "the zoom tool of {k:?}");
    }
    d.app().ed_mut().view.need_fit = true;
    d.frames();
    // Space holds the hand and gives the tool back.
    press(d, Key::R);
    key(d, Key::Space, true);
    assert_eq!(d.app().ed().tool, Tool::Pan, "the hand while Space is held");
    key(d, Key::Space, false);
    assert_eq!(d.app().ed().tool, Tool::Rect, "and the tool again after it");
    // Arrows move the selection, Delete deletes it, undo brings it back.
    let id = one_rect(d);
    let x = |d: &mut D| d.app().scene().bounds_of(&[id]).unwrap().min.x;
    let before = x(d);
    press(d, Key::ArrowRight);
    assert!((x(d) - before - 1.0).abs() < 1e-6, "the arrow moves the selection by 1 mm");
    press(d, Key::Delete);
    assert!(d.app().scene().shapes.is_empty(), "Delete removes the selection");
    d.app().undo();
    assert_eq!(d.app().scene().shapes.len(), 1, "and undo brings it back");
    // Escape: the selection goes, a second one soon after takes the select tool.
    d.app().ed_mut().sel = vec![id];
    press(d, Key::R);
    press(d, Key::Escape);
    assert!(d.app().ed().sel.is_empty(), "Escape clears the selection");
    d.app().note_escape(0.0);
    d.app().note_escape(0.1);
    assert_eq!(d.app().ed().tool, Tool::Select, "two Escapes give the select tool");
    // The right button opens the menu with the commands of the editor.
    let id = one_rect(d);
    let at = place(d, 0.45, 0.45);
    click(d, at, PointerButton::Secondary);
    assert!(shows(d, "Copy"), "the context menu of the work area");
    press(d, Key::Escape);
    d.settle();
    // Every window of the menus that works on the selection.
    let ctx = d.ctx();
    for (act, title) in [
        (StdAct::OffsetShape, "Offset shape"),
        (StdAct::RoundCorners, "Round corners"),
        (StdAct::AutoJoin, "Join paths"),
        (StdAct::DogboneCorners, "Dogbone corners"),
        (StdAct::GridArray, "Grid array…"),
        (StdAct::CircleArray, "Circular array…"),
        (StdAct::CopyAlongPath, "Copy along path…"),
        (StdAct::TextOnPath, "Text on path…"),
        (StdAct::TrimByShape, "Trim by shape…"),
        (StdAct::SimplifyShapes, "Simplify…"),
        (StdAct::SerialText, "Serial text…"),
        (StdAct::MakeCode, "Barcode / QR code…"),
        (StdAct::NestShapes, "Nest shapes…"),
        (StdAct::MeasureShapes, "Measure…"),
    ] {
        d.app().ed_mut().sel = vec![id];
        let a: <D::App as MenuHost>::Act = act.into();
        d.app().run_act(&ctx, a);
        d.frames();
        assert!(shows(d, title), "{act:?} opens a window with {title:?}");
        let ed = d.app().ed_mut();
        (ed.show_offset, ed.show_round, ed.show_join, ed.show_bones) = (false, false, false, false);
        ed.design.open = None;
        d.frames();
    }
    // The picture under the drawing, or the camera overlay where the program has a camera.
    let a: <D::App as MenuHost>::Act = StdAct::BackgroundPicture.into();
    d.app().run_act(&ctx, a);
    d.frames();
    assert!(shows(d, "Camera overlay") || shows(d, "Background picture"), "the window of the picture under the drawing");
    clear(d);
}

/// The windows only some programs have, or that are not commands of the editor: each action opens a window with this text.
pub fn check_windows<D: Drive>(d: &mut D, windows: &[(<D::App as MenuHost>::Act, &'static str)]) {
    let ctx = d.ctx();
    if windows.iter().any(|(_, title)| *title == "Quick guide") {
        press(d, Key::F1);
        assert!(shows(d, "Quick guide"), "F1 opens the quick guide");
    }
    if windows.iter().any(|(_, title)| *title == "Print") {
        d.send(Event::Key { key: Key::P, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
        d.frames();
        d.send(Event::Key { key: Key::P, physical_key: None, pressed: false, repeat: false, modifiers: Modifiers::COMMAND });
        d.frames();
        assert!(shows(d, "Prints the design on a paper printer, for checking placement or as a template."), "Ctrl+P opens the window of printing");
    }
    for (act, title) in windows {
        d.app().run_act(&ctx, *act);
        d.frames();
        assert!(shows(d, title), "{act:?} opens a window with {title:?}");
    }
}

/// Every text of the menus of a program has a translation: none is left in English.
pub fn check_menu_texts<A>(menus: &Menus<A>) {
    fn walk<A>(entries: &[Entry<A>], out: &mut Vec<&'static str>) {
        for e in entries {
            match e {
                Entry::Item(_, label, _) => out.push(label),
                Entry::Sub(label, more) => {
                    out.push(label);
                    walk(more, out);
                }
                _ => {}
            }
        }
    }
    let mut all = vec![];
    for (title, entries) in menus {
        if !title.is_empty() {
            all.push(*title);
        }
        walk(entries, &mut all);
    }
    let names: Vec<&str> = Lang::ALL.iter().map(|l| l.name()).collect();
    let schemes: Vec<&str> = crate::theme::Scheme::available().iter().map(|s| s.label()).collect();
    // A word can be the same in one language (Laser in Polish); in all of these it has no translation.
    let mut left: Vec<&str> = all.iter().copied().filter(|t| !names.contains(t) && !schemes.contains(t)).filter(|t| [Lang::Pl, Lang::Es, Lang::Fr].iter().all(|l| tr_in(*l, t) == *t)).collect();
    left.sort();
    left.dedup();
    assert!(left.is_empty(), "menu texts without a translation: {left:#?}");
}

/// Every `Menu > Entry` a quick guide names exists in the menus of the program, with its name in the language of the guide, so that the
/// guide cannot send anybody to a place that is gone. A name that is not a menu of this program is not a path.
pub fn check_guide_paths<A>(sections: &[crate::dialogs::Section], menus: &Menus<A>, lang: Lang) {
    fn same(lang: Lang, label: &'static str, name: &str) -> bool {
        tr_in(lang, label).trim_end_matches('…') == name.trim_end_matches('…')
    }
    fn has<A>(entries: &[Entry<A>], lang: Lang, name: &str, rest: &[&str]) -> bool {
        entries.iter().any(|e| match e {
            Entry::Item(_, label, _) => rest.is_empty() && same(lang, label, name),
            Entry::Sub(label, more) => same(lang, label, name) && (rest.is_empty() || has(more, lang, rest[0], &rest[1..])),
            Entry::Recent => rest.is_empty() && same(lang, "Open recent", name),
            _ => false,
        })
    }
    let mut wrong = vec![];
    for line in sections.iter().flat_map(|s| s.lines) {
        for (title, entries) in menus {
            let start = format!("{} > ", tr_in(lang, title));
            for (at, _) in line.match_indices(&start) {
                if at > 0 && line[..at].ends_with(|c: char| c.is_alphanumeric()) {
                    continue;
                }
                let path = &line[at + start.len()..];
                // The longest run of words that names an entry: entries have words of their own (`Device settings…`).
                let words: Vec<&str> = path.split(' ').take(10).collect();
                let found = (1..=words.len()).any(|n| {
                    let text = words[..n].join(" ");
                    let text = text.trim_end_matches([',', '.', ';', ':', ')']);
                    let parts: Vec<&str> = text.split(" > ").collect();
                    has(entries, lang, parts[0], &parts[1..])
                });
                if !found {
                    wrong.push(format!("{} > {}", tr_in(lang, title), words.join(" ")));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "the guide names what the menus do not have: {wrong:#?}");
}

/// The same thing has one Polish name in every program. A program's own table is asked before the shared one, so a row of its own can
/// give a name of its own; the buttons that start a job once said one thing here and another there.
pub fn check_polish_names() {
    for (en, pl) in [("Export", "Eksportuj"), ("Import", "Importuj"), ("Opacity", "Przezroczystość (krycie)")] {
        assert_eq!(tr_in(Lang::Pl, en), pl, "the Polish name of {en}");
    }
    // Start is the same word in every language.
    for lang in Lang::ALL {
        assert_eq!(tr_in(*lang, "Start"), "Start", "Start in {lang:?}");
    }
}

/// Can the fonts of the program draw every character in the string literals of the sources in `dir`? A character that no font has
/// shows as an empty box; egui 0.37 no longer ships its symbols unless asked, so a symbol that worked before can turn into one.
pub fn check_symbols(dir: &std::path::Path) {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
    out.textures_delta.clear();
    let font = egui::FontId::proportional(14.0);
    let mut files = vec![];
    walk(dir, &mut files);
    assert!(!files.is_empty(), "no sources in {dir:?}");
    let mut missing = vec![];
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (n, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for c in chars_in_literals(line) {
                if !c.is_ascii() && !ctx.fonts_mut(|f| f.has_glyph(&font, c)) {
                    missing.push(format!("{}:{} {c} (U+{:04X})", file.display(), n + 1, c as u32));
                }
            }
        }
    }
    assert!(missing.is_empty(), "no font has these characters:\n{}", missing.join("\n"));
}

/// The characters inside the string literals of one line of code.
fn chars_in_literals(line: &str) -> Vec<char> {
    let (mut out, mut inside, mut escaped) = (vec![], false, false);
    for c in line.chars() {
        if inside {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => inside = false,
                _ => out.push(c),
            }
        } else if c == '"' {
            inside = true;
        }
    }
    out
}

#[cfg(test)]
mod guide_path_tests {
    #[test]
    fn the_shared_table_has_the_polish_names() {
        super::check_polish_names();
    }

    #[test]
    fn the_fonts_draw_every_symbol_of_the_shared_code() {
        super::check_symbols(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    }

    #[test]
    fn only_the_characters_of_literals_are_looked_at() {
        assert_eq!(super::chars_in_literals(r#"let a = "x\"y"; // z"#), vec!['x', 'y']);
    }

    #[test]
    #[should_panic(expected = "U+270E")]
    fn a_symbol_no_font_has_is_found() {
        let dir = std::env::temp_dir().join(format!("base-symbols-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "let a = \"\u{270E}\";").unwrap();
        super::check_symbols(&dir);
    }

    use super::*;
    use crate::dialogs::Section;

    fn menus() -> Menus<u8> {
        vec![("File", vec![Entry::Item(1, "Open…", None), Entry::Sub("Export", vec![Entry::Item(2, "Export SVG…", None)])]), ("Machine", vec![Entry::Item(3, "Device settings…", None)])]
    }

    #[test]
    fn a_guide_that_names_the_real_entries_passes() {
        check_guide_paths(&[Section { title: "x", lines: &["Press File > Open… first. Then Machine > Device settings…, and File > Export > Export SVG…"] }], &menus(), Lang::En);
    }

    #[test]
    #[should_panic(expected = "Machine > Switch")]
    fn a_guide_that_names_an_entry_that_is_gone_fails() {
        check_guide_paths(&[Section { title: "x", lines: &["Machine > Switch device… returns to the start window."] }], &menus(), Lang::En);
    }
}
