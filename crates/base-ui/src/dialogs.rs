//! Ready-made dialogs: the About window, the quick guide and the check for a newer release.
use crate::i18n::{tr, trf};
use crate::window::popup;
use crate::theme;
use egui::{Color32, RichText};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

/// What the About window says.
pub struct AboutInfo<'a> {
    /// The title of the window, translated ("About LightCreator").
    pub title: &'a str,
    pub name: &'a str,
    pub version: &'a str,
    /// Lines under the name, already translated (what the program is, who wrote it, ...).
    pub lines: Vec<String>,
    pub url: &'a str,
    /// The home page of the programs (the landing page).
    pub site: &'a str,
    pub licence: &'a str,
    /// The text of the program's CREDITS.md: its bullets are listed in the window (the projects the program is made with).
    pub credits: &'a str,
}

/// One entry of the credits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credit {
    pub name: String,
    pub url: Option<String>,
    /// What it is for and its licence, as plain text.
    pub note: String,
}

/// The bullets of a CREDITS.md as a list: the name (a link when it has one) and what follows it, without the marks of markdown.
pub fn parse_credits(md: &str) -> Vec<Credit> {
    let mut entries: Vec<String> = vec![];
    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("* ").or_else(|| line.strip_prefix("- ")) {
            entries.push(rest.trim().to_string());
        } else if line.starts_with("  ") && !line.trim().is_empty() {
            if let Some(last) = entries.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
            }
        }
    }
    entries.into_iter().filter_map(|e| credit_of(&e)).collect()
}

fn credit_of(entry: &str) -> Option<Credit> {
    // The name: the first **bold** part, which may hold a link: **[name](url)** or **name**.
    let start = entry.find("**")? + 2;
    let end = start + entry[start..].find("**")?;
    let bold = &entry[start..end];
    // A link inside the name keeps its text: **[name](url)** or **Name ([site](url))**.
    let (mut name, mut url) = (String::new(), None);
    let mut chars = bold.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut text = String::new();
            for t in chars.by_ref() {
                if t == ']' {
                    break;
                }
                text.push(t);
            }
            if chars.peek() == Some(&'(') {
                chars.next();
                let mut u = String::new();
                for t in chars.by_ref() {
                    if t == ')' {
                        break;
                    }
                    u.push(t);
                }
                url.get_or_insert(u);
            }
            name.push_str(&text);
        } else {
            name.push(c);
        }
    }
    let rest = &entry[end + 2..];
    // Links in the rest keep their text; bold and code marks go.
    let mut note = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '[' => {
                let mut text = String::new();
                for t in chars.by_ref() {
                    if t == ']' {
                        break;
                    }
                    text.push(t);
                }
                if chars.peek() == Some(&'(') {
                    for t in chars.by_ref() {
                        if t == ')' {
                            break;
                        }
                    }
                }
                note.push_str(&text);
            }
            '*' | '`' => {}
            _ => note.push(c),
        }
    }
    let note = note.trim().trim_start_matches([':', ' ']).to_string();
    (!name.trim().is_empty()).then(|| Credit { name: name.trim().to_string(), url, note })
}

/// The version of baseCreator that the program is built with (the shared code: core, interface, plugins).
pub fn base_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The channel of Jakub (CNC KpK), who works on industrial CNC machines and suggested many improvements; the About window links to it.
pub const THANKS_URL: &str = "https://www.youtube.com/@CNC-KpK";

/// The About window. `logo` is the picture shown next to the name.
pub fn about_window(ctx: &egui::Context, open: &mut bool, info: &AboutInfo, logo: Option<&egui::TextureHandle>) {
    if !*open {
        return;
    }
    popup(ctx, "about_window", info.title, [440.0, 470.0], open, |ui| {
        ui.horizontal(|ui| {
            if let Some(logo) = logo {
                ui.image((logo.id(), egui::vec2(72.0, 72.0)));
            }
            ui.vertical(|ui| {
                ui.heading(info.name);
                ui.label(trf("Version {}", &[&info.version]));
                ui.label(RichText::new(format!("baseCreator {}", base_version())).color(theme::text_dim()));
            });
        });
        for (i, line) in info.lines.iter().enumerate() {
            ui.label(line);
            if i == 0 {
                ui.separator();
            }
        }
        ui.horizontal(|ui| {
            ui.hyperlink_to(tr("Home page"), info.site);
            ui.label("·");
            ui.hyperlink_to(tr("Project page on GitHub"), info.url);
        });
        ui.label(format!("{}: {}", tr("Licence"), info.licence));
        ui.separator();
        ui.label(tr("Big thanks to Jakub (CNC KpK) for the ideas and suggestions for improvements:"));
        ui.hyperlink(THANKS_URL);
        ui.add_space(4.0);
        credits_list(ui, &parse_credits(info.credits));
    });
}

