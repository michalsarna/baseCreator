//! Native macOS menu bar built with `muda`. On Windows and Linux the in-window egui menu bar is used instead, so no
//! GTK dependency is needed. The application describes its menus with `menu::Entry` and gives the few things that
//! differ between applications in a `NativeConfig`.
use crate::i18n::{lang, tr, Lang};
use crate::menu::{Entry, Menus};
use muda::{accelerator::Accelerator, CheckMenuItem, IconMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use std::path::PathBuf;
use std::sync::Mutex;

static QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// What differs between applications.
pub struct NativeConfig<A: 'static> {
    /// The name of the application menu, left of the first menu (the one that holds About and Quit).
    pub app_name: &'static str,
    /// English text of the About item (translated).
    pub about_label: &'static str,
    pub about: A,
    pub quit: A,
    pub menus: fn() -> Menus<A>,
    pub act_icon: fn(A) -> Option<&'static str>,
    /// Entries that are switches.
    pub is_check: fn(A) -> bool,
    /// Accelerators the system menu owns. Editing shortcuts (copy, paste, delete...) stay with egui so text fields keep
    /// working; the application's shortcut handler skips the ones listed here.
    pub owns_shortcut: fn(A) -> bool,
    /// The action that opens recent file `i`, and the one that clears the list.
    pub open_recent: fn(u8) -> A,
    pub clear_recent: A,
    pub max_recent: u8,
}

enum Handle {
    Plain(MenuItem),
    Icon(IconMenuItem),
    Check(CheckMenuItem),
}

pub struct NativeMenu<A: 'static> {
    cfg: &'static NativeConfig<A>,
    _menu: Menu,
    items: Vec<(A, Handle)>,
    built_for: Lang,
    /// The "Open recent" submenu, its current children and the files they stand for.
    recent_sub: Option<Submenu>,
    recent_items: Vec<Box<dyn muda::IsMenuItem>>,
    recent_shown: Option<Vec<PathBuf>>,
}

fn id<A: std::fmt::Debug>(a: A) -> String {
    format!("{a:?}")
}

/// Is the macOS menu bar dark right now? Icons are drawn light on dark and dark on light.
fn menu_is_dark() -> bool {
    std::process::Command::new("defaults").args(["read", "-g", "AppleInterfaceStyle"]).output().map(|o| String::from_utf8_lossy(&o.stdout).contains("Dark")).unwrap_or(false)
}

/// An SVG icon from the assets as a menu icon (36 px, shown at 18 points on Retina screens).
fn rasterize(name: &str) -> Option<muda::Icon> {
    use std::sync::OnceLock;
    static DARK: OnceLock<bool> = OnceLock::new();
    let ink = if *DARK.get_or_init(menu_is_dark) { "#f0f0f0" } else { "#303030" };
    let text = crate::icons::svg_text(name)?.replace("currentColor", ink);
    let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default()).ok()?;
    let px = 36u32;
    let mut pm = resvg::tiny_skia::Pixmap::new(px, px)?;
    let k = px as f32 / tree.size().width();
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pm.as_mut());
    // tiny-skia stores premultiplied colours; the menu wants straight alpha.
    let mut rgba = Vec::with_capacity((px * px * 4) as usize);
    for p in pm.pixels() {
        let c = p.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    muda::Icon::from_rgba(rgba, px, px).ok()
}

fn append<A: Copy + PartialEq + std::fmt::Debug + 'static>(cfg: &NativeConfig<A>, sub: &Submenu, entries: &[Entry<A>], items: &mut Vec<(A, Handle)>, recent_sub: &mut Option<Submenu>) {
    for e in entries {
        match e {
            Entry::Sep => {
                let _ = sub.append(&PredefinedMenuItem::separator());
            }
            Entry::Sub(title, children) => {
                let s = Submenu::new(tr(title), true);
                append(cfg, &s, children, items, recent_sub);
                let _ = sub.append(&s);
            }
            Entry::Recent => {
                let s = Submenu::new(tr("Open recent"), true);
                let _ = sub.append(&s);
                *recent_sub = Some(s);
            }
            Entry::Item(act, label, accel) => {
                let accel: Option<Accelerator> = if (cfg.owns_shortcut)(*act) { accel.and_then(|(a, _)| a.parse().ok()) } else { None };
                if (cfg.is_check)(*act) {
                    let it = CheckMenuItem::with_id(id(*act), tr(label), true, false, accel);
                    let _ = sub.append(&it);
                    items.push((*act, Handle::Check(it)));
                } else if let Some(icon) = (cfg.act_icon)(*act).and_then(rasterize) {
                    let it = IconMenuItem::with_id(id(*act), tr(label), true, Some(icon), accel);
                    let _ = sub.append(&it);
                    items.push((*act, Handle::Icon(it)));
                } else {
                    let it = MenuItem::with_id(id(*act), tr(label), true, accel);
                    let _ = sub.append(&it);
                    items.push((*act, Handle::Plain(it)));
                }
            }
        }
    }
}

