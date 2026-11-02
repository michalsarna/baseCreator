//! Prototype: does egui on an Android tablet get the soft keyboard, Polish letters, pinch and two-finger pan?
use eframe::egui;
use winit::platform::android::activity::{input::TextInputState, AndroidApp};

struct Proto {
    app: AndroidApp,
    text: String,
    note: String,
    zoom: f32,
    pan: egui::Vec2,
    ime_last: String,
    kb_shown: bool,
    log: Vec<String>,
}

impl Proto {
    /// winit 0.30 drops GameActivity's text events, so the text of the input method is read here every frame and the
    /// difference is given to egui as typed text and Backspaces.
    fn bridge_ime(&mut self, raw: &mut egui::RawInput) {
        // Before the first `set_text_input_state` GameTextInput has no buffer at all (a null pointer that
        // android-activity 0.6.1 turns into a slice), so it is read only while the keyboard is up.
        if !self.kb_shown {
            return;
        }
        let st = self.app.text_input_state();
        if st.text != self.ime_last {
            let old: Vec<char> = self.ime_last.chars().collect();
            let new: Vec<char> = st.text.chars().collect();
            let common = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
            for _ in common..old.len() {
                raw.events.push(egui::Event::Key { key: egui::Key::Backspace, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() });
            }
            let added: String = new[common..].iter().collect();
            if !added.is_empty() {
                raw.events.push(egui::Event::Text(added.clone()));
            }
            self.log.push(format!("ime: {:?} -> {:?}", self.ime_last, st.text));
            self.ime_last = st.text;
        }
    }
}

impl eframe::App for Proto {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.bridge_ime(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = &ui.ctx().clone();
        ctx.set_pixels_per_point(2.0);
        egui::Panel::top("top").show(ui, |ui| {
            ui.heading("BaseCreator prototype");
            ui.horizontal(|ui| {
                ui.label("Text:");
                ui.text_edit_singleline(&mut self.text);
            });
            ui.horizontal(|ui| {
                ui.label("Note:");
                ui.text_edit_multiline(&mut self.note);
            });
            ui.label(format!("zoom {:.2}  pan {:.0},{:.0}  keyboard {}", self.zoom, self.pan.x, self.pan.y, self.kb_shown));
            for l in self.log.iter().rev().take(4) {
                ui.small(l);
            }
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
            let (zd, td, touches) = ctx.input(|i| (i.zoom_delta(), i.translation_delta(), i.multi_touch()));
            if resp.hovered() || touches.is_some() {
                self.zoom = (self.zoom * zd).clamp(0.1, 50.0);
                if touches.is_some() {
                    self.pan += td;
                }
            }
            if resp.dragged() && touches.is_none() {
                self.pan += resp.drag_delta();
            }
            let o = resp.rect.center() + self.pan;
            let r = egui::Rect::from_center_size(o, egui::vec2(100.0, 60.0) * self.zoom);
            painter.rect_stroke(r, 0.0, egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE), egui::StrokeKind::Middle);
            painter.text(o, egui::Align2::CENTER_CENTER, &self.text, egui::FontId::proportional(14.0 * self.zoom), ui.visuals().text_color());
        });
        let wants = ctx.egui_wants_keyboard_input();
        if wants != self.kb_shown {
            if wants { self.app.show_soft_input(true) } else { self.app.hide_soft_input(true) }
            self.kb_shown = wants;
            if wants {
                // Start the input method from the text of nothing; the bridge sends only what changes.
                self.app.set_text_input_state(TextInputState { text: String::new(), selection: winit::platform::android::activity::input::TextSpan { start: 0, end: 0 }, compose_region: None });
                self.ime_last.clear();
            }
        }
        if self.kb_shown {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info));
    let opts = eframe::NativeOptions { android_app: Some(app.clone()), ..Default::default() };
    let _ = eframe::run_native(
        "BaseCreator",
        opts,
        Box::new(move |_cc| {
            Ok(Box::new(Proto { app, text: "ABC".into(), note: String::new(), zoom: 1.0, pan: egui::Vec2::ZERO, ime_last: String::new(), kb_shown: false, log: vec![] }))
        }),
    );
}