/// The credits in a list with the arrows always in view (the wheel works too). The list takes the whole width of the window.
fn credits_list(ui: &mut egui::Ui, credits: &[Credit]) {
    const H: f32 = 150.0;
    const STEP: f32 = 60.0;
    const ARROWS: f32 = 28.0;
    ui.separator();
    ui.label(RichText::new(tr("Made with:")).strong());
    let id = ui.id().with("about_credits_offset");
    let mut offset: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    // The list is drawn in a child that the window does not count into its size: a width that the content asks for would keep the window from shrinking.
    let full = ui.available_width().max(60.0);
    let (outer, _) = ui.allocate_exact_size(egui::vec2(60.0, H), egui::Sense::hover());
    let list = egui::Rect::from_min_size(outer.min, egui::vec2(full - ARROWS, H));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(list));
    let out = egui::ScrollArea::vertical().id_salt("about_credits").max_height(H).min_scrolled_height(H).max_width(list.width()).auto_shrink([false, false]).vertical_scroll_offset(offset).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(&mut child, |ui| {
        ui.set_max_width(list.width() - 4.0);
        for c in credits {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                match &c.url {
                    Some(u) => ui.hyperlink_to(&c.name, u),
                    None => ui.label(RichText::new(&c.name).strong()),
                };
                if !c.note.is_empty() {
                    ui.label(RichText::new(&c.note).small().color(theme::text_dim()));
                }
            });
        }
    });
    let max = (out.content_size.y - out.inner_rect.height()).max(0.0);
    offset = out.state.clamped_offset().y.clamp(0.0, max);
    let arrows = egui::Rect::from_min_size(egui::pos2(list.max.x + 4.0, list.min.y), egui::vec2(24.0, 52.0));
    let mut col = ui.new_child(egui::UiBuilder::new().max_rect(arrows));
    for (icon, up) in [("chevron-up", true), ("chevron-down", false)] {
        let enabled = if up { offset > 0.5 } else { offset < max - 0.5 };
        let (rect, resp) = col.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
        if resp.hovered() && enabled {
            col.painter().rect_filled(rect, 4.0, theme::panel_dark());
        }
        crate::icons::paint(&col, rect.shrink(5.0), icon, if enabled { theme::text() } else { theme::text_dim().gamma_multiply(0.5) });
        if enabled && resp.clicked() {
            offset = if up { (offset - STEP).max(0.0) } else { (offset + STEP).min(max) };
        }
    }
    ui.data_mut(|d| d.insert_temp(id, offset));
}

/// A part of the quick guide.
pub struct Section {
    pub title: &'static str,
    pub lines: &'static [&'static str],
}

/// The quick-guide window: a search field, the project link and the sections (collapsible, filtered by the search).
/// Sections whose title is in `open_first` start open. `footer` is the note under the sections.
pub fn guide_window(ctx: &egui::Context, show: &mut bool, filter: &mut String, sections: &[Section], open_first: &[&str], url: &str, footer: &str) {
    if !*show {
        return;
    }
    let mut open = true;
    popup(ctx, "w_help_ui_9", &tr("Quick guide"), [340.0, 220.0], &mut open, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(filter).hint_text(tr("Search")).desired_width(220.0));
            ui.hyperlink_to(tr("Project page on GitHub"), url);
        });
        if sections.iter().all(|s| s.title.is_empty()) {
            return;
        }
        ui.add_space(4.0);
        let q = filter.to_lowercase();
        egui::ScrollArea::vertical().max_height(crate::window::fill_size(ui).y).auto_shrink([false, false]).show(ui, |ui| {
            for s in sections {
                let hits: Vec<&&str> = s.lines.iter().filter(|l| q.is_empty() || l.to_lowercase().contains(&q) || s.title.to_lowercase().contains(&q)).collect();
                if hits.is_empty() {
                    continue;
                }
                egui::CollapsingHeader::new(RichText::new(s.title).strong().size(15.0))
                    .id_salt(s.title)
                    .default_open(!q.is_empty() || open_first.contains(&s.title))
                    .open(if q.is_empty() { None } else { Some(true) })
                    .show(ui, |ui| {
                        for l in hits {
                            ui.label(*l);
                            ui.add_space(3.0);
                        }
                    });
            }
            ui.add_space(6.0);
            ui.label(RichText::new(footer).color(theme::text_dim()));
        });
    });
    *show = open;
}

