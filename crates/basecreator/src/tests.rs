use super::*;
use base_core::scene::Kind;
use base_ui::menu::Entry;

const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="10" y="10" width="40" height="20" stroke="#ff0000" fill="none"/><circle cx="70" cy="70" r="10"/></svg>"##;

fn app() -> App {
    let mut a = App::build(&egui::Context::default(), None, false);
    a.screen = Screen::Editor;
    a
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("basecreator-{name}-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Without the "links" feature of eframe, open_url silently does nothing: no link of the program would open.
#[test]
fn eframe_has_the_links_feature_so_that_links_open() {
    let manifest = include_str!("../Cargo.toml");
    let eframe = manifest.lines().find(|l| l.starts_with("eframe")).unwrap();
    assert!(eframe.contains("\"links\""), "{eframe}");
}

/// The numbers of the program are the tags of baseCreator, and the update check looks where its releases are.
#[test]
fn the_version_is_the_one_of_the_base_and_the_update_check_asks_the_repository_of_the_base() {
    assert_eq!(VERSION.trim_start_matches('v'), base_core::registry::base_version());
    assert_eq!(update_glue::CONFIG.repo, "michalsarna/baseCreator");
    assert_eq!(update_glue::CONFIG.version, VERSION);
}

/// A project of another program is opened, saved and still has everything of that program; the page of this program is added.
#[test]
fn a_project_of_the_machine_programs_keeps_its_parts_after_a_save_here() {
    let dir = temp_dir("sections");
    let file = dir.join("job.cre");
    let mut scene = Scene::default();
    scene.add(3, Kind::Rect { w: 500.0, h: 20.0 }, Xf::translate(10.0, 10.0));
    let mut p = Project::new(scene);
    p.description = "from the laser".into();
    for name in ["laser", "mill", "pen"] {
        p.set_section(name, &serde_json::json!({ "format": 1, "who": name }));
    }
    std::fs::write(&file, p.to_json()).unwrap();

    let mut a = app();
    a.open(file.clone());
    assert_eq!(a.scene.shapes.len(), 1, "{}", a.status);
    assert_eq!(a.description, "from the laser");
    assert_eq!(a.page.h, Page::default().h);
    assert!(a.page.w >= 510.0, "the page holds the drawing: {:?}", a.page);
    a.layers[3].name = "Cut".into();
    a.save(false);

    let back = Project::from_json(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let mut names = back.section_names();
    names.sort();
    assert_eq!(names, ["draw", "laser", "mill", "pen"]);
    for name in ["laser", "mill", "pen"] {
        assert_eq!(back.section::<serde_json::Value>(name).unwrap().unwrap()["who"], name);
    }
    // And the same file opened again has the layer and the page that were saved.
    let mut b = app();
    b.open(file);
    assert_eq!((b.layers[3].name.as_str(), b.page), ("Cut", a.page));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_project_with_a_newer_part_of_this_program_is_refused_and_leaves_no_tab() {
    let dir = temp_dir("newer");
    let file = dir.join("new.cre");
    let mut p = Project::new(Scene::default());
    p.set_section(section::SECTION, &serde_json::json!({ "format": 99, "page": { "w": 100, "h": 100 }, "layers": [] }));
    std::fs::write(&file, p.to_json()).unwrap();
    let mut a = app();
    a.scene.add(0, Kind::Rect { w: 1.0, h: 1.0 }, Xf::IDENTITY);
    a.touch_rev();
    a.open(file);
    assert!(a.status.starts_with("Could not open"), "{}", a.status);
    assert_eq!(a.tab_count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_svg_opens_as_a_drawing_that_asks_for_a_name_and_a_drawing_is_exported_whole() {
    let dir = temp_dir("svg");
    let file = dir.join("in.svg");
    std::fs::write(&file, SVG).unwrap();
    let mut a = app();
    a.open(file);
    assert_eq!(a.scene.shapes.len(), 2, "{}", a.status);
    assert!(a.path.is_none(), "an SVG is not saved over");
    // One selected: the export of the drawing still has both.
    a.ed.sel = vec![a.scene.shapes[0].id];
    a.layers[0].name = "Lines".into();
    let names = |l: usize| a.layer_title(l);
    let svg = base_core::export::svg(&a.scene, &[], a.page.w, a.page.h, &names, &|l| base_core::layers::color(l));
    assert_eq!(svg.matches("<path").count(), 2, "{svg}");
    assert!(svg.contains("id=\"Lines\"") && svg.contains(&format!("width=\"{}mm\"", a.page.w)), "{svg}");
    // What was written is read back as the same two outlines.
    assert_eq!(base_core::svg::parse(svg.as_bytes()).unwrap().len(), 2);
    let dxf = base_core::export::dxf(&a.scene, &[], a.page.h, &names);
    assert!(dxf.contains("ENTITIES") && dxf.contains("Lines"), "{dxf}");
    assert_eq!(base_core::dxf::parse(dxf.as_bytes(), a.page.h).unwrap().outlines.len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}

/// *Import SVG…* adds to the drawing that is open, on the layers of the colours, and can be undone; *Open…* would make a new tab.
#[test]
fn an_imported_svg_joins_the_drawing_by_colour_and_can_be_undone() {
    let dir = temp_dir("import-svg");
    let file = dir.join("in.svg");
    std::fs::write(&file, SVG).unwrap();
    let mut a = app();
    a.scene.add(5, Kind::Rect { w: 5.0, h: 5.0 }, Xf::IDENTITY);
    let tabs = a.tabs_state.parked.len();
    a.import_drawing_path(file.clone());
    assert_eq!(a.scene.shapes.len(), 3, "{}", a.status);
    assert_eq!(a.tabs_state.parked.len(), tabs, "no new tab");
    assert_eq!(a.ed.sel.len(), 2, "what came is selected");
    assert_eq!(a.scene.shapes[1].layer, base_core::layers::nearest([255, 0, 0]));
    assert_eq!(a.scene.shapes[2].layer, base_core::layers::nearest([0, 0, 0]));
    a.undo();
    assert_eq!(a.scene.shapes.len(), 1);
}

/// A file with Illustrator layers waits for the answer; the answer puts the paths on the layers.
#[test]
fn outlines_with_illustrator_layers_go_on_the_layers_of_the_program_when_asked() {
    let c = base_core::bezier::Contour::rect(10.0, 10.0);
    let o = |l: &str| base_core::svg::Outline { contours: vec![c.clone()], color: [0, 0, 0], layer: Some(l.to_string()) };
    let mut a = app();
    a.place_drawing(std::path::Path::new("x.ai"), vec![o("Cut"), o("Mark")], true);
    assert_eq!(a.scene.shapes.iter().map(|s| s.layer).collect::<Vec<_>>(), [0, 1]);
}

/// What the program writes it reads back: SVG, DXF; and UGS designs come in, as in the other programs.
#[test]
fn drawings_written_as_dxf_come_back_and_a_ugs_design_is_imported() {
    let dir = temp_dir("formats");
    let mut a = app();
    a.scene.add(0, Kind::Rect { w: 40.0, h: 20.0 }, Xf::translate(10.0, 10.0));
    let names = |l: usize| format!("L{l}");
    let dxf_file = dir.join("out.dxf");
    std::fs::write(&dxf_file, base_core::export::dxf(&a.scene, &[], a.page.h, &names)).unwrap();
    let plt = base_core::export::hpgl(&a.scene, &[], a.page.h);
    assert!(plt.contains("SP1;") && plt.contains("PD"), "{plt}");
    let mut b = app();
    b.import_file(dxf_file);
    assert_eq!(b.scene.shapes.len(), 1, "{}", b.status);
    let ugs = dir.join("in.ugsd");
    std::fs::write(&ugs, r#"{"version":"1","entities":[{"type":"RECTANGLE","cutType":"ON_PATH","cornerRadius":0,"transform":{"m00":100,"m10":0,"m01":0,"m11":20,"m02":0,"m12":0}}]}"#).unwrap();
    assert!(is_import(&ugs));
    b.import_file(ugs);
    assert_eq!(b.scene.shapes.len(), 2, "{}", b.status);
}

#[test]
fn every_tab_has_its_own_page_and_a_new_drawing_gets_the_last_one_chosen() {
    let mut a = app();
    a.new_design();
    a.set_page(Page { w: 100.0, h: 50.0 });
    assert_eq!(a.work_size_mm(), (100.0, 50.0));
    assert!(a.new_tab());
    assert_eq!(a.page, Page { w: 100.0, h: 50.0 }, "the last page chosen");
    a.set_page(Page { w: 300.0, h: 400.0 });
    a.switch_to(0);
    assert_eq!(a.page, Page { w: 100.0, h: 50.0 });
    a.switch_to(1);
    assert_eq!(a.page, Page { w: 300.0, h: 400.0 });
    // A tab that is not shown is saved with its page.
    a.switch_to(0);
    let parked = a.tabs_state.parked[1].as_ref().unwrap();
    let saved = section::load(&parked.to_project()).unwrap();
    assert_eq!(saved.page, Some(Page { w: 300.0, h: 400.0 }));
}

#[test]
fn hidden_layers_are_not_drawn_and_the_settings_come_back_from_a_backup() {
    let ctx = egui::Context::default();
    let mut a = app();
    a.layers[2].visible = false;
    assert!(!a.layer_visible(2) && a.shape_color(2).is_none() && a.shape_color(1).is_some());
    a.set_page(Page { w: 123.0, h: 456.0 });
    a.units = Units::Inch;
    let pairs: std::collections::BTreeMap<String, String> = a.settings_pairs().into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    let mut b = app();
    b.apply_settings(&ctx, &pairs);
    assert_eq!((b.new_page, b.units), (Page { w: 123.0, h: 456.0 }, Units::Inch));
}

/// The plugins are looked for under the name of this program, and a plugin for a program that is not known yet is no error.
#[test]
fn a_plugin_for_this_program_is_used_here_and_an_unknown_program_in_its_list_is_no_error() {
    assert!(base_core::plugins::PROGRAMS.contains(&"basecreator"));
    let dir = temp_dir("plugins");
    let folder = dir.join("plugins").join("star");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("plugin.json"),
        r#"{ "format": 1, "id": "star", "name": "Star", "version": "1.0.0", "programs": ["basecreator", "a-program-of-the-future"],
             "generators": [ { "id": "s", "name": "S", "script": "s.rhai" } ] }"#,
    )
    .unwrap();
    std::fs::write(folder.join("s.rhai"), "#{ shapes: [] }").unwrap();
    let ui = base_ui::plugins_ui::PluginsUi::new("basecreator", &dir);
    assert!(ui.registry.broken.is_empty(), "{:?}", ui.registry.broken.iter().map(|b| &b.reason).collect::<Vec<_>>());
    assert_eq!(ui.registry.generators("basecreator").len(), 1);
    assert!(ui.registry.generators("millcreator").is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

fn labels(entries: &[Entry<Act>], out: &mut Vec<&'static str>) {
    for e in entries {
        match e {
            Entry::Item(_, l, _) => out.push(l),
            Entry::Sub(t, inner) => {
                out.push(t);
                labels(inner, out);
            }
            _ => {}
        }
    }
}

/// Every label of the menus and every text of the code has its translation in every language: in the texts of this program or the shared ones.
#[test]
fn every_text_of_the_program_is_translated_in_all_languages() {
    let _ = app();
    let mut wanted: Vec<String> = vec![];
    let mut menu_labels = vec![];
    for (title, entries) in menus() {
        menu_labels.push(title);
        labels(&entries, &mut menu_labels);
    }
    // The names of the languages and of the colour schemes are not translated.
    let names: Vec<String> = Lang::ALL.iter().map(|l| l.name().to_string()).chain(Scheme::available().iter().map(|s| s.label().to_string())).collect();
    wanted.extend(menu_labels.iter().map(|s| s.to_string()).filter(|l| !names.contains(l)));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.ends_with("texts.rs") || path.ends_with("tests.rs") {
            continue;
        }
        let code = std::fs::read_to_string(&path).unwrap();
        for call in ["tr(\"", "trf(\""] {
            for (at, _) in code.match_indices(call) {
                if code[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let part = &code[at + call.len()..];
                let text = &part[..part.find('"').unwrap()];
                if text != "{}" && !text.is_empty() {
                    wanted.push(text.to_string());
                }
            }
        }
    }
    for u in Units::ALL {
        wanted.push(u.label().to_string());
    }
    wanted.sort();
    wanted.dedup();
    let mut missing = vec![];
    for t in wanted {
        let key: &'static str = Box::leak(t.clone().into_boxed_str());
        for l in Lang::ALL.iter().filter(|l| **l != Lang::En) {
            let found = texts::TEXTS.lookup(l.index() as usize, key).or_else(|| base_ui::texts::TEXTS.lookup(l.index() as usize, key));
            if found.is_none_or(|s| s.is_empty()) {
                missing.push(format!("{}: {t}", l.code()));
            }
        }
    }
    assert!(missing.is_empty(), "texts without a translation: {missing:#?}");
}

/// The windows and the panels are drawn without a window system, on the start screen and in the editor, also with a split view.
#[test]
fn the_program_draws_its_screens_and_windows() {
    let ctx = egui::Context::default();
    let mut a = App::build(&ctx, None, false);
    let frame = |a: &mut App| {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..Default::default() }, |ui| a.draw(ui));
        out.textures_delta.clear();
    };
    frame(&mut a);
    assert_eq!(a.screen, Screen::Start);
    a.new_tab();
    a.scene.add(0, Kind::Rect { w: 30.0, h: 30.0 }, Xf::translate(10.0, 10.0));
    a.show_page = true;
    a.show_about = true;
    a.show_program_options = true;
    for t in [SideTab::Properties, SideTab::Layers, SideTab::Clipart, SideTab::Description] {
        a.side_tab = t;
        frame(&mut a);
    }
    assert_eq!(a.screen, Screen::Editor);
}

/// Every festive icon can be read at its size, and the tests (so also the screenshots) get the normal one.
#[test]
fn every_festive_icon_decodes_at_its_size() {
    let v = &crate::FESTIVE;
    for (name, (small, big)) in [("christmas", v.christmas), ("new year", v.new_years_eve), ("halloween", v.halloween), ("easter", v.easter)] {
        let small = base_ui::start::window_icon(small).unwrap_or_else(|| panic!("{name} 256 does not decode"));
        let big = base_ui::start::window_icon(big).unwrap_or_else(|| panic!("{name} 512 does not decode"));
        assert_eq!((small.width, small.height), (256, 256), "{name}");
        assert_eq!((big.width, big.height), (512, 512), "{name}");
    }
    assert_eq!(crate::app_icon(), (crate::ICON_256, crate::ICON_512));
}

/// The window of the plugins of this program refuses what changes G-code.
#[test]
fn the_plugins_of_this_program_refuse_gcode() {
    let dir = temp_dir("no-gcode");
    let mut ui = base_ui::plugins_ui::PluginsUi::new("basecreator", &dir);
    assert!(ui.registry.refuse_gcode);
    ui.reload();
    assert!(ui.registry.refuse_gcode, "also after loading the plugins again");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The text tool puts a text on the page, the Properties tab opens to type in, and one undo takes it back.
#[test]
fn the_text_tool_places_a_text_to_type_in() {
    let ctx = egui::Context::default();
    base_ui::fonts::install(&ctx);
    let mut a = app();
    a.new_tab();
    a.side_tab = crate::SideTab::Layers;
    a.place_text(base_core::geom::Pt { x: 20.0, y: 30.0 });
    assert_eq!(a.side_tab, crate::SideTab::Properties);
    assert!(a.ed.focus_text);
    assert_eq!(a.ed.sel.len(), 1);
    let id = a.ed.sel[0];
    assert!(matches!(a.scene.shape(id).unwrap().kind, base_core::scene::Kind::Text(_)));
    assert!(!a.scene.shape(id).unwrap().polys().is_empty(), "it has outlines to draw");
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            base_ui::editor::properties_sections(&mut a, ui);
        });
    });
    out.textures_delta.clear();
    assert!(!a.ed.focus_text, "the field has taken the keyboard");
    a.undo();
    assert!(a.scene.shape(id).is_none());
}

/// A real click with the text tool on the page of the window puts the text there.
#[test]
fn a_click_with_the_text_tool_on_the_work_area_makes_a_text() {
    let mut h = egui_kittest::Harness::builder().with_accessibility_check(false).with_size([1280.0, 800.0]).build_ui_state(
        |ui, state: &mut Option<App>| {
            let a = state.get_or_insert_with(|| App::build(ui.ctx(), None, false));
            a.draw(ui);
        },
        None,
    );
    h.run_steps(2);
    {
        let a = h.state_mut().as_mut().unwrap();
        a.new_tab();
        a.ed.tool = base_ui::editor::Tool::Text;
    }
    h.run_steps(4);
    let at = egui::pos2(440.0, 400.0);
    h.event(egui::Event::PointerMoved(at));
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
        h.run_steps(2);
    }
    h.run_steps(4);
    let a = h.state_mut().as_mut().unwrap();
    assert_eq!(a.scene.shapes.len(), 1, "the click made one shape");
    assert!(matches!(a.scene.shapes[0].kind, Kind::Text(_)));
    assert_eq!(a.side_tab, crate::SideTab::Properties);
    if let Ok(dir) = std::env::var("BC_SHOT_DIR") {
        h.render().unwrap().save(format!("{dir}/text.png")).unwrap();
    }
}

/// The window of the program for the check of the tools.
struct Window(egui_kittest::Harness<'static, Option<App>>);

impl base_ui::tool_check::Drive for Window {
    type App = App;
    fn app(&mut self) -> &mut App {
        self.0.state_mut().as_mut().unwrap()
    }
    fn send(&mut self, e: egui::Event) {
        self.0.event(e);
    }
    fn frames(&mut self) {
        self.0.run_steps(3);
    }
    fn shows(&mut self, text: &str) -> bool {
        egui_kittest::kittest::Queryable::query_all_by_label_contains(&self.0, text).next().is_some()
    }
    fn ctx(&mut self) -> egui::Context {
        self.0.ctx.clone()
    }
}

/// Every tool through the real window: the same check runs in every program.
#[test]
fn every_tool_works_in_the_window() {
    let mut h = egui_kittest::Harness::builder().with_accessibility_check(false).with_size([1280.0, 800.0]).build_ui_state(
        |ui, state: &mut Option<App>| {
            let a = state.get_or_insert_with(|| App::build(ui.ctx(), None, false));
            a.draw(ui);
        },
        None,
    );
    h.run_steps(2);
    h.state_mut().as_mut().unwrap().new_tab();
    h.run_steps(4);
    let mut w = Window(h);
    base_ui::tool_check::check_tools(&mut w);
    {
        use crate::menu::Act;
        base_ui::tool_check::check_windows(&mut w, &[(Act::Print, "Print"), (Act::QuickGuide, "Quick guide")]);
    }
}

#[test]
fn every_text_of_the_menus_has_a_translation() {
    let _ = app();
    base_ui::tool_check::check_menu_texts(&crate::menu::menus());
}

/// The guide sends nobody to a place in the menus that is not there.
#[test]
fn the_quick_guide_names_only_what_the_menus_have() {
    base_ui::tool_check::check_guide_paths(crate::guide::EN, &crate::menu::menus());
}

/// The picture under the drawing goes into the project and comes back from it, with its corners, and in a new project it is gone.
#[test]
fn the_overlay_is_kept_in_the_project_file() {
    use base_ui::editor::EditorBackground;
    let ctx = egui::Context::default();
    let mut a = app();
    a.set_background(&ctx, egui::ColorImage::from_rgb([8, 4], &[120u8; 8 * 4 * 3]), "photo.jpg");
    a.ed.background.as_mut().unwrap().corners[2] = base_core::geom::Pt::new(55.0, 66.0);
    a.ed.background.as_mut().unwrap().visible = false;
    let text = a.to_project().to_json();
    assert!(text.contains("\"overlay\""));
    let p = base_core::project::Project::from_json(&text).unwrap();
    a.background_from_project(&base_core::project::Project::default());
    a.background_tick(&ctx);
    assert!(a.ed.background.is_none(), "a project without one has none");
    assert!(!a.to_project().to_json().contains("\"overlay\""));
    a.kept = p.clone();
    a.background_from_project(&p);
    assert!(a.to_project().to_json().contains("\"overlay\""), "not lost while it waits to be shown");
    a.background_tick(&ctx);
    let b = a.ed.background.as_ref().expect("it came back");
    assert_eq!((b.size, b.corners[2].x, b.visible, b.name.as_str()), ((8, 4), 55.0, false, "photo.jpg"));
}
