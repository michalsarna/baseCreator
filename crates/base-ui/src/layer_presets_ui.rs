//! Presets of layers, the same in every program: the settings of a layer saved under a name and put on any layer later. The program
//! says what a preset holds (the type `T`: for a mill the way of cutting, for a plotter the pen); this module keeps the list in a
//! file of the settings folder, and draws the window: a search field, the list (a click puts the preset on the layer; rename and
//! delete beside each), a field to save the layer under a name, and export and import of a file.
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::popup;
use egui::RichText;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_PRESETS: usize = 60;
const MAX_NAME: usize = 60;
/// An exported file is never read when it is bigger than this (bytes).
const MAX_FILE: u64 = 4_000_000;

/// A preset: a name and the settings of a layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Named<T> {
    pub name: String,
    pub layer: T,
}

/// The list of presets of a program, kept in a file.
pub struct Library<T> {
    pub items: Vec<Named<T>>,
    path: Option<PathBuf>,
}

fn clean(name: &str) -> String {
    name.trim().chars().filter(|c| !c.is_control()).take(MAX_NAME).collect()
}

impl<T: Serialize + DeserializeOwned + Clone> Library<T> {
    /// The list in `file_name` of the settings folder; a preset that `valid` refuses is left out.
    pub fn load(file_name: &str, valid: &dyn Fn(&T) -> bool) -> Library<T> {
        let path = base_core::paths::base_dir().map(|d| d.join(file_name));
        Library::from_path(path, valid)
    }

    pub fn from_path(path: Option<PathBuf>, valid: &dyn Fn(&T) -> bool) -> Library<T> {
        let items = path.as_ref().and_then(|f| std::fs::read_to_string(f).ok()).map(|t| parse(&t, valid)).unwrap_or_default();
        Library { items, path }
    }

    pub fn store(&self) {
        if let (Some(f), Ok(t)) = (&self.path, serde_json::to_string_pretty(&self.items)) {
            let _ = std::fs::write(f, t);
        }
    }

    /// Add a preset (the one of the same name is replaced). `false` for no name or a full list.
    pub fn put(&mut self, name: &str, layer: T) -> bool {
        let name = clean(name);
        if name.is_empty() {
            return false;
        }
        if let Some(old) = self.items.iter_mut().find(|p| p.name == name) {
            old.layer = layer;
            return true;
        }
        if self.items.len() >= MAX_PRESETS {
            return false;
        }
        self.items.push(Named { name, layer });
        true
    }

    /// Give preset `i` another name; `false` when the name is empty or another preset has it.
    pub fn rename(&mut self, i: usize, to: &str) -> bool {
        let to = clean(to);
        if to.is_empty() || i >= self.items.len() || self.items.iter().enumerate().any(|(j, p)| j != i && p.name == to) {
            return false;
        }
        self.items[i].name = to;
        true
    }

    pub fn remove(&mut self, i: usize) {
        if i < self.items.len() {
            self.items.remove(i);
        }
    }

    /// The presets as the text of a file that can be given to another person or program.
    pub fn export_text(&self) -> String {
        serde_json::to_string_pretty(&self.items).unwrap_or_default()
    }

    /// Add the presets of an exported text (the ones that are not valid, or that do not fit, are left out; one of the same name
    /// replaces the old one). Returns how many came in.
    pub fn import_text(&mut self, text: &str, valid: &dyn Fn(&T) -> bool) -> usize {
        let mut n = 0;
        for p in parse::<T>(text, valid) {
            if self.put(&p.name, p.layer) {
                n += 1;
            }
        }
        n
    }

    /// The indices of the presets whose name has `filter` in it (any case); all of them for an empty filter.
    pub fn matching(&self, filter: &str) -> Vec<usize> {
        let f = filter.trim().to_lowercase();
        (0..self.items.len()).filter(|i| f.is_empty() || self.items[*i].name.to_lowercase().contains(&f)).collect()
    }
}

/// What a text holds: the valid presets with a name, at most [`MAX_PRESETS`].
pub fn parse<T: DeserializeOwned>(text: &str, valid: &dyn Fn(&T) -> bool) -> Vec<Named<T>> {
    let mut v: Vec<Named<T>> = serde_json::from_str(text).unwrap_or_default();
    v.retain(|p| !p.name.trim().is_empty() && p.name.len() <= 4 * MAX_NAME && valid(&p.layer));
    for p in &mut v {
        p.name = clean(&p.name);
    }
    v.retain(|p| !p.name.is_empty());
    v.truncate(MAX_PRESETS);
    v
}

