//! Help > Download backup… and Restore from backup…: the dialogs around `base_core::backup`, the same in every
//! program. The program supplies its settings (`settings_pairs`) and applies the restored ones.
use crate::i18n::{tr, trf};
use base_core::backup::{self, Restored};
use std::path::{Path, PathBuf};

/// Menu labels (English; the menu translates them).
pub const DOWNLOAD: &str = "Download backup…";
pub const RESTORE: &str = "Restore from backup…";

/// Ask where to put the backup and write it. `None` when the person cancels; otherwise the line for the status bar.
pub fn download(title: &str, version: &str, settings: &[(&str, String)], gallery: &Path) -> Option<String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let name = backup::file_name(&title.to_lowercase(), now);
    let dest = rfd::FileDialog::new().add_filter("Zip", &["zip"]).set_file_name(&name).save_file()?;
    Some(match backup::write(&dest, title, version, settings, gallery) {
        Ok(_) => trf("Backup saved: {}", &[&dest.display()]),
        Err(e) => trf("Backup failed: {}", &[&e]),
    })
}

/// Choose a backup file; with `confirm` ask first, because the settings of the program are replaced.
pub fn pick_restore(confirm: bool) -> Option<PathBuf> {
    let src = rfd::FileDialog::new().add_filter("Zip", &["zip"]).pick_file()?;
    if confirm && !confirm_restore() {
        return None;
    }
    Some(src)
}

/// The yes / no question before a restore.
pub fn confirm_restore() -> bool {
    let answer = rfd::MessageDialog::new()
        .set_title(tr("Restore from backup"))
        .set_description(tr("Your current settings will be replaced by the backup. Its pictures are added to the shared gallery. Continue?"))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    answer == rfd::MessageDialogResult::Yes
}

/// The line for the status bar after a restore from `src`.
pub fn restore_status(src: &Path, result: &Result<Restored, String>) -> String {
    match result {
        Ok(_) => trf("Backup restored: {}", &[&src.display()]),
        Err(e) => trf("Restore failed: {}", &[&e]),
    }
}