/// Which program looks for which releases.
pub struct UpdateConfig {
    pub app_name: &'static str,
    /// The GitHub repository, `owner/name`.
    pub repo: &'static str,
    /// The version that runs, such as `v0.1.5`.
    pub version: &'static str,
}

impl UpdateConfig {
    /// Page with the packages of the newest release.
    pub fn releases_url(&self) -> String {
        format!("https://github.com/{}/releases/latest", self.repo)
    }
    fn api_url(&self) -> String {
        format!("https://api.github.com/repos/{}/releases/latest", self.repo)
    }
}

pub enum Outcome {
    Newer(String),
    UpToDate(String),
    Failed(String),
}

#[derive(Default)]
pub struct UpdateState {
    pub(crate) rx: Option<Receiver<Result<String, String>>>,
    /// The check was asked for by hand: tell the result whatever it is.
    pub(crate) manual: bool,
    pub window: Option<Outcome>,
}

/// `v0.0.8` or `0.0.8` as numbers.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let mut it = s.trim().trim_start_matches(['v', 'V']).split('.');
    let mut next = || it.next().and_then(|p| p.split(['-', '+']).next()).and_then(|p| p.parse::<u64>().ok());
    Some((next()?, next().unwrap_or(0), next().unwrap_or(0)))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// The `tag_name` of the answer of the GitHub releases API.
pub fn latest_tag_from_json(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("tag_name")?.as_str().map(str::to_string)
}

fn fetch_latest(cfg: &UpdateConfig) -> Result<String, String> {
    let mut resp = ureq::get(&cfg.api_url())
        .header("User-Agent", cfg.app_name)
        .header("Accept", "application/vnd.github+json")
        .config()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .call()
        .map_err(|e| e.to_string())?;
    let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    latest_tag_from_json(&body).ok_or_else(|| "no release found".to_string())
}

