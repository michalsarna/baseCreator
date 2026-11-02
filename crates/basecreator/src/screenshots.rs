//! Renders the screenshots headlessly (no window, no display needed).
//!
//! `cargo test -p basecreator render_screenshots -- --ignored --nocapture` writes PNG files into `docs/basecreator/`.
use crate::section::Page;
use crate::{App, SideTab};
use egui_kittest::Harness;
use std::path::PathBuf;

type Shot = Harness<'static, Option<App>>;

fn save(h: &mut Shot, name: &str) {
    h.run_steps(5);
    let img = h.render().expect("render");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/basecreator");
    std::fs::create_dir_all(&dir).expect("create screenshot dir");
    let p = dir.join(name);
    img.save(&p).expect("write png");
    println!("wrote {}", p.display());
}

fn app(h: &mut Shot) -> &mut App {
    h.state_mut().as_mut().expect("app built")
}

#[test]
#[ignore = "writes docs/basecreator; run explicitly"]
fn render_screenshots() {
    use base_core::bezier::Contour;
    use base_core::geom::Xf;
    use base_core::scene::Kind;
    let mut h: Shot = Harness::builder().with_accessibility_check(false).with_size([1280.0, 800.0]).build_ui_state(
        |ui, state: &mut Option<App>| {
            let a = state.get_or_insert_with(|| App::build(ui.ctx(), None, false));
            a.draw(ui);
        },
        None,
    );
    h.run_steps(2);
    save(&mut h, "start.png");
    {
        let a = app(&mut h);
        a.new_tab();
        a.set_page(Page { w: 297.0, h: 210.0 });
        a.scene.add(0, Kind::Rect { w: 150.0, h: 100.0 }, Xf::translate(70.0, 55.0));
        a.scene.add(2, Kind::Ellipse { w: 46.0, h: 46.0 }, Xf::translate(85.0, 70.0));
        a.scene.add(1, Kind::Bezier(vec![Contour::star(5, 0.45, 40.0, 40.0)]), Xf::translate(145.0, 68.0));
        a.layers[0].name = "Frame".into();
        a.layers[1].name = "Star".into();
        a.layers[2].name = "Circle".into();
        a.touch_rev();
        a.side_tab = SideTab::Layers;
    }
    save(&mut h, "editor.png");
    app(&mut h).show_page = true;
    save(&mut h, "page-setup.png");
    app(&mut h).show_page = false;
}
