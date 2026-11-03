//! BaseCreator: a drawing program on baseCreator, with nothing of a machine. It looks like LightCreator, MillCreator and PenCreator (the
//! window frame, the menus, the tool palette, the Properties tab, the colour schemes and the languages are shared) and writes the same
//! project files: a project keeps the parts of the other programs, and they keep its part.
mod autosave_glue;
mod backup_ui;
mod layers_ui;
mod menu;
#[cfg(target_os = "macos")]
mod native_menu;
mod page_ui;
mod panes_glue;
mod plugins_glue;
#[cfg(test)]
mod screenshots;
mod section;
mod texts;
mod update_glue;
mod workspaces;

use base_core::geom::Xf;
use base_core::project::{self, Project};
use base_core::scene::{Kind, Scene};
use base_core::units::Units;
use base_ui::clipart_ui::{self, ClipGallery, ClipHost, ClipState};
use base_ui::dialogs::AboutInfo;
use base_ui::editor::{EditorCommands, EditorHost, EditorImages, EditorShapeOps, EditorState, Tool};
use base_ui::i18n::{self, tr, Lang};
use base_ui::menu::{MenuHost, Menus};
use base_ui::theme::{self, Scheme};
use base_ui::widgets::icon_button_ink;
use base_ui::window::{NEW_BLUE, OPEN_ORANGE, SAVE_GREEN};
use eframe::egui::{self, Color32, RichText};
use menu::{menus, Act, MAX_RECENT};
use section::{DrawLayer, Page};
use std::path::PathBuf;

const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));
const REPO_URL: &str = "https://github.com/michalsarna/baseCreator";
/// How many steps of undo are kept.
const UNDO_STEPS: usize = 100;
const ICON_256: &[u8] = include_bytes!("../../../assets/icons/icon-256.png");
const ICON_512: &[u8] = include_bytes!("../../../assets/icons/icon-512.png");

// Easter egg: around a holiday the program wears a festive icon (see `base_ui::seasonal`). The sources are the SVG files next to them.
const FESTIVE: base_ui::seasonal::Variants = base_ui::seasonal::Variants {
    christmas: (include_bytes!("../../../assets/icons/seasonal/christmas-256.png"), include_bytes!("../../../assets/icons/seasonal/christmas-512.png")),
    new_years_eve: (include_bytes!("../../../assets/icons/seasonal/newyear-256.png"), include_bytes!("../../../assets/icons/seasonal/newyear-512.png")),
    halloween: (include_bytes!("../../../assets/icons/seasonal/halloween-256.png"), include_bytes!("../../../assets/icons/seasonal/halloween-512.png")),
    easter: (include_bytes!("../../../assets/icons/seasonal/easter-256.png"), include_bytes!("../../../assets/icons/seasonal/easter-512.png")),
};

/// The 256 px and 512 px logo of the day; the tests (and so the screenshots) always get the normal one.
fn app_icon() -> base_ui::seasonal::Icon {
    let normal = (ICON_256, ICON_512);
    if cfg!(test) { normal } else { base_ui::seasonal::icon_today(&FESTIVE, normal) }
}

/// Read an SVG or PDF (by its content) into a new scene. Every outline becomes one shape on layer 0.
fn read_scene(data: &[u8]) -> Result<Scene, String> {
    let is_pdf = data.windows(5).take(1024).any(|w| w == b"%PDF-");
    let outlines = if is_pdf { base_core::pdf::parse(data)? } else { base_core::svg::parse(data)? };
    let mut scene = Scene::default();
    for o in outlines {
        scene.add(0, Kind::Bezier(o.contours), Xf::IDENTITY);
    }
    Ok(scene)
}

/// Undo and redo of whole scenes.
#[derive(Default)]
struct History {
    undo: Vec<Scene>,
    redo: Vec<Scene>,
}

impl History {
    /// Remember `before`, the state before a change.
    fn push(&mut self, before: Scene) {
        self.undo.push(before);
        if self.undo.len() > UNDO_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    /// The state to go back to, given the current one (which becomes the redo state).
    fn back(&mut self, now: &Scene) -> Option<Scene> {
        let s = self.undo.pop()?;
        self.redo.push(now.clone());
        Some(s)
    }
    fn forward(&mut self, now: &Scene) -> Option<Scene> {
        let s = self.redo.pop()?;
        self.undo.push(now.clone());
        Some(s)
    }
}

/// The icon of each action in the menus.
pub fn act_icon(a: Act) -> Option<&'static str> {
    match a {
        Act::Std(s) => Some(s.icon()),
        Act::New => Some("file-plus"),
        Act::Open => Some("folder-open"),
        Act::Save | Act::SaveAs => Some("save"),
        Act::PageSetup => Some("settings"),
        Act::ImportSvg | Act::ImportAi | Act::ImportDxf | Act::ImportLightburn | Act::ImportUgsd | Act::ImportPlugin => Some("file-input"),
        Act::ExportSvg | Act::ExportCam => Some("file-output"),
        Act::Plugins => Some("package-open"),
        Act::GenerateShape => Some("shapes"),
        Act::OnlineClipart => Some("globe"),
        Act::OpenRecent(_) | Act::ClearRecent | Act::NoGcode => None,
        Act::Quit => Some("x"),
        Act::ViewOptions => Some("sliders-horizontal"),
        Act::ProgramOptions => Some("settings"),
        Act::CheckUpdates => Some("download"),
        Act::DownloadBackup => Some("package"),
        Act::RestoreBackup => Some("package-open"),
        Act::OpenGithub => Some("github"),
        Act::About => Some("info"),
        Act::SetLang(_) | Act::SetScheme(_) => None,
    }
}

/// The tabs of the side panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SideTab {
    Properties,
    Layers,
    Clipart,
    Description,
}

