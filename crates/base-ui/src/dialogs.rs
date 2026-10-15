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
    pub licence: &'a str,
}

/// The About window. `logo` is the picture shown next to the name.
pub fn about_window(ctx: &egui::Context, open: &mut bool, info: &AboutInfo, logo: Option<&egui::TextureHandle>) {
    if !*open {
        return;
    }
    popup(ctx, "about_window", info.title, [380.0, 300.0], open, |ui| {
        ui.horizontal(|ui| {
            if let Some(logo) = logo {
                ui.image((logo.id(), egui::vec2(72.0, 72.0)));
            }
            ui.vertical(|ui| {
                ui.heading(info.name);
                ui.label(trf("Version {}", &[&info.version]));
            });
        });
        for (i, line) in info.lines.iter().enumerate() {
            ui.label(line);
            if i == 0 {
                ui.separator();
            }
        }
        ui.hyperlink(info.url);
        ui.label(format!("{}: {}", tr("Licence"), info.licence));
    });
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
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
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
