//! A small Markdown view for the notes of a device: headings, bullet and numbered lists, quotes, code, bold, italic,
//! inline code and links (only http and https are made clickable). No HTML, no images.
use crate::theme;
use egui::{self, RichText};

#[derive(Clone, Debug, PartialEq)]
pub enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link { text: String, url: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading(u8, Vec<Span>),
    Bullet(Vec<Span>),
    Numbered(String, Vec<Span>),
    Quote(Vec<Span>),
    Code(String),
    Paragraph(Vec<Span>),
    Space,
}

/// The inline parts of a line.
pub fn spans(line: &str) -> Vec<Span> {
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<Span> = vec![];
    let mut text = String::new();
    let mut i = 0;
    let flush = |text: &mut String, out: &mut Vec<Span>| {
        if !text.is_empty() {
            out.push(Span::Text(std::mem::take(text)));
        }
    };
    let find = |from: usize, pat: &[char]| -> Option<usize> { (from..chars.len().saturating_sub(pat.len() - 1)).find(|&j| chars[j..j + pat.len()] == *pat) };
    while i < chars.len() {
        let c = chars[i];
        if c == '*' && chars.get(i + 1) == Some(&'*') {
            if let Some(j) = find(i + 2, &['*', '*']).filter(|&j| j > i + 2) {
                flush(&mut text, &mut out);
                out.push(Span::Bold(chars[i + 2..j].iter().collect()));
                i = j + 2;
                continue;
            }
        }
        if (c == '*' || c == '_') && chars.get(i + 1).is_some_and(|n| !n.is_whitespace() && *n != c) {
            if let Some(j) = find(i + 1, &[c]).filter(|&j| j > i + 1) {
                flush(&mut text, &mut out);
                out.push(Span::Italic(chars[i + 1..j].iter().collect()));
                i = j + 1;
                continue;
            }
        }
        if c == '`' {
            if let Some(j) = find(i + 1, &['`']).filter(|&j| j > i + 1) {
                flush(&mut text, &mut out);
                out.push(Span::Code(chars[i + 1..j].iter().collect()));
                i = j + 1;
                continue;
            }
        }
        if c == '[' {
            if let Some(close) = find(i + 1, &[']']) {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(end) = find(close + 2, &[')']) {
                        flush(&mut text, &mut out);
                        out.push(Span::Link { text: chars[i + 1..close].iter().collect(), url: chars[close + 2..end].iter().collect() });
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        text.push(c);
        i += 1;
    }
    flush(&mut text, &mut out);
    out
}

/// The blocks of a Markdown text.
pub fn blocks(md: &str) -> Vec<Block> {
    let mut out = vec![];
    let mut code: Option<String> = None;
    for raw in md.lines() {
        let line = raw.trim_end();
        if line.trim_start().starts_with("```") {
            match code.take() {
                Some(c) => out.push(Block::Code(c)),
                None => code = Some(String::new()),
            }
            continue;
        }
        if let Some(c) = &mut code {
            if !c.is_empty() {
                c.push('\n');
            }
            c.push_str(line);
            continue;
        }
        let t = line.trim_start();
        if t.is_empty() {
            out.push(Block::Space);
        } else if let Some(rest) = t.strip_prefix("### ") {
            out.push(Block::Heading(3, spans(rest)));
        } else if let Some(rest) = t.strip_prefix("## ") {
            out.push(Block::Heading(2, spans(rest)));
        } else if let Some(rest) = t.strip_prefix("# ") {
            out.push(Block::Heading(1, spans(rest)));
        } else if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            out.push(Block::Bullet(spans(rest)));
        } else if let Some(rest) = t.strip_prefix("> ") {
            out.push(Block::Quote(spans(rest)));
        } else if let Some((n, rest)) = t.split_once(". ").filter(|(n, _)| !n.is_empty() && n.len() <= 3 && n.chars().all(|c| c.is_ascii_digit())) {
            out.push(Block::Numbered(n.to_string(), spans(rest)));
        } else {
            out.push(Block::Paragraph(spans(t)));
        }
    }
    if let Some(c) = code {
        out.push(Block::Code(c));
    }
    out
}

fn is_web(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    (u.starts_with("http://") || u.starts_with("https://")) && !u.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn show_spans(ui: &mut egui::Ui, spans: &[Span], size: f32, strong: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for s in spans {
            match s {
                Span::Text(t) => {
                    let r = RichText::new(t).size(size);
                    ui.label(if strong { r.strong() } else { r });
                }
                Span::Bold(t) => {
                    ui.label(RichText::new(t).size(size).strong());
                }
                Span::Italic(t) => {
                    ui.label(RichText::new(t).size(size).italics());
                }
                Span::Code(t) => {
                    ui.label(RichText::new(t).size(size).monospace().background_color(theme::panel_dark()));
                }
                Span::Link { text, url } => {
                    if is_web(url) {
                        ui.hyperlink_to(RichText::new(text).size(size), url.trim());
                    } else {
                        // Anything else is not a link: show it as it was written.
                        ui.label(RichText::new(format!("[{text}]({url})")).size(size));
                    }
                }
            }
        }
    });
}

/// Draw the Markdown text.
pub fn show(ui: &mut egui::Ui, md: &str) {
    for b in blocks(md) {
        match b {
            Block::Heading(n, s) => show_spans(ui, &s, [0.0, 20.0, 17.0, 15.0][n as usize], true),
            Block::Bullet(s) => {
                ui.horizontal_wrapped(|ui| {
                    ui.label("•");
                });
                show_spans(ui, &s, 13.5, false);
            }
            Block::Numbered(n, s) => {
                ui.label(format!("{n}."));
                show_spans(ui, &s, 13.5, false);
            }
            Block::Quote(s) => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("▌").color(theme::text_dim()));
                    show_spans(ui, &s, 13.5, false);
                });
            }
            Block::Code(c) => {
                egui::Frame::new().fill(theme::panel_dark()).inner_margin(6.0).corner_radius(3).show(ui, |ui| {
                    ui.label(RichText::new(c).monospace());
                });
            }
            Block::Paragraph(s) => show_spans(ui, &s, 13.5, false),
            Block::Space => ui.add_space(6.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_parts_are_found_and_unclosed_marks_stay_text() {
        assert_eq!(spans("plain"), vec![Span::Text("plain".into())]);
        assert_eq!(
            spans("a **bold** and *it* and `x = 1` and [site](https://example.org)."),
            vec![
                Span::Text("a ".into()),
                Span::Bold("bold".into()),
                Span::Text(" and ".into()),
                Span::Italic("it".into()),
                Span::Text(" and ".into()),
                Span::Code("x = 1".into()),
                Span::Text(" and ".into()),
                Span::Link { text: "site".into(), url: "https://example.org".into() },
                Span::Text(".".into()),
            ]
        );
        // Marks that never close, or hold nothing, are text.
        assert_eq!(spans("2 * 3 = 6 and ** nothing"), vec![Span::Text("2 * 3 = 6 and ** nothing".into())]);
        assert_eq!(spans("a `b"), vec![Span::Text("a `b".into())]);
        assert_eq!(spans("[no link]"), vec![Span::Text("[no link]".into())]);
        assert_eq!(spans("snake_case_name"), vec![Span::Text("snake".into()), Span::Italic("case".into()), Span::Text("name".into())]);
    }

    #[test]
    fn blocks_follow_the_lines() {
        let md = "# Title\n\nSome text.\n- one\n* two\n1. first\n> quoted\n```\ncode line\nmore\n```\n### small";
        let b = blocks(md);
        assert_eq!(b[0], Block::Heading(1, vec![Span::Text("Title".into())]));
        assert_eq!(b[1], Block::Space);
        assert!(matches!(&b[2], Block::Paragraph(_)));
        assert!(matches!(&b[3], Block::Bullet(_)) && matches!(&b[4], Block::Bullet(_)));
        assert_eq!(b[5], Block::Numbered("1".into(), vec![Span::Text("first".into())]));
        assert!(matches!(&b[6], Block::Quote(_)));
        assert_eq!(b[7], Block::Code("code line\nmore".into()));
        assert_eq!(b[8], Block::Heading(3, vec![Span::Text("small".into())]));
        // An unclosed fence still shows its text; nothing panics on odd input.
        assert_eq!(blocks("```\nopen"), vec![Block::Code("open".into())]);
        let _ = blocks("\u{0}\u{1b}[2J ** __ ``` > - 1.");
        assert!(blocks("").is_empty());
    }

    #[test]
    fn only_web_links_are_clickable() {
        assert!(is_web("https://example.org/a?b=c"));
        assert!(is_web(" HTTP://example.org "));
        assert!(!is_web("javascript:alert(1)"));
        assert!(!is_web("file:///etc/passwd"));
        assert!(!is_web("https://exa mple.org"));
        assert!(!is_web("example.org"));
    }
}