impl<A: Copy + PartialEq + std::fmt::Debug + 'static> NativeMenu<A> {
    pub fn install(ctx: &egui::Context, cfg: &'static NativeConfig<A>) -> NativeMenu<A> {
        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if let Ok(mut q) = QUEUE.lock() {
                q.push(e.id.0.clone());
            }
            ctx.request_repaint();
        }));
        Self::build(cfg)
    }

    fn build(cfg: &'static NativeConfig<A>) -> NativeMenu<A> {
        let menu = Menu::new();
        let mut items = Vec::new();
        // Application menu (the one named after the app, left of File).
        let app = Submenu::new(cfg.app_name, true);
        let about = MenuItem::with_id(id(cfg.about), tr(cfg.about_label), true, None);
        let _ = app.append(&about);
        items.push((cfg.about, Handle::Plain(about)));
        let _ = app.append(&PredefinedMenuItem::separator());
        let _ = app.append(&PredefinedMenuItem::services(None));
        let _ = app.append(&PredefinedMenuItem::separator());
        let _ = app.append(&PredefinedMenuItem::hide(None));
        let _ = app.append(&PredefinedMenuItem::hide_others(None));
        let _ = app.append(&PredefinedMenuItem::show_all(None));
        let _ = app.append(&PredefinedMenuItem::separator());
        // Our own Quit item, so closing asks about unsaved work first.
        let quit = MenuItem::with_id(id(cfg.quit), tr("Quit"), true, "CmdOrCtrl+Q".parse::<Accelerator>().ok());
        let _ = app.append(&quit);
        items.push((cfg.quit, Handle::Plain(quit)));
        let _ = menu.append(&app);
        let mut recent_sub = None;
        for (title, entries) in (cfg.menus)() {
            let sub = Submenu::new(tr(title), true);
            append(cfg, &sub, &entries, &mut items, &mut recent_sub);
            let _ = menu.append(&sub);
        }
        let window = Submenu::new("Window", true);
        let _ = window.append(&PredefinedMenuItem::minimize(None));
        let _ = window.append(&PredefinedMenuItem::maximize(None));
        let _ = menu.append(&window);
        menu.init_for_nsapp();
        window.set_as_windows_menu_for_nsapp();
        NativeMenu { cfg, _menu: menu, items, built_for: lang(), recent_sub, recent_items: vec![], recent_shown: None }
    }

    /// Replace the children of "Open recent" with the current list of files.
    fn fill_recent(&mut self, recent: &[PathBuf]) {
        let Some(sub) = &self.recent_sub else { return };
        for it in self.recent_items.drain(..) {
            let _ = sub.remove(it.as_ref());
        }
        let add = |it: Box<dyn muda::IsMenuItem>, items: &mut Vec<Box<dyn muda::IsMenuItem>>| {
            let _ = sub.append(it.as_ref());
            items.push(it);
        };
        let mut kept = vec![];
        if recent.is_empty() {
            add(Box::new(MenuItem::new(tr("(empty)"), false, None)), &mut kept);
        }
        for (i, p) in recent.iter().enumerate() {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string());
            add(Box::new(MenuItem::with_id(id((self.cfg.open_recent)(i as u8)), name, true, None)), &mut kept);
        }
        if !recent.is_empty() {
            add(Box::new(PredefinedMenuItem::separator()), &mut kept);
            add(Box::new(MenuItem::with_id(id(self.cfg.clear_recent), tr("Clear list"), true, None)), &mut kept);
        }
        self.recent_items = kept;
        self.recent_shown = Some(recent.to_vec());
    }

    /// Actions the user picked in the menu since the last call.
    pub fn take_actions(&self) -> Vec<A> {
        let ids: Vec<String> = QUEUE.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        ids.iter()
            .filter_map(|i| {
                self.items
                    .iter()
                    .find(|(a, _)| &id(*a) == i)
                    .map(|(a, _)| *a)
                    .or_else(|| (0..self.cfg.max_recent).map(self.cfg.open_recent).chain([self.cfg.clear_recent]).find(|a| &id(*a) == i))
            })
            .collect()
    }

    /// Rebuild after a language change and mirror check marks / enabled state from the app.
    pub fn sync(&mut self, checked: impl Fn(A) -> Option<bool>, enabled: impl Fn(A) -> bool, recent: &[PathBuf]) {
        if self.built_for != lang() {
            *self = Self::build(self.cfg);
        }
        if self.recent_shown.as_deref() != Some(recent) {
            self.fill_recent(recent);
        }
        for (act, h) in &self.items {
            match h {
                Handle::Check(c) => {
                    if let Some(v) = checked(*act) {
                        if c.is_checked() != v {
                            c.set_checked(v);
                        }
                    }
                }
                Handle::Plain(p) => {
                    let en = enabled(*act);
                    if p.is_enabled() != en {
                        p.set_enabled(en);
                    }
                }
                Handle::Icon(p) => {
                    let en = enabled(*act);
                    if p.is_enabled() != en {
                        p.set_enabled(en);
                    }
                }
            }
        }
    }
}

/// Show the application icon in the Dock (running outside an .app bundle gives no icon otherwise).
pub fn set_dock_icon(png: &[u8]) {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;
    let Some(mtm) = MainThreadMarker::new() else { return };
    let data = NSData::with_bytes(png);
    if let Some(img) = NSImage::initWithData(NSImage::alloc(), &data) {
        // SAFETY: called on the main thread with a valid NSImage.
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&img)) };
    }
}
