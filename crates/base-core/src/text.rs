//! Text to outlines: glyph shapes from system fonts (plus a bundled fallback) as editable Bézier contours.
//! Layout is simple left-to-right with advance widths; complex scripts (Devanagari, Arabic) are not shaped.
use crate::bezier::{Contour, Node};
use crate::geom::Pt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use usvg::fontdb;

/// Family name of the font bundled with the program, used when nothing else matches.
pub const DEFAULT_FAMILY: &str = "Noto Sans";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TextData {
    pub text: String,
    pub family: String,
    /// Em size in millimetres.
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    /// Extra space after each glyph, as a fraction of the size.
    pub spacing: f64,
    /// Line distance as a multiple of the size.
    pub line_spacing: f64,
    /// 0 = left, 1 = centre, 2 = right.
    pub align: u8,
}

impl Default for TextData {
    fn default() -> Self {
        TextData { text: "Text".into(), family: DEFAULT_FAMILY.into(), size: 10.0, bold: false, italic: false, spacing: 0.0, line_spacing: 1.2, align: 0 }
    }
}

fn db() -> &'static fontdb::Database {
    static DB: OnceLock<fontdb::Database> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = fontdb::Database::new();
        // The fonts of the application itself come first so text always works, even without system fonts.
        db.load_font_data(include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf").to_vec());
        db.load_font_data(include_bytes!("../../../assets/fonts/NotoSansMono-Regular.ttf").to_vec());
        db.load_font_data(include_bytes!("../../../assets/fonts/NotoSansSC-Regular.otf").to_vec());
        db.load_font_data(include_bytes!("../../../assets/fonts/NotoSansDevanagari-Regular.ttf").to_vec());
        db.load_system_fonts();
        db
    })
}

/// Start loading the system fonts in the background so the first text object is instant.
pub fn preload() {
    std::thread::spawn(|| {
        let _ = db();
    });
}

/// Sorted, de-duplicated family names available for text.
pub fn families() -> Vec<String> {
    let mut v: Vec<String> = db().faces().filter_map(|f| f.families.first().map(|(n, _)| n.clone())).filter(|n| !n.starts_with('.')).collect();
    v.sort_by_key(|s| s.to_lowercase());
    v.dedup();
    v
}

fn find_face(t: &TextData) -> Option<fontdb::ID> {
    let db = db();
    let q = |name: &str| {
        db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(name)],
            weight: if t.bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
            stretch: fontdb::Stretch::Normal,
            style: if t.italic { fontdb::Style::Italic } else { fontdb::Style::Normal },
        })
    };
    q(&t.family).or_else(|| q(DEFAULT_FAMILY))
}

struct Collector {
    contours: Vec<Contour>,
    cur: Vec<Node>,
    last: Pt,
    ox: f64,
    oy: f64,
    s: f64,
}

impl Collector {
    fn pt(&self, x: f32, y: f32) -> Pt {
        Pt::new(self.ox + x as f64 * self.s, self.oy - y as f64 * self.s)
    }
    fn end(&mut self, closed: bool) {
        let mut nodes = std::mem::take(&mut self.cur);
        if closed && nodes.len() > 2 && nodes[0].p.dist(nodes[nodes.len() - 1].p) < 1e-9 {
            let last = nodes.pop().unwrap_or(nodes[0]);
            nodes[0].hin = last.hin;
        }
        if nodes.len() >= 2 {
            self.contours.push(Contour { nodes, closed });
        }
    }
}

impl ttf_parser::OutlineBuilder for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        self.end(false);
        self.last = self.pt(x, y);
        self.cur.push(Node::corner(self.last));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.last = self.pt(x, y);
        self.cur.push(Node::corner(self.last));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (c, p) = (self.pt(x1, y1), self.pt(x, y));
        let l = self.last;
        let c1 = Pt::new(l.x + 2.0 / 3.0 * (c.x - l.x), l.y + 2.0 / 3.0 * (c.y - l.y));
        let c2 = Pt::new(p.x + 2.0 / 3.0 * (c.x - p.x), p.y + 2.0 / 3.0 * (c.y - p.y));
        if let Some(n) = self.cur.last_mut() {
            n.hout = c1;
        }
        self.cur.push(Node { p, hin: c2, hout: p, smooth: false });
        self.last = p;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, p) = (self.pt(x1, y1), self.pt(x2, y2), self.pt(x, y));
        if let Some(n) = self.cur.last_mut() {
            n.hout = c1;
        }
        self.cur.push(Node { p, hin: c2, hout: p, smooth: false });
        self.last = p;
    }
    fn close(&mut self) {
        self.end(true);
    }
}

fn layout(t: &TextData) -> Vec<Contour> {
    let Some(id) = find_face(t) else { return vec![] };
    let size = t.size.max(0.1);
    let mut out = vec![];
    db().with_face_data(id, |data, index| {
        let Ok(face) = ttf_parser::Face::parse(data, index) else { return };
        let upem = face.units_per_em().max(1) as f64;
        let s = size / upem;
        let advance_of = |c: char| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g)).map(|a| a as f64 * s).unwrap_or(size * 0.3);
        let lines: Vec<&str> = t.text.split('\n').collect();
        let widths: Vec<f64> = lines.iter().map(|l| l.chars().map(|c| advance_of(c) + t.spacing * size).sum::<f64>()).collect();
        let max_w = widths.iter().cloned().fold(0.0, f64::max);
        let ascent = face.ascender() as f64 * s;
        for (li, line) in lines.iter().enumerate() {
            let shift = match t.align {
                1 => (max_w - widths[li]) / 2.0,
                2 => max_w - widths[li],
                _ => 0.0,
            };
            let mut pen = shift;
            let baseline = ascent + li as f64 * t.line_spacing * size;
            for c in line.chars() {
                if let Some(g) = face.glyph_index(c) {
                    let mut col = Collector { contours: vec![], cur: vec![], last: Pt::default(), ox: pen, oy: baseline, s };
                    if face.outline_glyph(g, &mut col).is_some() {
                        col.end(false);
                        out.extend(col.contours);
                    }
                }
                pen += advance_of(c) + t.spacing * size;
            }
        }
    });
    out
}

/// Glyph outlines for `t` in local coordinates (top-left origin, y down). Results are cached.
pub fn contours(t: &TextData) -> Arc<Vec<Contour>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Vec<Contour>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = serde_json::to_string(t).unwrap_or_default();
    if let Ok(c) = cache.lock() {
        if let Some(hit) = c.get(&key) {
            return hit.clone();
        }
    }
    let v = Arc::new(layout(t));
    if let Ok(mut c) = cache.lock() {
        if c.len() > 512 {
            c.clear();
        }
        c.insert(key, v.clone());
    }
    v
}