impl SideTab {
    const IDS: [&'static str; 4] = ["properties", "layers", "clipart", "description"];

    fn id(self) -> &'static str {
        match self {
            SideTab::Properties => "properties",
            SideTab::Layers => "layers",
            SideTab::Clipart => "clipart",
            SideTab::Description => "description",
        }
    }

    fn from_id(id: &str) -> Option<SideTab> {
        [SideTab::Properties, SideTab::Layers, SideTab::Clipart, SideTab::Description].into_iter().find(|t| t.id() == id)
    }

    fn label(self) -> String {
        match self {
            SideTab::Properties => tr("Properties").to_string(),
            SideTab::Layers => tr("Layers").to_string(),
            SideTab::Clipart => clipart_ui::tab_label().to_string(),
            SideTab::Description => base_ui::description_ui::tab_label().to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    Start,
    Editor,
}

struct App {
    scene: Scene,
    ed: EditorState,
    history: History,
    scheme: Scheme,
    lang: Lang,
    status: String,
    screen: Screen,
    path: Option<PathBuf>,
    recent: Vec<PathBuf>,
    logo: Option<egui::TextureHandle>,
    side_tab: SideTab,
    /// The order of the tabs of the side panel and whether it is collapsed.
    tabs: base_ui::side_tabs::SideTabs,
    clip: ClipState,
    /// The free text of the project, kept for the next save.
    description: String,
    /// The parts of the project file that are not ours (laser, mill, pen), kept as they were read and written back.
    kept: Project,
    layers: Vec<DrawLayer>,
    /// The page of the shown drawing, the page a new drawing gets, and the units lengths are shown in.
    page: Page,
    new_page: Page,
    units: Units,
    show_page: bool,
    /// Counts the changes of the project.
    rev: u64,
    /// The list of layers shows all of them (not only those with shapes).
    show_all_layers: bool,
    autosave: base_core::autosave::Timer,
    /// The revision at the last save by hand, open or new; `rev` differing from it is unsaved work (see `is_dirty`).
    saved_rev: u64,
    /// The tabs of open drawings (the shown one is the live fields above), and the number of the recovery file of the shown one.
    tabs_state: workspaces::Tabs,
    /// The central area in panes (one pane unless the person split the view).
    panes: base_ui::work_panes::Panes,
    slot: usize,
    /// The program is being closed: every tab with unsaved work is asked about in turn.
    quitting: bool,
    /// The last tab was closed: the program closes too.
    close_program: bool,
    allow_close: bool,
    /// When the tabs that are not shown were last written.
    parked_saved_at: std::time::Instant,
    /// An unsaved project left by an earlier run (offered on the start screen).
    recovery: Vec<base_core::autosave::Found>,
    plugins: base_ui::plugins_ui::PluginsUi,
    show_options: bool,
    /// When the last Escape was pressed (a second one within half a second selects the select tool).
    esc_time: f64,
    /// Look for a newer release when the program starts.
    check_updates: bool,
    update: base_ui::dialogs::UpdateState,
    show_program_options: bool,
    show_about: bool,
    #[cfg(target_os = "macos")]
    native_menu: Option<native_menu::NativeMenu>,
}

/// The shapes of a DXF drawing as a scene: every layer of the DXF goes to a layer of the project that has no shapes yet (in the order
/// of the DXF); when there are more layers than free ones, the rest goes to the last layer. The second value says where they went.
fn dxf_scene(outlines: Vec<base_core::svg::Outline>, used: &std::collections::HashSet<usize>) -> (Scene, Option<String>) {
    let mut free: Vec<usize> = (0..section::LAYER_COUNT).filter(|i| !used.contains(i)).collect();
    if free.is_empty() {
        free.push(section::LAYER_COUNT - 1);
    }
    let names = base_core::svg::layer_names(&outlines);
    let mut scene = Scene::default();
    for o in outlines {
        let at = o.layer.as_ref().and_then(|l| names.iter().position(|n| n == l)).unwrap_or(0);
        scene.add(free[at.min(free.len() - 1)], Kind::Bezier(o.contours), Xf::IDENTITY);
    }
    let note = (names.len() > 1).then(|| {
        let spread = names.iter().enumerate().map(|(i, n)| format!("{n} → {}", free[i.min(free.len() - 1)] + 1)).collect::<Vec<_>>().join(", ");
        format!("layers: {spread}")
    });
    (scene, note)
}

/// A file that is imported into the drawing: a LightBurn project or a DXF.
fn is_import(p: &std::path::Path) -> bool {
    p.extension().is_some_and(|e| ["lbrn2", "lbrn", "ugsd", "dxf"].iter().any(|x| e.eq_ignore_ascii_case(x)))
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> App {
        let mut app = App::build(&cc.egui_ctx, cc.storage, cfg!(target_os = "macos"));
        if app.check_updates {
            app.update.start(&cc.egui_ctx, &update_glue::CONFIG, false);
        }
        app
    }

    /// Build the app without a window: used by `new` and by the tests.
    fn build(ctx: &egui::Context, storage: Option<&dyn eframe::Storage>, native_menu: bool) -> App {
        static TEXTS: std::sync::Once = std::sync::Once::new();
        TEXTS.call_once(|| i18n::register(&base_ui::texts::TEXTS));
        texts::register();
        let saved = |k: &str| storage.and_then(|st| st.get_string(k));
        let lang = saved("lang").and_then(|c| Lang::from_code(&c)).unwrap_or(Lang::En);
        let scheme = Scheme::from_saved(saved("scheme").as_deref());
        i18n::set_lang(lang);
        base_ui::fonts::install(ctx);
        base_ui::icons::install(ctx);
        theme::apply(ctx, scheme);
        let on = |k: &str, default: bool| saved(k).map(|v| v == "1").unwrap_or(default);
        let ed = EditorState { show_grid: on("grid_main_on", true), snap: on("snap", false), snap_objects: on("snap_objects", false), grid_prefs: saved("grid_prefs").and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default(), grid: saved("grid_main_mm").and_then(|v| v.parse::<f64>().ok()).filter(|g| *g >= 0.1).unwrap_or(EditorState::new().grid), ..EditorState::new() };
        let recent = saved("recent_files").map(|s| s.lines().map(PathBuf::from).take(MAX_RECENT).collect()).unwrap_or_default();
        #[cfg(not(target_os = "macos"))]
        let _ = native_menu;
        let new_page = saved("page").and_then(|j| serde_json::from_str::<Page>(&j).ok()).map(Page::sane).unwrap_or_default();
        let mut app = App {
            scene: Scene::default(),
            ed,
            history: History::default(),
            scheme,
            lang,
            status: tr("Ready").into(),
            screen: Screen::Start,
            path: None,
            recent,
            logo: base_ui::start::load_logo(ctx, app_icon().0, "basecreator-logo"),
            side_tab: SideTab::Properties,
            tabs: base_ui::side_tabs::SideTabs::new(&SideTab::IDS, &saved("tab_order").unwrap_or_default(), saved("side_collapsed").is_some_and(|v| v == "1")),
            clip: ClipState::new(base_core::clipart::Library::open(data_paths().gallery.clone())),
            description: String::new(),
            kept: Project::default(),
            layers: section::default_layers(),
            page: new_page,
            new_page,
            units: saved("units").and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default(),
            show_page: false,
            rev: 0,
            show_all_layers: false,
            autosave: Default::default(),
            saved_rev: 0,
            tabs_state: Default::default(),
            panes: base_ui::work_panes::Panes::from_json(&saved("panes").unwrap_or_default()),
            slot: 0,
            quitting: false,
            close_program: false,
            allow_close: false,
            parked_saved_at: std::time::Instant::now(),
            recovery: base_core::autosave::find_all(&data_paths().base, "basecreator"),
            plugins: base_ui::plugins_ui::PluginsUi::new("basecreator", &data_paths().base),
            show_options: false,
            esc_time: -10.0,
            check_updates: on("check_updates", true),
            update: Default::default(),
            show_program_options: false,
            show_about: false,
            #[cfg(target_os = "macos")]
            native_menu: native_menu.then(|| native_menu::install(ctx)),
        };
        app.after_plugins_changed();
        app
    }

    /// What is kept between runs.
    fn settings_pairs(&self) -> Vec<(&'static str, String)> {
        let flag = |b: bool| if b { "1" } else { "0" }.to_string();
        vec![
            ("lang", self.lang.code().to_owned()),
            ("scheme", self.scheme.id().to_owned()),
            ("grid_main_on", flag(self.ed.show_grid)),
            ("snap", flag(self.ed.snap)),
            ("snap_objects", flag(self.ed.snap_objects)),
            ("grid_prefs", serde_json::to_string(&self.ed.grid_prefs).unwrap_or_default()),
            ("grid_main_mm", self.ed.grid.to_string()),
            ("page", serde_json::to_string(&self.new_page).unwrap_or_default()),
            ("units", serde_json::to_string(&self.units).unwrap_or_default()),
            ("check_updates", flag(self.check_updates)),
            ("tab_order", self.tabs.order_string()),
            ("panes", self.panes.to_json()),
            ("side_collapsed", flag(self.tabs.collapsed)),
            ("recent_files", self.recent.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>().join("\n")),
        ]
    }

    fn add_recent(&mut self, p: &std::path::Path) {
        let p = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        self.recent.retain(|r| *r != p);
        self.recent.insert(0, p);
        self.recent.truncate(MAX_RECENT);
    }

    /// A new, empty design.
    fn new_design(&mut self) {
        self.scene = Scene::default();
        self.history = History::default();
        self.path = None;
        self.description.clear();
        self.kept = Project::default();
        self.layers = section::default_layers();
        self.page = self.new_page;
        self.touch_rev();
        self.autosave_in_sync();
        self.ed.sel.clear();
        self.ed.node_sel.clear();
        self.ed.view.need_fit = true;
        self.screen = Screen::Editor;
        self.status = tr("Ready").into();
    }

    fn open(&mut self, path: PathBuf) {
        // A picture goes on the page of the drawing (the import dialog adjusts it, and it can be traced into outlines).
        if base_ui::editor::is_image_file(&path) {
            if self.screen == Screen::Start {
                self.new_design();
            }
            return self.open_image_dialog(path);
        }
        // A LightBurn project or a DXF is imported into the drawing, not opened.
        if is_import(&path) {
            return self.import_file(path);
        }
        if self.try_plugin_import(&path) {
            return;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let is_project = path.extension().is_some_and(|e| e.eq_ignore_ascii_case(project::EXTENSION));
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|d| {
            if is_project {
                let text = String::from_utf8(d).map_err(|_| "this is not a project file".to_string())?;
                Project::from_json(&text).map(|p| (p.scene.clone(), Some(p)))
            } else {
                read_scene(&d).map(|s| (s, None))
            }
        });
        match read {
            Ok((scene, file)) => {
                // The part of this program. A drawing and a project of another program get the defaults. It is read before a tab is
                // made: a project that cannot be read leaves no empty tab behind.
                let loaded = match &file {
                    Some(p) => section::load(p),
                    None => section::load(&Project::default()),
                };
                let loaded = match loaded {
                    Ok(l) => l,
                    Err(e) => {
                        self.status = format!("Could not open {name}: {e}");
                        return;
                    }
                };
                // A drawing goes into a tab of its own (a blank tab is used as it is); a project that is open is shown.
                if !self.tab_for_file(&path, is_project) {
                    return;
                }
                self.layers = loaded.layers;
                // A drawing without a page of its own gets the usual one, big enough to hold it.
                self.page = loaded.page.unwrap_or_else(|| {
                    let reach = scene.shapes.iter().filter_map(|s| s.bounds()).fold((0.0f64, 0.0f64), |r, b| (r.0.max(b.max.x), r.1.max(b.max.y)));
                    self.new_page.holding(reach.0, reach.1)
                });
                self.touch_rev();
                // Sections of other programs wait here until the next save; a drawing from SVG or PDF has none.
                self.description = file.as_ref().map(|p| p.description.clone()).unwrap_or_default();
                self.kept = file.unwrap_or_default();
                self.kept.scene = Scene::default();
                self.status = format!("Opened {name}: {} shapes", scene.shapes.len());
                self.scene = scene;
                self.history = History::default();
                self.ed.sel.clear();
                self.ed.node_sel.clear();
                self.ed.view.need_fit = true;
                self.screen = Screen::Editor;
                self.add_recent(&path);
                // An SVG or PDF is not saved over: the next save asks for a project name.
                self.path = is_project.then_some(path);
                self.autosave_in_sync();
            }
            Err(e) => self.status = format!("Could not open {name}: {e}"),
        }
    }

    /// Import a LightBurn project, a UGS design or a DXF into the drawing: its shapes are added, selected, and undoable.
    fn import_file(&mut self, path: PathBuf) {
        if self.screen == Screen::Start {
            self.new_design();
        }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let is_ugsd = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("ugsd"));
        let is_dxf = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("dxf"));
        let area_h = self.work_size().1;
        // A DXF drawing: its layers go to the layers of the project that have no shapes yet, one after the other.
        let used_layers: std::collections::HashSet<usize> = self.scene.shapes.iter().map(|s| s.layer).collect();
        let last = section::LAYER_COUNT - 1;
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|d| {
            if is_dxf {
                let dxf = base_core::dxf::parse(&d, area_h)?;
                let (scene, note) = dxf_scene(dxf.outlines, &used_layers);
                let mut warnings = dxf.warnings;
                warnings.extend(note);
                return Ok((scene, warnings));
            }
            let text = String::from_utf8(d).map_err(|_| "this is not a text file".to_string())?;
            if is_ugsd {
                let imp = base_core::ugsd::import(&text, area_h, section::LAYER_COUNT)?;
                return Ok((imp.scene, imp.warnings));
            }
            let imp = base_core::lightburn::import(&text, area_h)?;
            Ok((imp.scene, imp.warnings))
        });
        match read {
            Ok((scene, warnings)) => {
                self.checkpoint();
                let n = scene.shapes.len();
                let ids = self.scene.append(scene, last);
                self.ed.sel = ids.into_iter().collect();
                self.ed.node_sel.clear();
                self.touch_rev();
                self.ed.view.need_fit = true;
                self.status = format!("Imported {n} shapes from {name}");
                if !warnings.is_empty() {
                    self.status.push_str(&format!(" (some parts could not be used: {})", warnings.join("; ")));
                }
            }
            Err(e) => self.status = format!("Could not import {name}: {e}"),
        }
    }

    /// The project as it is saved: the drawing, the description and the part of this program, with the sections of other programs as
    /// they were read.
    fn to_project(&self) -> Project {
        let mut p = Project::new(self.scene.clone());
        p.description = self.description.clone();
        p.set_section(section::SECTION, &section::DrawSection { format: section::SECTION_FORMAT, page: self.page, layers: self.layers.clone() });
        p.restore_sections(&self.kept.other_sections(section::SECTION));
        p.restore_extra(self.kept.extra());
        p
    }

    fn save(&mut self, as_new: bool) {
        let path = match (&self.path, as_new) {
            (Some(p), false) => Some(p.clone()),
            _ => rfd::FileDialog::new().add_filter("Project", &[project::EXTENSION]).set_file_name("untitled.cre").save_file(),
        };
        let Some(path) = path else { return };
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match std::fs::write(&path, self.to_project().to_json()) {
            Ok(()) => {
                self.status = format!("Saved {name}");
                self.add_recent(&path);
                self.path = Some(path);
                // The project has a name now: what was kept for it while it had none is not needed.
                self.autosave_saved_by_hand();
            }
            Err(e) => self.status = format!("Could not save {name}: {e}"),
        }
    }

    fn open_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Project, SVG, PDF, Illustrator, DXF", &[project::EXTENSION, "svg", "pdf", "ai", "dxf"]).pick_file() {
            self.open(p);
        }
    }

    /// File > Import > Import DXF…: the lines, arcs and curves of the drawing are added to the drawing.
    fn import_dxf_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("DXF", &["dxf"]).pick_file() {
            self.open(p);
        }
    }

    /// File > Import > Import UGS design…: the shapes of the file are added to the drawing, one layer for each way of cutting. Its feeds
    /// and depths mean nothing here and are not taken.
    fn import_ugsd_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("UGS Designer", &["ugsd"]).pick_file() {
            self.open(p);
        }
    }

    /// File > Import > Import SVG… and Import Adobe Illustrator / PDF…: the paths are added to the drawing. (*Open…* makes a new tab of them.)
    fn import_drawing_dialog(&mut self, what: &str, extensions: &[&str]) {
        if let Some(p) = rfd::FileDialog::new().add_filter(what, extensions).pick_file() {
            if self.screen == Screen::Start {
                self.new_design();
            }
            self.import_drawing_path(p);
        }
    }

    /// File > Import > Import LightBurn project…: the shapes of the project are added to the drawing.
    fn import_lightburn_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("LightBurn", &["lbrn2", "lbrn"]).pick_file() {
            self.open(p);
        }
    }

    /// The first name of an exported file: that of the project.
    fn file_stem(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_stem()).map_or("drawing".to_string(), |s| s.to_string_lossy().into_owned())
    }

    /// The size of the page in mm.
    fn work_size_mm(&self) -> (f64, f64) {
        (self.page.w, self.page.h)
    }

    /// Something in the project changed.
    fn touch_rev(&mut self) {
        self.rev += 1;
    }

    fn undo(&mut self) {
        if let Some(s) = self.history.back(&self.scene) {
            self.scene = s;
            self.touch_rev();
            self.forget_missing();
        }
    }

    fn redo(&mut self) {
        if let Some(s) = self.history.forward(&self.scene) {
            self.scene = s;
            self.touch_rev();
            self.forget_missing();
        }
    }

    /// Drop selected shapes that the undo took away.
    fn forget_missing(&mut self) {
        let scene = &self.scene;
        self.ed.sel.retain(|id| scene.shape(*id).is_some());
        self.ed.node_sel.retain(|(id, _, _)| scene.shape(*id).is_some());
        self.ed.active_seg = None;
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        // The Ctrl/Cmd keys of the menus. The native bar of macOS has only the keys of the entries it owns (New, Open, Save, Save as); the
        // others are taken here, on every system.
        #[cfg(target_os = "macos")]
        let native_bar = self.native_menu.is_some();
        #[cfg(not(target_os = "macos"))]
        let native_bar = false;
        // The native menu module exists only on macOS.
        #[cfg(target_os = "macos")]
        let owned = |a: Act| native_bar && native_menu::owns_shortcut(a);
        #[cfg(not(target_os = "macos"))]
        let owned = |_: Act| native_bar;
        if let Some(a) = base_ui::menu::shortcut_act(ctx, &crate::menu::menus(), |a| base_ui::menu::MenuHost::act_enabled(self, a) && !owned(a)) {
            base_ui::menu::MenuHost::run_act(self, ctx, a);
            return;
        }
        // Ctrl+1 … Ctrl+0 show a tab, Ctrl+W closes the shown one.
        match base_ui::doc_tabs::shortcut(ctx) {
            Some(base_ui::doc_tabs::TabKey::Select(i)) => self.switch_to(i),
            Some(base_ui::doc_tabs::TabKey::Close) => self.close_tab_request(),
            None => {}
        }
        let (cmd, shift) = ctx.input(|i| (i.modifiers.command, i.modifiers.shift));
        let pressed = |k: egui::Key| ctx.input(|i| i.key_pressed(k));
        if cmd {
            #[cfg(target_os = "macos")]
            let native = self.native_menu.is_some();
            #[cfg(not(target_os = "macos"))]
            let native = false;
            if pressed(egui::Key::Z) {
                if shift {
                    self.redo();
                } else {
                    self.undo();
                }
            } else if pressed(egui::Key::Y) {
                self.redo();
            } else if pressed(egui::Key::A) {
                self.select_all();
            } else if pressed(egui::Key::C) {
                self.copy();
            } else if pressed(egui::Key::V) {
                self.paste();
            } else if pressed(egui::Key::D) {
                self.duplicate();
            } else if pressed(egui::Key::G) {
                if shift {
                    self.ungroup_selection();
                } else {
                    self.group_selection();
                }
            } else if !native && pressed(egui::Key::N) {
                self.new_tab();
            } else if !native && pressed(egui::Key::S) {
                self.save(shift);
            } else if !native && pressed(egui::Key::O) {
                self.open_dialog();
            } else if !native && pressed(egui::Key::Q) {
                self.request_close(ctx);
            }
        } else if pressed(egui::Key::Delete) || pressed(egui::Key::Backspace) {
            if self.ed.tool == Tool::Node && !self.ed.node_sel.is_empty() {
                self.delete_nodes();
            } else {
                self.delete_selection();
            }
        } else if pressed(egui::Key::Escape) {
            let now = ctx.input(|i| i.time);
            self.note_escape(now);
            self.ed.sel.clear();
            self.ed.node_sel.clear();
            self.ed.pen_pts.clear();
        } else {
            for (tool, _, key) in Tool::ALL {
                if pressed(key) {
                    self.ed.tool = tool;
                }
            }
        }
    }

    /// A second Escape within half a second goes back to the select tool.
    pub(crate) fn note_escape(&mut self, now: f64) {
        if now - self.esc_time < 0.5 {
            self.ed.tool = Tool::Select;
            self.esc_time = -10.0;
        } else {
            self.esc_time = now;
        }
    }

    #[cfg(target_os = "macos")]
    fn native_menu_frame(&mut self, ctx: &egui::Context) {
        let Some(mut nm) = self.native_menu.take() else { return };
        for a in nm.take_actions() {
            self.run_act(ctx, a);
        }
        nm.sync(|a| self.act_checked(a), |a| self.act_enabled(a), &self.recent);
        self.native_menu = Some(nm);
    }

    /// A file dropped on the window is opened.
    fn handle_drops(&mut self, ctx: &egui::Context) {
        let files: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if let Some(p) = files.into_iter().next() {
            self.open(p);
        }
    }

    /// The bar across the top of the window.
    fn title_bar(&mut self, ui: &mut egui::Ui) {
        let mut title = format!("BaseCreator {VERSION}");
        if let Some(name) = self.path.as_ref().and_then(|p| p.file_name()) {
            title.push_str(&format!("  —  {}", name.to_string_lossy()));
        }
        if base_ui::window::title_bar(ui, &title, self.logo.as_ref()) {
            let ctx = ui.ctx().clone();
            self.request_close(&ctx);
        }
    }

    /// The bar under the menu: the tool, new and open, the grid, and the size of the selection.
    fn control_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            base_ui::editor::control_name(self, ui);
            ui.separator();
            let ctx = ui.ctx().clone();
            for (act, icon, tip, ink) in [(Act::New, "file-plus", "New", NEW_BLUE), (Act::Open, "folder-open", "Open…", OPEN_ORANGE), (Act::Save, "save", "Save", SAVE_GREEN)] {
                if icon_button_ink(ui, icon, &tr(tip), true, 28.0, ink) {
                    self.run_act(&ctx, act);
                }
            }
            ui.separator();
            base_ui::editor::grid_toggles(self, ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                base_ui::editor::selection_size(self, ui);
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.label(&self.status);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                base_ui::editor::zoom_and_cursor(self, ui);
                ui.separator();
                let page = format!("{} × {}", base_ui::units_ui::fmt_len(self.units, self.page.w, 0), base_ui::units_ui::fmt_len(self.units, self.page.h, 0));
                if ui.add(egui::Button::new(RichText::new(page).color(theme::text_dim())).frame(false)).on_hover_text(tr("Page setup…")).clicked() {
                    self.show_page = true;
                }
            });
        });
    }

    /// The label of a tab.
    fn tab_label(&self, id: &str) -> String {
        SideTab::from_id(id).map(SideTab::label).unwrap_or_default()
    }

    /// A click on the tab `id`.
    fn pick_side_tab(&mut self, id: &'static str) {
        if let Some(t) = SideTab::from_id(id) {
            self.side_tab = t;
        }
    }

    /// The collapsed side panel: a narrow strip with the names of the tabs; a click on one opens the panel on it.
    fn side_strip(&mut self, ui: &mut egui::Ui) {
        let labels: std::collections::HashMap<&str, String> = SideTab::IDS.iter().copied().map(|id| (id, self.tab_label(id))).collect();
        let label = |id: &str| labels.get(id).cloned().unwrap_or_default();
        if let Some(t) = self.tabs.strip(ui, self.side_tab.id(), &[], &label) {
            self.pick_side_tab(t);
        }
    }

    /// The tabs of the right panel.
    fn side_panel(&mut self, ui: &mut egui::Ui) {
        let labels: std::collections::HashMap<&str, String> = SideTab::IDS.iter().copied().map(|id| (id, self.tab_label(id))).collect();
        let label = |id: &str| labels.get(id).cloned().unwrap_or_default();
        if let Some(t) = self.tabs.bar(ui, self.side_tab.id(), &[], &label) {
            self.pick_side_tab(t);
        }
        ui.separator();
        match self.side_tab {
            SideTab::Properties => {
                egui::ScrollArea::vertical().id_salt("tab_props").auto_shrink([false, false]).show(ui, |ui| {
                    if !base_ui::editor::properties_sections(self, ui) {
                        ui.label(RichText::new(tr("Nothing selected")).color(theme::text_dim()));
                    }
                });
            }
            SideTab::Layers => self.layers_panel(ui),
            SideTab::Clipart => self.clipart_panel(ui),
            SideTab::Description => {
                if base_ui::description_ui::panel(ui, &mut self.description) {
                    self.touch_rev();
                }
            }
        }
    }

    fn start_screen(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                base_ui::start::start_header(ui, self.logo.as_ref(), "BaseCreator", VERSION);
                ui.add_space(18.0);
                if !self.recovery.is_empty() {
                    if let Some(choice) = base_ui::autosave_ui::banner_all(ui, &self.recovery.clone(), 460.0) {
                        self.recovery_choice(choice);
                    }
                    ui.add_space(12.0);
                }
                ui.label(RichText::new(tr("Start")).size(16.0).strong());
                ui.label(RichText::new(tr("Open a drawing or start an empty design.")).color(theme::text_dim()));
                ui.add_space(8.0);
                let frame = egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(12));
                frame.show(ui, |ui| {
                    ui.set_width(460.0);
                    let new = egui::Button::image_and_text(base_ui::icons::slot(Some("file-plus"), Color32::WHITE), RichText::new(tr("New")).color(Color32::WHITE).strong()).fill(theme::accent());
                    if ui.add_sized([ui.available_width(), 30.0], new).clicked() {
                        self.run_act(&ctx, Act::New);
                    }
                    ui.add_space(4.0);
                    let open = egui::Button::image_and_text(base_ui::icons::slot(Some("folder-open"), theme::text()), tr("Open…"));
                    if ui.add_sized([ui.available_width(), 30.0], open).clicked() {
                        self.run_act(&ctx, Act::Open);
                    }
                    if !self.recent.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new(tr("Open recent")).color(theme::text_dim()));
                        let mut chosen = None;
                        for (i, p) in self.recent.iter().enumerate().take(5) {
                            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            if ui.add_sized([ui.available_width(), 24.0], egui::Button::new(name)).on_hover_text(p.display().to_string()).clicked() {
                                chosen = Some(i as u8);
                            }
                        }
                        if let Some(i) = chosen {
                            self.run_act(&ctx, Act::OpenRecent(i));
                        }
                    }
                });
                ui.add_space(14.0);
                let (l, s) = base_ui::start::start_prefs(ui, self.lang, self.scheme);
                if let Some(l) = l {
                    self.run_act(&ctx, Act::SetLang(l));
                }
                if let Some(s) = s {
                    self.run_act(&ctx, Act::SetScheme(s));
                }
            });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        self.close_dialogs(ctx);
        self.image_dialogs(ctx);
        let mut show = self.show_options;
        base_ui::editor::view_options_window(self, ctx, &mut show);
        self.show_options = show;
        self.clip_dialogs(ctx);
        self.online_clipart_window(ctx);
        self.plugin_windows(ctx);
        self.page_window(ctx);
        self.update.tick(ctx, &update_glue::CONFIG);
        self.program_options_window(ctx);
        let about = AboutInfo {
            title: "About BaseCreator",
            name: "BaseCreator",
            version: VERSION,
            lines: vec!["Open-source drawing program.".into(), "Written in Rust with egui.".into()],
            url: REPO_URL,
            site: REPO_URL,
            licence: "MIT OR Apache-2.0",
            credits: include_str!("../../../CREDITS.md"),
        };
        base_ui::dialogs::about_window(ctx, &mut self.show_about, &about, self.logo.as_ref());
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.autosave_tick(&ctx, false);
        #[cfg(target_os = "macos")]
        self.native_menu_frame(&ctx);
        self.title_bar(ui);
        base_ui::window::window_edges(&ctx);
        #[cfg(target_os = "macos")]
        base_ui::window::window_edge_drag(&ctx);
        self.handle_drops(&ctx);
        self.close_requested_tick(&ctx);
        if self.screen == Screen::Start {
            self.start_screen(ui);
            self.dialogs(&ctx);
            return;
        }
        self.shortcuts(&ctx);
        let chrome = egui::Frame::new().fill(theme::panel()).stroke(egui::Stroke::new(1.0, theme::border())).inner_margin(egui::Margin::symmetric(8, 4));
        #[cfg(target_os = "macos")]
        let in_window_menu = self.native_menu.is_none();
        #[cfg(not(target_os = "macos"))]
        let in_window_menu = true;
        if in_window_menu {
            egui::Panel::top("menu").frame(chrome).show(ui, |ui| base_ui::menu::menu_bar(self, ui));
        }
        egui::Panel::top("controls").frame(chrome).exact_size(36.0).show(ui, |ui| self.control_bar(ui));
        egui::Panel::bottom("status").frame(chrome).exact_size(26.0).show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("tools").frame(chrome).exact_size(52.0).resizable(false).show(ui, |ui| base_ui::editor::tool_palette(self, ui, None));
        let side = if self.tabs.collapsed {
            egui::Panel::right("side_strip").frame(chrome.inner_margin(egui::Margin::same(3))).exact_size(base_ui::side_tabs::STRIP_W).resizable(false).show(ui, |ui| self.side_strip(ui))
        } else {
            egui::Panel::right("side").frame(chrome.inner_margin(egui::Margin::same(8))).default_size(410.0).resizable(!base_ui::window::pointer_over_window(ui.ctx())).show(ui, |ui| self.side_panel(ui))
        };
        base_ui::widgets::panel_edge_highlight(ui.ctx(), side.response.rect, base_ui::widgets::Edge::Left);
        egui::Panel::right("arrange_bar").frame(chrome.inner_margin(egui::Margin::symmetric(2, 4))).exact_size(40.0).resizable(false).show(ui, |ui| base_ui::editor::arrange_bar(self, ui));
        egui::Panel::bottom("layer_bar").frame(chrome).exact_size(36.0).show(ui, |ui| base_ui::layer_bar::layer_bar(self, ui));
        egui::CentralPanel::no_frame().show(ui, |ui| self.central_panes(ui));
        self.dialogs(&ctx);
        self.pane_windows(&ctx);
    }
}