impl UpdateState {
    /// Ask GitHub for the newest release in the background. `manual` shows the answer even when there is nothing new.
    pub fn start(&mut self, ctx: &egui::Context, cfg: &'static UpdateConfig, manual: bool) {
        if self.rx.is_some() {
            self.manual |= manual;
            return;
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(fetch_latest(cfg));
            ctx.request_repaint();
        });
        self.rx = Some(rx);
        self.manual = manual;
    }

    /// Pick up the answer and draw the window about it.
    pub fn tick(&mut self, ctx: &egui::Context, cfg: &UpdateConfig) {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(res) => {
                    let manual = self.manual;
                    self.rx = None;
                    self.manual = false;
                    self.window = match res {
                        Ok(tag) if is_newer(&tag, cfg.version) => Some(Outcome::Newer(tag)),
                        Ok(tag) if manual => Some(Outcome::UpToDate(tag)),
                        Err(e) if manual => Some(Outcome::Failed(e)),
                        _ => None,
                    };
                }
                Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(300)),
                Err(TryRecvError::Disconnected) => self.rx = None,
            }
        }
        let Some(outcome) = &self.window else { return };
        let mut open = true;
        let mut go = false;
        let mut shut = false;
        popup(ctx, "update_window", &tr("Updates"), [360.0, 200.0], &mut open, |ui| match outcome {
            Outcome::Newer(tag) => {
                ui.label(RichText::new(trf("A newer version is available: {}", &[tag])).strong());
                ui.label(RichText::new(trf("You have {}.", &[&cfg.version])).color(theme::text_dim()));
                ui.add_space(8.0);
                let btn = egui::Button::new(RichText::new(tr("Open the download page")).color(Color32::WHITE).strong()).fill(Color32::from_rgb(0x2e, 0xa0, 0x4f));
                go = ui.add_sized([ui.available_width(), 30.0], btn).clicked();
            }
            Outcome::UpToDate(tag) => {
                ui.label(RichText::new(tr("You have the newest version.")).strong());
                ui.label(RichText::new(trf("Newest release: {}", &[tag])).color(theme::text_dim()));
                ui.add_space(8.0);
                let close = egui::Button::new(RichText::new(tr("Close")).color(Color32::WHITE).strong()).fill(Color32::from_rgb(0x2f, 0x6f, 0xd0));
                shut = ui.add_sized([ui.available_width(), 30.0], close).clicked();
            }
            Outcome::Failed(e) => {
                ui.label(RichText::new(tr("Could not check for updates.")).strong());
                ui.label(RichText::new(e.as_str()).color(theme::text_dim()));
            }
        });
        if shut {
            open = false;
        }
        if go {
            ctx.open_url(egui::OpenUrl::new_tab(cfg.releases_url()));
            open = false;
        }
        if !open {
            self.window = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(parse_version("v0.0.8"), Some((0, 0, 8)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("v0.0.10-rc1"), Some((0, 0, 10)));
        assert_eq!(parse_version("nonsense"), None);
        assert!(is_newer("v0.0.9", "v0.0.8"));
        assert!(is_newer("v0.0.10", "v0.0.9"), "10 is more than 9, not less as text");
        assert!(is_newer("v0.1.0", "v0.0.99"));
        assert!(!is_newer("v0.0.8", "v0.0.8"));
        assert!(!is_newer("v0.0.7", "v0.0.8"));
        assert!(!is_newer("garbage", "v0.0.8"));
    }

    #[test]
    fn the_tag_is_read_from_the_github_answer() {
        assert_eq!(latest_tag_from_json(r#"{"id":1,"tag_name":"v0.0.9","name":"x"}"#), Some("v0.0.9".into()));
        assert_eq!(latest_tag_from_json(r#"{"message":"Not Found"}"#), None);
        assert_eq!(latest_tag_from_json("not json"), None);
    }

    #[test]
    fn a_newer_answer_opens_the_window_and_an_old_one_stays_quiet() {
        static CFG: UpdateConfig = UpdateConfig { app_name: "Test", repo: "owner/name", version: "v0.1.5" };
        let ctx = egui::Context::default();
        let mut st = UpdateState::default();
        for (tag, manual, shown) in [("v9.9.9", false, true), ("v0.1.5", false, false), ("v0.1.5", true, true)] {
            let (tx, rx) = channel();
            tx.send(Ok(tag.to_string())).unwrap();
            st.rx = Some(rx);
            st.manual = manual;
            st.window = None;
            let mut out = ctx.run_ui(egui::RawInput::default(), |_| {});
            st.tick(&ctx, &CFG);
            out.textures_delta.clear();
            assert_eq!(st.window.is_some(), shown, "{tag} manual={manual}");
        }
        // An error is only reported when asked for by hand.
        let (tx, rx) = channel();
        tx.send(Err("offline".to_string())).unwrap();
        st.rx = Some(rx);
        st.manual = false;
        st.window = None;
        st.tick(&ctx, &CFG);
        assert!(st.window.is_none());
        assert_eq!(CFG.releases_url(), "https://github.com/owner/name/releases/latest");
    }

    #[test]
    fn the_button_of_the_newer_version_window_opens_the_download_page() {
        static CFG: UpdateConfig = UpdateConfig { app_name: "Test", repo: "owner/name", version: "v0.1.5" };
        let ctx = egui::Context::default();
        let mut st = UpdateState::default();
        st.window = Some(Outcome::Newer("v9.9.9".into()));
        let opened = std::cell::RefCell::new(Vec::<String>::new());
        let frame = |st: &mut UpdateState, events: Vec<egui::Event>| {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0))), events, ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| st.tick(ui.ctx(), &CFG));
            out.textures_delta.clear();
            for c in &out.platform_output.commands {
                if let egui::OutputCommand::OpenUrl(u) = c {
                    opened.borrow_mut().push(u.url.clone());
                }
            }
        };
        frame(&mut st, vec![]);
        frame(&mut st, vec![]);
        // Click down the middle of the screen until something happens: the button is somewhere in the window.
        let mut y = 100.0;
        while y < 600.0 && opened.borrow().is_empty() && st.window.is_some() {
            let at = egui::pos2(500.0, y);
            let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
            frame(&mut st, vec![egui::Event::PointerMoved(at)]);
            frame(&mut st, vec![button(true)]);
            frame(&mut st, vec![button(false)]);
            frame(&mut st, vec![]);
            y += 8.0;
        }
        assert_eq!(*opened.borrow(), vec!["https://github.com/owner/name/releases/latest".to_string()]);
        assert!(st.window.is_none(), "the window closes");
    }

    #[test]
    fn the_about_window_shows_the_version_of_base() {
        assert!(parse_version(&format!("v{}", base_version())).is_some(), "{}", base_version());
        let ctx = egui::Context::default();
        let mut open = true;
        let info = AboutInfo { title: "About", name: "App", version: "v1.0.0", lines: vec!["a".into()], url: "https://example.org", site: "https://example.org/home", licence: "MIT", credits: include_str!("../../../CREDITS.md") };
        // A new window is only measured in its first frame: the texts are drawn in the second.
        ctx.run_ui(egui::RawInput::default(), |ui| about_window(ui.ctx(), &mut open, &info, None)).textures_delta.clear();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| about_window(ui.ctx(), &mut open, &info, None));
        out.textures_delta.clear();
        // The thanks to Jakub are drawn: the line and the link to the channel.
        let shown: String = out.shapes.iter().filter_map(|s| if let egui::epaint::Shape::Text(t) = &s.shape { Some(t.galley.text().to_string()) } else { None }).collect::<Vec<_>>().join("\n");
        assert!(shown.contains("Jakub") && shown.contains(THANKS_URL), "{shown}");
        assert!(THANKS_URL.starts_with("https://www.youtube.com/@"));
        for _ in 0..4 {
            ctx.run_ui(egui::RawInput::default(), |ui| about_window(ui.ctx(), &mut open, &info, None)).textures_delta.clear();
        }
        let r = ctx.memory(|m| m.area_rect(egui::Id::unique(Some("about_window")))).expect("the window");
        assert!(r.width() < 520.0, "the window keeps the size it was given, not the width of its longest line: {r:?}");
        // The home page and the credits are in the window.
        assert!(shown.contains("Home page") && shown.contains("resvg"), "{shown}");
    }

    #[test]
    fn the_credits_are_read_from_the_bullets_of_the_file() {
        let md = "# Credits\n\nSome words.\n\n* **[resvg / usvg](https://github.com/linebender/resvg)** (MPL-2.0): reading SVG and its font database.\n* **[Noto fonts](https://fonts.google.com/noto)** (SIL OFL 1.1): the fonts built in\n  (Latin, Greek; licence in `assets/fonts/OFL.txt`).\n* **VectorCraft**: the interface that inspired the look.\n* **Jakub ([CNC KpK](https://www.youtube.com/@CNC-KpK))**: ideas, from [industrial](https://x.org) machines.\n* Everyone who tested the program.\n";
        let c = parse_credits(md);
        assert_eq!(c.len(), 4, "a bullet without a bold name is not an entry: {c:?}");
        assert_eq!((c[0].name.as_str(), c[0].url.as_deref()), ("resvg / usvg", Some("https://github.com/linebender/resvg")));
        assert_eq!(c[0].note, "(MPL-2.0): reading SVG and its font database.");
        assert!(c[1].note.contains("fonts built in (Latin, Greek; licence in assets/fonts/OFL.txt)."), "{}", c[1].note);
        assert_eq!((c[2].name.as_str(), c[2].url.clone()), ("VectorCraft", None));
        assert_eq!(c[3].note, "ideas, from industrial machines.");
        assert_eq!(c[3].name, "Jakub (CNC KpK)");
        assert_eq!(c[3].url.as_deref(), Some("https://www.youtube.com/@CNC-KpK"));
        // The real file of this repository is read: every entry has a name, and ffmpeg is among them.
        let real = parse_credits(include_str!("../../../CREDITS.md"));
        assert!(real.len() > 10 && real.iter().all(|c| !c.name.is_empty()), "{real:?}");
        assert!(real.iter().any(|c| c.name.to_lowercase().contains("ffmpeg")), "ffmpeg is in the credits");
        assert!(real.iter().any(|c| c.name.contains("Jakub")), "Jakub is in the credits");
    }

    /// Against the real GitHub API: `cargo test -p base-ui -- --ignored real_network` (needs the internet).
    #[test]
    #[ignore = "uses the internet"]
    fn real_network_latest_release_of_a_repository() {
        static REAL: UpdateConfig = UpdateConfig { app_name: "baseCreator-test", repo: "michalsarna/LightCreator", version: "v0.0.1" };
        let tag = fetch_latest(&REAL).expect("the newest release");
        assert!(parse_version(&tag).is_some(), "{tag}");
        assert!(is_newer(&tag, REAL.version));
    }
}