/// The state of the window of the presets.
#[derive(Default)]
pub struct PresetsUi {
    pub open: bool,
    filter: String,
    name: String,
    renaming: Option<(usize, String)>,
    message: String,
}

impl PresetsUi {
    /// Open the window; with `save` the name field is where the cursor goes.
    pub fn show(&mut self) {
        self.open = true;
    }
}

/// What the person asked for in the window.
#[derive(Debug, PartialEq, Eq)]
pub enum Pick {
    /// Put preset `i` on the layer.
    Apply(usize),
}

/// The window of the presets of layers. `current` gives the settings of the layer that is being edited (for *Save*). Returns the
/// preset to put on the layer when one was clicked.
pub fn window<T: Serialize + DeserializeOwned + Clone>(ctx: &egui::Context, st: &mut PresetsUi, lib: &mut Library<T>, current: &dyn Fn() -> T, valid: &dyn Fn(&T) -> bool) -> Option<Pick> {
    if !st.open {
        return None;
    }
    let mut open = st.open;
    let mut pick = None;
    let mut changed = false;
    popup(ctx, "layer_presets_window", &tr("Layer presets"), [420.0, 420.0], &mut open, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(180.0).hint_text(tr("Name")));
            let ok = !clean(&st.name).is_empty();
            if ui.add_enabled(ok, egui::Button::new(tr("Save preset…"))).on_hover_text(tr("Keep the settings of this layer under this name")).clicked() {
                if lib.put(&st.name, current()) {
                    st.message = trf("Saved {}", &[&clean(&st.name)]);
                    st.name.clear();
                    changed = true;
                } else {
                    st.message = trf("At most {} presets", &[&MAX_PRESETS]);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut st.filter).desired_width(180.0).hint_text(tr("Search")));
            if ui.button(tr("Export…")).on_hover_text(tr("Write all the presets to a file")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).set_file_name("layer-presets.json").save_file() {
                    st.message = match std::fs::write(&p, lib.export_text()) {
                        Ok(()) => trf("Saved {}", &[&p.display()]),
                        Err(e) => trf("Could not save: {}", &[&e]),
                    };
                }
            }
            if ui.button(tr("Import…")).on_hover_text(tr("Add the presets of a file")).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    st.message = match std::fs::metadata(&p).map(|m| m.len()) {
                        Ok(n) if n > MAX_FILE => tr("The file is too big").to_string(),
                        Ok(_) => match std::fs::read_to_string(&p) {
                            Ok(t) => {
                                let n = lib.import_text(&t, valid);
                                changed |= n > 0;
                                if n > 0 { trf("{} presets added", &[&n]) } else { tr("Nothing could be read from the file").to_string() }
                            }
                            Err(e) => trf("Open failed: {}", &[&e]),
                        },
                        Err(e) => trf("Open failed: {}", &[&e]),
                    };
                }
            }
        });
        if !st.message.is_empty() {
            ui.label(RichText::new(&st.message).small().color(theme::text_dim()));
        }
        ui.separator();
        let shown = lib.matching(&st.filter);
        if lib.items.is_empty() {
            ui.label(RichText::new(tr("No presets yet")).color(theme::text_dim()));
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let (mut remove, mut rename) = (None, None);
            for i in shown {
                ui.horizontal(|ui| {
                    if let Some((j, text)) = &mut st.renaming {
                        if *j == i {
                            let r = ui.add(egui::TextEdit::singleline(text).desired_width(200.0));
                            if r.lost_focus() && ui.input(|k| k.key_pressed(egui::Key::Enter)) || ui.small_button("✔").clicked() {
                                rename = Some((i, text.clone()));
                            } else if r.lost_focus() && ui.input(|k| k.key_pressed(egui::Key::Escape)) {
                                st.renaming = None;
                            }
                            return;
                        }
                    }
                    if ui.selectable_label(false, &lib.items[i].name).on_hover_text(tr("Put these settings on the layer")).clicked() {
                        pick = Some(Pick::Apply(i));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").on_hover_text(tr("Forget this preset")).clicked() {
                            remove = Some(i);
                        }
                        if ui.small_button("✎").on_hover_text(tr("Rename")).clicked() {
                            st.renaming = Some((i, lib.items[i].name.clone()));
                        }
                    });
                });
            }
            if let Some((i, to)) = rename {
                if lib.rename(i, &to) {
                    st.renaming = None;
                    changed = true;
                } else {
                    st.message = tr("The name is empty or used by another preset").to_string();
                }
            }
            if let Some(i) = remove {
                lib.remove(i);
                st.renaming = None;
                changed = true;
            }
        });
    });
    st.open = open;
    if changed {
        lib.store();
    }
    pick
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct S {
        depth: f64,
    }

    fn ok(s: &S) -> bool {
        s.depth >= 0.0
    }

    fn lib() -> Library<S> {
        Library { items: vec![], path: None }
    }

    #[test]
    fn a_preset_of_the_same_name_is_replaced_and_the_list_is_limited() {
        let mut l = lib();
        assert!(l.put("  A ", S { depth: 1.0 }) && l.put("A", S { depth: 2.0 }));
        assert_eq!(l.items.len(), 1);
        assert_eq!(l.items[0].layer.depth, 2.0);
        assert!(!l.put("   ", S { depth: 1.0 }), "no name, no preset");
        for i in 0..MAX_PRESETS + 5 {
            l.put(&format!("P{i}"), S { depth: 1.0 });
        }
        assert_eq!(l.items.len(), MAX_PRESETS);
        assert!(!l.put("one more", S { depth: 1.0 }));
    }

    #[test]
    fn a_preset_is_renamed_found_and_forgotten() {
        let mut l = lib();
        l.put("Pocket 4 mm", S { depth: 4.0 });
        l.put("Profile", S { depth: 6.0 });
        assert!(l.rename(1, "Cut out") && l.items[1].name == "Cut out");
        assert!(!l.rename(1, "Pocket 4 mm"), "another preset has the name");
        assert!(!l.rename(1, "  "), "no empty name");
        assert!(l.rename(1, "Cut out"), "its own name again is fine");
        assert_eq!(l.matching("POCKET"), vec![0]);
        assert_eq!(l.matching("").len(), 2);
        assert!(l.matching("zzz").is_empty());
        l.remove(0);
        assert_eq!(l.items.len(), 1);
        l.remove(9);
        assert_eq!(l.items.len(), 1);
    }

    #[test]
    fn the_list_is_saved_exported_imported_and_bad_entries_are_left_out() {
        let dir = std::env::temp_dir().join(format!("base-ui-presets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("p.json");
        let mut l: Library<S> = Library { items: vec![], path: Some(file.clone()) };
        l.put("A", S { depth: 1.0 });
        l.put("B", S { depth: 2.0 });
        l.store();
        let back: Library<S> = Library::from_path(Some(file), &ok);
        assert_eq!(back.items, l.items);
        // Export to another list: both come in; one of the same name replaces; a bad number and broken text give nothing.
        let text = l.export_text();
        let mut other = lib();
        other.put("A", S { depth: 9.0 });
        assert_eq!(other.import_text(&text, &ok), 2);
        assert_eq!(other.items.len(), 2);
        assert_eq!(other.items[0].layer.depth, 1.0, "replaced by the imported one");
        assert_eq!(other.import_text("nonsense", &ok), 0);
        let bad = serde_json::to_string(&vec![Named { name: "bad".to_string(), layer: S { depth: -1.0 } }, Named { name: "good".to_string(), layer: S { depth: 1.0 } }]).unwrap();
        assert_eq!(lib().import_text(&bad, &ok), 1);
        // The old file of MillCreator has the same shape: a name and a "layer".
        assert_eq!(parse::<S>(r#"[{"name":"old","layer":{"depth":3.0}}]"#, &ok)[0].name, "old");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_window_draws_and_a_click_on_a_preset_picks_it() {
        let ctx = egui::Context::default();
        let mut l = lib();
        l.put("Deep", S { depth: 5.0 });
        let mut st = PresetsUi::default();
        st.show();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(window(ui.ctx(), &mut st, &mut l, &|| S { depth: 1.0 }, &ok).is_none());
        });
        out.textures_delta.clear();
        assert!(st.open);
    }
}