impl ClipHost for App {
    fn clip(&self) -> &ClipState {
        &self.clip
    }
    fn clip_mut(&mut self) -> &mut ClipState {
        &mut self.clip
    }
}

impl EditorHost for App {
    fn has_machine(&self) -> bool {
        false
    }
    fn show_properties_tab(&mut self) {
        self.side_tab = SideTab::Properties;
    }
    fn panes_mut(&mut self) -> Option<&mut base_ui::work_panes::Panes> {
        Some(&mut self.panes)
    }
    fn ed(&self) -> &EditorState {
        &self.ed
    }
    fn ed_mut(&mut self) -> &mut EditorState {
        &mut self.ed
    }
    fn scene(&self) -> &Scene {
        &self.scene
    }
    fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
    fn layer_visible(&self, layer: usize) -> bool {
        self.layers.get(layer).is_none_or(|l| l.visible)
    }
    fn checkpoint(&mut self) {
        let before = self.scene.clone();
        self.history.push(before);
    }
    fn touch(&mut self) {
        self.touch_rev();
    }
    fn undo(&mut self) {
        App::undo(self)
    }
    fn redo(&mut self) {
        App::redo(self)
    }
    fn can_undo(&self) -> bool {
        !self.history.undo.is_empty()
    }
    fn can_redo(&self) -> bool {
        !self.history.redo.is_empty()
    }
    fn drop_checkpoint(&mut self) {
        self.history.undo.pop();
    }
    fn open_shape_generators(&mut self) {
        self.plugins.gen_open = true;
    }
    fn open_online_clipart_search(&mut self, ctx: &egui::Context) {
        self.side_tab = SideTab::Clipart;
        base_ui::clipart_ui::ClipGallery::open_online_clipart(self, ctx);
    }
    fn say(&mut self, msg: String) {
        self.status = msg;
    }
    fn said(&self) -> &str {
        &self.status
    }
    fn work_size(&self) -> (f64, f64) {
        self.work_size_mm()
    }
    fn units(&self) -> Units {
        self.units
    }
    fn workspace_fill(&self) -> egui::Color32 {
        self.ed.grid_prefs.workspace_fill()
    }
    fn bed_fill(&self) -> egui::Color32 {
        self.ed.grid_prefs.bed_fill(self.scheme.is_dark())
    }
    fn shape_color(&self, layer: usize) -> Option<egui::Color32> {
        self.layer_visible(layer).then(|| self.layer_color(layer))
    }
    fn layer_color(&self, layer: usize) -> egui::Color32 {
        self.layers.get(layer).map_or(egui::Color32::from_rgb(0x20, 0x20, 0x30), |l| l.color32())
    }
    fn layer_title(&self, layer: usize) -> String {
        self.layers.get(layer).map_or_else(|| format!("Layer {}", layer + 1), |l| l.name.clone())
    }
}

