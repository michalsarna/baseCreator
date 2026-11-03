//! What the menus of BaseCreator can do and how they are laid out. The layout comes from base-ui, without what needs a machine: no
//! Machine menu, no menu of a process, no G-code.
use base_ui::i18n::Lang;
use base_ui::menu::{standard_menus, Entry, MenuParts, Menus, StdAct};
use base_ui::theme::Scheme;

/// How many recent files the File menu keeps.
pub const MAX_RECENT: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    New,
    Open,
    Save,
    SaveAs,
    PageSetup,
    ImportSvg,
    ImportAi,
    ImportDxf,
    ImportLightburn,
    ImportUgsd,
    ImportPlugin,
    ExportSvg,
    ExportCam,
    /// The place of *View G-code file…* in the common layout: taken out of the menus by `menus`.
    NoGcode,
    Plugins,
    GenerateShape,
    OnlineClipart,
    OpenRecent(u8),
    ClearRecent,
    Quit,
    Std(StdAct),
    ViewOptions,
    ProgramOptions,
    CheckUpdates,
    SetLang(Lang),
    SetScheme(Scheme),
    OpenGithub,
    DownloadBackup,
    RestoreBackup,
    About,
}

impl From<StdAct> for Act {
    fn from(a: StdAct) -> Act {
        Act::Std(a)
    }
}

/// The entries without the one that has no use here.
fn without_gcode(entries: Vec<Entry<Act>>) -> Vec<Entry<Act>> {
    entries
        .into_iter()
        .filter(|e| !matches!(e, Entry::Item(Act::NoGcode, _, _)))
        .map(|e| match e {
            Entry::Sub(t, inner) => Entry::Sub(t, without_gcode(inner)),
            other => other,
        })
        .collect()
}

/// The menus: the layout of baseCreator (`standard_menus`) with the entries of BaseCreator in their places, and without the empty
/// menus of a machine. The macOS app menu (About, Quit) is added by `native_menu`.
pub fn menus() -> Menus<Act> {
    use Entry::Item;
    let all = standard_menus(MenuParts {
        new: Act::New,
        open: Act::Open,
        save: Act::Save,
        save_as: Act::SaveAs,
        quit: Act::Quit,
        about: Act::About,
        view_gcode: Act::NoGcode,
        generate: Act::GenerateShape,
        import_plugin: Act::ImportPlugin,
        online_clipart: Act::OnlineClipart,
        plugins: Act::Plugins,
        view_options: Act::ViewOptions,
        program_options: Act::ProgramOptions,
        set_lang: Act::SetLang,
        set_scheme: Act::SetScheme,
        github: Act::OpenGithub,
        download_backup: Act::DownloadBackup,
        restore_backup: Act::RestoreBackup,
        check_updates: Act::CheckUpdates,
        file_extra: vec![Item(Act::PageSetup, "Page setup…", None)],
        import_svg: Act::ImportSvg,
        import_ai: Act::ImportAi,
        import_dxf: Act::ImportDxf,
        import_lightburn: Act::ImportLightburn,
        import_ugsd: Act::ImportUgsd,
        export_svg: Act::ExportSvg,
        export_cam: Act::ExportCam,
        import_extra: vec![],
        export_first: vec![],
        export_extra: vec![],
        output_extra: vec![],
        view_extra: vec![],
        background: true,
        machine: vec![],
        process: ("", vec![]),
        settings_extra: vec![],
        help_first: vec![],
        help_extra: vec![],
    });
    all.into_iter().filter(|(_, entries)| !entries.is_empty()).map(|(title, entries)| (title, without_gcode(entries))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(entries: &[Entry<Act>], a: Act) -> bool {
        entries.iter().any(|e| match e {
            Entry::Item(x, _, _) => *x == a,
            Entry::Sub(_, inner) => has(inner, a),
            _ => false,
        })
    }

    /// Every key of the menus belongs to one entry only, the layout is the common one, and nothing of a machine is left.
    #[test]
    fn the_menus_have_the_common_layout_without_a_machine_and_no_two_entries_share_a_key() {
        let m = menus();
        assert!(base_ui::menu::key_clashes(&m).is_empty(), "{:?}", base_ui::menu::key_clashes(&m));
        let titles: Vec<&str> = m.iter().map(|(t, _)| *t).collect();
        assert_eq!(titles, ["File", "Edit", "Insert", "Arrange", "View", "Settings", "Help"]);
        assert!(m.iter().all(|(_, e)| !has(e, Act::NoGcode)), "no View G-code file…");
        let file = &m[0].1;
        assert!(has(file, Act::ExportSvg) && has(file, Act::ExportCam) && has(file, Act::PageSetup));
        assert!([Act::ImportSvg, Act::ImportAi, Act::ImportDxf, Act::ImportLightburn, Act::ImportUgsd].iter().all(|a| has(file, *a)));
    }
}
