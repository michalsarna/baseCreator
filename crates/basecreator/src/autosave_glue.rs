//! Automatic saving (see `base_core::autosave`): a project with a file is written to it, one without a name to a recovery
//! file, which the start screen offers to recover; saving under a name removes it.
use crate::{data_paths, App, Screen};
use base_core::autosave::{self, Found};
use base_ui::autosave_ui::Recovery;

const APP: &str = "basecreator";

impl App {
    /// Save what changed, every `autosave::INTERVAL` (and at once with `force`, when the program closes or the settings are
    /// saved); the tabs that are not shown go with the forced saving.
    pub(crate) fn autosave_tick(&mut self, ctx: &egui::Context, force: bool) {
        if self.screen != Screen::Editor {
            return;
        }
        if force || (self.parked_need_autosave() && self.parked_saved_at.elapsed() >= autosave::INTERVAL) {
            self.autosave_parked();
        }
        if !self.autosave.changed(self.rev) {
            return;
        }
        if !force && !self.autosave.due(self.rev) {
            // Come back to look again, also when nothing else makes the window repaint.
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
            return;
        }
        let text = self.to_project().to_json();
        let r = match &self.path {
            Some(p) => {
                // The project's own file is up to date: there is nothing left to ask about when closing.
                let r = autosave::write_atomic(p, &text);
                if r.is_ok() {
                    self.saved_rev = self.rev;
                }
                r
            }
            None if self.scene.shapes.is_empty() => Ok(()),
            None => autosave::save_slot(&data_paths().base, APP, self.slot, &text),
        };
        if let Err(e) = r {
            self.status = format!("Automatic saving failed: {e}");
        }
        self.autosave.saved(self.rev);
    }

    /// The person saved the project under a name: the recovery file is not needed any more.
    pub(crate) fn autosave_saved_by_hand(&mut self) {
        autosave::clear_slot(&data_paths().base, APP, self.slot);
        let slot = self.slot;
        self.recovery.retain(|f| f.slot != slot);
        self.autosave.in_sync(self.rev);
        self.saved_rev = self.rev;
    }

    /// The project on the screen is what is on disk (just opened, or new).
    pub(crate) fn autosave_in_sync(&mut self) {
        self.autosave.in_sync(self.rev);
        self.saved_rev = self.rev;
    }

    /// The choice on the start screen: open the unsaved projects (each in a tab, as one without a name), or throw them away.
    pub(crate) fn recovery_choice(&mut self, choice: Recovery) {
        if self.recovery.is_empty() {
            return;
        }
        match choice {
            Recovery::Discard => {
                for f in std::mem::take(&mut self.recovery) {
                    autosave::clear_slot(&data_paths().base, APP, f.slot);
                }
            }
            Recovery::Recover => {
                let recent = self.recent.clone();
                // The files still to be opened keep their slots reserved (see `used_slots`), so no new tab clears one.
                while !self.recovery.is_empty() {
                    let Found { path, slot, .. } = self.recovery.remove(0);
                    self.open(path.clone());
                    if self.screen != Screen::Editor {
                        continue;
                    }
                    // It is still unsaved work: no name, not a recent file, and the recovery file stays until it is saved.
                    if self.slot != slot {
                        autosave::clear_slot(&data_paths().base, APP, self.slot);
                        self.slot = slot;
                    }
                    self.path = None;
                    self.recent = recent.clone();
                    self.autosave.in_sync(self.rev);
                    self.saved_rev = self.rev.wrapping_sub(1);
                    if !self.status.starts_with("Could not") {
                        self.status = "Recovered the project that was not saved".into();
                    }
                }
            }
        }
    }
}