impl MenuHost for App {
    type Act = Act;
    fn menus(&self) -> Menus<Act> {
        menus()
    }
    fn act_enabled(&self, a: Act) -> bool {
        match a {
            Act::Std(s) => self.screen == Screen::Editor && s.enabled(self),
            Act::Save | Act::SaveAs | Act::PageSetup | Act::ExportSvg | Act::ExportCam => self.screen == Screen::Editor,
            _ => true,
        }
    }
    fn act_checked(&self, a: Act) -> Option<bool> {
        match a {
            Act::Std(s) => s.checked(self),
            Act::SetLang(l) => Some(self.lang == l),
            Act::SetScheme(s) => Some(self.scheme == s),
            _ => None,
        }
    }
    fn act_icon(&self, a: Act) -> Option<&'static str> {
        act_icon(a)
    }
    fn recent_files(&self) -> Vec<PathBuf> {
        self.recent.clone()
    }
    fn open_recent_act(&self, i: usize) -> Option<Act> {
        (i < self.recent.len()).then_some(Act::OpenRecent(i as u8))
    }
    fn clear_recent_act(&self) -> Option<Act> {
        Some(Act::ClearRecent)
    }
    fn run_act(&mut self, ctx: &egui::Context, a: Act) {
        match a {
            Act::New => {
                self.new_tab();
            }
            Act::Open => self.open_dialog(),
            Act::Save => self.save(false),
            Act::SaveAs => self.save(true),
            Act::PageSetup => self.show_page = true,
            Act::ImportSvg => self.import_drawing_dialog("SVG", &["svg"]),
            Act::ImportAi => self.import_drawing_dialog("Adobe Illustrator / PDF", &["ai", "pdf"]),
            Act::ImportDxf => self.import_dxf_dialog(),
            Act::ImportLightburn => self.import_lightburn_dialog(),
            Act::ImportUgsd => self.import_ugsd_dialog(),
            Act::ImportPlugin => self.import_with_plugin_dialog(),
            Act::ExportSvg => {
                let stem = self.file_stem();
                self.export_drawing_svg(&stem);
            }
            Act::ExportCam => {
                let stem = self.file_stem();
                self.export_drawing_cam(&stem);
            }
            Act::NoGcode => {}
            Act::Plugins => self.plugins.open = true,
            Act::GenerateShape => self.plugins.gen_open = true,
            Act::OnlineClipart => self.open_online_clipart_search(ctx),
            Act::OpenRecent(i) => {
                if let Some(p) = self.recent.get(i as usize).cloned() {
                    self.open(p);
                }
            }
            Act::ClearRecent => self.recent.clear(),
            Act::Quit => self.request_close(ctx),
            Act::Std(s) => {
                if self.screen == Screen::Editor {
                    s.run(self)
                }
            }
            Act::ViewOptions => self.show_options = true,
            Act::ProgramOptions => self.show_program_options = true,
            Act::CheckUpdates => self.update.start(ctx, &update_glue::CONFIG, true),
            Act::SetLang(l) => {
                self.lang = l;
                i18n::set_lang(l);
            }
            Act::SetScheme(s) => {
                self.scheme = s;
                theme::apply(ctx, s);
            }
            Act::DownloadBackup => self.download_backup(),
            Act::RestoreBackup => self.restore_backup_dialog(ctx),
            Act::OpenGithub => ctx.open_url(egui::OpenUrl::new_tab(REPO_URL)),
            Act::About => self.show_about = true,
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Called every half minute and when the program closes: a last chance to keep the work.
        self.autosave_tick(&egui::Context::default(), true);
        for (k, v) in self.settings_pairs() {
            storage.set_string(k, v);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.draw(ui);
    }
}

impl base_ui::layer_bar::LayerBarHost for App {
    fn layer_label(&self, layer: usize) -> String {
        self.layers.get(layer).map(|l| l.name.clone()).unwrap_or_default()
    }
    fn layer_total(&self) -> usize {
        self.layers.len()
    }
    fn open_layer_options(&mut self, layer: usize) {
        self.ed.active_layer = layer;
        self.side_tab = SideTab::Layers;
    }
}

/// Where this program keeps its data: the settings in its own folder, the gallery and the plugins shared with the other programs
/// (see `base_core::paths`).
pub fn data_paths() -> &'static base_core::paths::DataPaths {
    #[cfg(test)]
    {
        // Tests never touch the real folders, and each test (a thread of its own) has a folder of its own.
        thread_local! {
            static PATHS: &'static base_core::paths::DataPaths = Box::leak(Box::new(base_core::paths::prepare_in(
                &std::env::temp_dir().join(format!("basecreator-test-data-{}-{:?}", std::process::id(), std::thread::current().id())),
                "basecreator",
            )));
        }
        PATHS.with(|p| *p)
    }
    #[cfg(not(test))]
    {
        static PATHS: std::sync::OnceLock<base_core::paths::DataPaths> = std::sync::OnceLock::new();
        PATHS.get_or_init(|| base_core::paths::prepare("basecreator"))
    }
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title(format!("BaseCreator {VERSION}"))
        .with_app_id("basecreator")
        .with_inner_size([1360.0, 860.0])
        .with_min_inner_size([900.0, 560.0])
        // The application draws its own flat, square frame, as the other programs do.
        .with_decorations(false);
    if let Some(icon) = base_ui::start::window_icon(app_icon().1) {
        viewport = viewport.with_icon(icon);
    }
    let opts = eframe::NativeOptions { viewport, persistence_path: Some(data_paths().settings.clone()), ..Default::default() };
    eframe::run_native(
        "BaseCreator",
        opts,
        Box::new(|cc| {
            #[cfg(target_os = "macos")]
            native_menu::set_dock_icon(app_icon().1);
            let mut app = App::new(cc);
            if let Some(p) = std::env::args().nth(1) {
                app.open(p.into());
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests;
