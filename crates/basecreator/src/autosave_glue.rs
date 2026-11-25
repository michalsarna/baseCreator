//! Automatic saving (see `base_core::autosave`): a project with a file is written to it, one without a name to a recovery
//! file, which the start screen offers to recover; saving under a name removes it.
use crate::{data_paths, App, Screen};
use base_core::autosave::{self, Found};
use base_ui::autosave_ui::Recovery;

const APP: &str = "basecreator";

impl App {
    /// Save what changed, as often as the program options say (and at once with `force`, when the program closes or the settings
    /// are saved); the tabs that are not shown go with the forced saving. Nothing when the auto-save is off.
    pub(crate) fn autosave_tick(&mut self, ctx: &egui::Context, force: bool) {
        if self.screen != Screen::Editor || !self.autosave_opts.on {
            return;
        }
        let every = self.autosave_opts.every();
        if force || (self.parked_need_autosave() && self.parked_saved_at.elapsed() >= every) {
            self.autosave_parked();
        }
        if !self.autosave.changed(self.rev) {
            return;
        }
        if !force && !self.autosave.due_every(self.rev, every) {
            // Come back to look again, also when nothing else makes the window repaint.
            ctx.request_repaint_after(std::time::Duration::from_secs(5));
            return;
        }
        if self.path.is_some() {
            // Written to its own file: a save of the project (joined with the edit before it).
            base_ui::authorship_ui::stamp(&mut self.kept.authorship, "BaseCreator", crate::VERSION, true);
        }
        let text = self.lock.seal(self.to_project().to_json());
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

    /// Open the recovery file of `slot` (unlocked already when it is protected) as unsaved work.
    pub(crate) fn recover_one(&mut self, path: std::path::PathBuf, slot: usize) {
        let recent = self.recent.clone();
        self.open(path);
        if self.screen != Screen::Editor {
            return;
        }
        if self.slot != slot {
            autosave::clear_slot(&data_paths().base, APP, self.slot);
            self.slot = slot;
        }
        self.path = None;
        self.recent = recent;
        self.autosave.in_sync(self.rev);
        self.saved_rev = self.rev.wrapping_sub(1);
        if !self.status.starts_with("Could not") && !self.lock.locked() {
            self.status = "Recovered the project that was not saved".into();
        }
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
                    // A protected project waits for its password; it is recovered when it is given.
                    if std::fs::read_to_string(&path).is_ok_and(|t| base_core::protect::is_sealed(&t)) {
                        let text = std::fs::read_to_string(&path).unwrap_or_default();
                        self.unlock.push(base_ui::protect_ui::Prompt::new(path, text, crate::RECOVER + slot as u32));
                        continue;
                    }
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
