//! Style trace: a picture drawn again as pen lines in a style (squiggles, a spiral, stippling, hatching, flow lines…), for a
//! plotter to draw, a laser to burn or a mill to engrave. Everything is in the local millimetres of the picture (y down, the
//! picture from (0, 0) to (w, h)); the lines are put on the drawing with the transform of the picture.
//!
//! The styles sample the darkness of the picture in millimetres, so what they make does not depend on the number of pixels:
//! the preview is the result. Random styles take a seed and give the same lines for the same seed on every computer.
use crate::geom::{Polyline, Pt};
use crate::image::ImageData;
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};

/// The drawing styles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Style {
    #[default]
    Squiggle,
    Sawtooth,
    Springs,
    Spiral,
    Waves,
    Halftone,
    Etching,
    Squares,
    Needles,
    Truchet,
    Contours,
    Hilbert,
    Stipple,
    Tsp,
    Delaunay,
    LineArt,
    FlowLines,
    Sketch,
    Scribble,
}

/// The settings a style uses; their meaning (and the label) depends on the style, see [`Style::knobs`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Knob {
    Spacing,
    Strength,
    Detail,
    Angle,
}

/// How a setting of a style is shown: its label, the range and the unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnobSpec {
    pub knob: Knob,
    pub label: &'static str,
    pub min: f64,
    pub max: f64,
    pub unit: &'static str,
    /// Digits after the point (0: a whole number).
    pub decimals: usize,
}

const fn k(knob: Knob, label: &'static str, min: f64, max: f64, unit: &'static str, decimals: usize) -> KnobSpec {
    KnobSpec { knob, label, min, max, unit, decimals }
}

const ANGLE: KnobSpec = k(Knob::Angle, "Angle", -90.0, 90.0, "°", 0);
const LINE_GAP: KnobSpec = k(Knob::Spacing, "Spacing of the lines", 0.3, 20.0, " mm", 2);
const WAVE_HEIGHT: KnobSpec = k(Knob::Strength, "Wave height", 0.0, 1.5, "", 2);
const WAVES_PER_MM: KnobSpec = k(Knob::Detail, "Waves per mm", 0.1, 6.0, "", 2);
const CELL: KnobSpec = k(Knob::Spacing, "Cell size", 0.5, 30.0, " mm", 2);
const DOTS: KnobSpec = k(Knob::Detail, "Number of dots", 100.0, 30000.0, "", 0);

impl Style {
    pub const ALL: [Style; 19] = [
        Style::Squiggle,
        Style::Sawtooth,
        Style::Springs,
        Style::Spiral,
        Style::Waves,
        Style::Halftone,
        Style::Etching,
        Style::Squares,
        Style::Needles,
        Style::Truchet,
        Style::Contours,
        Style::Hilbert,
        Style::Stipple,
        Style::Tsp,
        Style::Delaunay,
        Style::LineArt,
        Style::FlowLines,
        Style::Sketch,
        Style::Scribble,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Style::Squiggle => "Squiggle",
            Style::Sawtooth => "Sawtooth",
            Style::Springs => "Springs",
            Style::Spiral => "Spiral",
            Style::Waves => "Waves",
            Style::Halftone => "Halftone circles",
            Style::Etching => "Etching",
            Style::Squares => "Squares",
            Style::Needles => "Needles",
            Style::Truchet => "Truchet tiles",
            Style::Contours => "Contour lines",
            Style::Hilbert => "Hilbert curve",
            Style::Stipple => "Stippling",
            Style::Tsp => "One line (TSP)",
            Style::Delaunay => "Triangles and cells",
            Style::LineArt => "Line art",
            Style::FlowLines => "Flow lines",
            Style::Sketch => "Sketch",
            Style::Scribble => "Scribble",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Style::Squiggle => "Rows of lines that wave faster and higher where the picture is darker.",
            Style::Sawtooth => "Rows of zigzags, closer and higher where the picture is darker.",
            Style::Springs => "Rows of loops like a stretched spring, bigger where the picture is darker.",
            Style::Spiral => "One spiral from the middle that wobbles more in the shadows.",
            Style::Waves => "Rows lifted by the darkness of the picture, like a landscape seen from the side.",
            Style::Halftone => "A grid of circles, bigger where the picture is darker.",
            Style::Etching => "Hatching in layers: every darker tone adds lines in another direction.",
            Style::Squares => "A grid of squares one inside the other, more of them where it is darker.",
            Style::Needles => "Short strokes on a grid, longer where it is darker, turned along the shapes.",
            Style::Truchet => "Tiles of quarter circles turned at random, more arcs where it is darker.",
            Style::Contours => "Lines of equal brightness, like the contour lines of a map.",
            Style::Hilbert => "One space-filling curve, with finer turns where it is darker.",
            Style::Stipple => "Dots spread by the tone of the picture: close in the shadows, sparse in the lights.",
            Style::Tsp => "The dots of stippling joined into one line that does not cross itself.",
            Style::Delaunay => "A mesh of triangles, or of the cells round them, over dots spread by the tone.",
            Style::LineArt => "Only the edges of the picture, as lines.",
            Style::FlowLines => "Long lines that flow along the shapes, closer together where it is darker.",
            Style::Sketch => "Many short straight strokes laid where the picture is darkest, as in a pencil sketch.",
            Style::Scribble => "One wandering line of loops that scribbles the dark parts in.",
        }
    }

    /// The settings the style uses, in the order they are shown.
    pub fn knobs(self) -> &'static [KnobSpec] {
        match self {
            Style::Squiggle | Style::Sawtooth | Style::Springs => { const L: &[KnobSpec] = &[LINE_GAP, WAVE_HEIGHT, WAVES_PER_MM, ANGLE]; L },
            Style::Spiral => { const L: &[KnobSpec] = &[k(Knob::Spacing, "Spacing of the turns", 0.3, 20.0, " mm", 2), WAVE_HEIGHT, WAVES_PER_MM]; L },
            Style::Waves => { const L: &[KnobSpec] = &[LINE_GAP, k(Knob::Strength, "Height of the waves", 0.0, 10.0, "", 1)]; L },
            Style::Halftone => { const L: &[KnobSpec] = &[CELL, k(Knob::Strength, "Largest circle", 0.2, 1.4, "", 2), k(Knob::Detail, "Spacing of the rings", 0.1, 5.0, " mm", 2), ANGLE]; L },
            Style::Etching => { const L: &[KnobSpec] = &[LINE_GAP, k(Knob::Detail, "Tone levels", 1.0, 6.0, "", 0), ANGLE]; L },
            Style::Squares => { const L: &[KnobSpec] = &[CELL, k(Knob::Detail, "Most squares in a cell", 1.0, 12.0, "", 0), k(Knob::Strength, "Shake", 0.0, 1.0, "", 2), ANGLE]; L },
            Style::Needles => { const L: &[KnobSpec] = &[CELL, k(Knob::Strength, "Stroke length", 0.2, 3.0, "", 2), ANGLE]; L },
            Style::Truchet => { const L: &[KnobSpec] = &[k(Knob::Spacing, "Tile size", 1.0, 30.0, " mm", 2), k(Knob::Detail, "Most arcs in a tile", 1.0, 10.0, "", 0)]; L },
            Style::Contours => { const L: &[KnobSpec] = &[k(Knob::Spacing, "Smoothing", 0.2, 10.0, " mm", 2), k(Knob::Detail, "Tone levels", 2.0, 40.0, "", 0)]; L },
            Style::Hilbert => { const L: &[KnobSpec] = &[k(Knob::Spacing, "Smallest step", 0.3, 10.0, " mm", 2), k(Knob::Detail, "Tone levels", 1.0, 6.0, "", 0)]; L },
            Style::Stipple => { const L: &[KnobSpec] = &[DOTS, k(Knob::Strength, "Dot size", 0.05, 5.0, " mm", 2)]; L },
            Style::Tsp => { const L: &[KnobSpec] = &[k(Knob::Detail, "Number of dots", 100.0, 15000.0, "", 0)]; L },
            Style::Delaunay => { const L: &[KnobSpec] = &[k(Knob::Detail, "Number of dots", 50.0, 6000.0, "", 0)]; L },
            Style::LineArt => {
                { const L: &[KnobSpec] = &[k(Knob::Spacing, "Smallest detail", 0.1, 5.0, " mm", 2), k(Knob::Strength, "Edge sensitivity", 0.0, 1.0, "", 2), k(Knob::Detail, "Shortest line", 0.0, 50.0, " mm", 1)]; L }
            }
            Style::FlowLines => { const L: &[KnobSpec] = &[
                k(Knob::Spacing, "Smallest gap", 0.2, 10.0, " mm", 2),
                k(Knob::Detail, "Largest gap", 0.5, 40.0, " mm", 2),
                k(Knob::Strength, "Size of the curves", 1.0, 200.0, " mm", 1),
                ANGLE,
            ]; L },
            Style::Sketch => { const L: &[KnobSpec] = &[k(Knob::Spacing, "Stroke length", 0.5, 50.0, " mm", 1), k(Knob::Detail, "Number of strokes", 50.0, 30000.0, "", 0), k(Knob::Strength, "Ink of one stroke", 0.02, 1.0, "", 2)]; L },
            Style::Scribble => { const L: &[KnobSpec] = &[k(Knob::Spacing, "Loop size", 0.3, 20.0, " mm", 2), k(Knob::Detail, "Length of the line", 0.5, 500.0, " m", 1), k(Knob::Strength, "Ink of one pass", 0.02, 1.0, "", 2)]; L },
        }
    }

    /// The variants a style can be drawn in (empty: one way only).
    pub fn variants(self) -> &'static [&'static str] {
        match self {
            Style::Squiggle | Style::Sawtooth | Style::Springs => &["Rows from edge to edge", "Lift the pen over white"],
            Style::Spiral => &["Waves", "Zigzag"],
            Style::Waves => &["Hide what is behind", "Draw every row whole"],
            Style::Halftone => &["Circles", "Rings", "Spirals"],
            Style::Etching => &["Straight lines", "Wavy lines"],
            Style::Needles => &["Along the shapes", "At the angle", "At random"],
            Style::Hilbert => &["One line", "Lift the pen over white"],
            Style::Stipple => &["Dots", "Circles by tone", "Dashes", "Triangles"],
            Style::Delaunay => &["Triangles", "Cells"],
            Style::LineArt => &["Edges", "Edges and the dark areas"],
            Style::FlowLines => &["Along the shapes", "Flow field"],
            Style::Sketch => &["Separate strokes", "Strokes in chains"],
            _ => &[],
        }
    }

    /// Does the style use random numbers (so a seed changes it)?
    pub fn random(self) -> bool {
        matches!(self, Style::Squares | Style::Needles | Style::Truchet | Style::Stipple | Style::Tsp | Style::Delaunay | Style::FlowLines | Style::Sketch | Style::Scribble)
    }

    /// The starting settings of the style (the tone settings are kept when the style changes).
    pub fn defaults(self) -> StyleParams {
        let p = StyleParams { style: self, ..StyleParams::default() };
        let (spacing, strength, detail, angle) = match self {
            Style::Squiggle => (1.5, 0.9, 1.0, 0.0),
            Style::Sawtooth => (1.5, 0.9, 1.0, 0.0),
            Style::Springs => (2.0, 0.6, 0.6, 0.0),
            Style::Spiral => (1.2, 0.9, 1.2, 0.0),
            Style::Waves => (2.0, 3.0, 0.0, 0.0),
            Style::Halftone => (3.0, 1.0, 0.5, 45.0),
            Style::Etching => (1.0, 0.0, 4.0, 45.0),
            Style::Squares => (4.0, 0.0, 5.0, 0.0),
            Style::Needles => (2.5, 1.0, 0.0, 45.0),
            Style::Truchet => (5.0, 0.0, 5.0, 0.0),
            Style::Contours => (1.0, 0.0, 10.0, 0.0),
            Style::Hilbert => (0.5, 0.0, 5.0, 0.0),
            Style::Stipple => (0.0, 0.4, 4000.0, 0.0),
            Style::Tsp => (0.0, 0.0, 3000.0, 0.0),
            Style::Delaunay => (0.0, 0.0, 1200.0, 0.0),
            Style::LineArt => (0.5, 0.5, 2.0, 0.0),
            Style::FlowLines => (0.8, 20.0, 6.0, 0.0),
            Style::Sketch => (6.0, 0.3, 3000.0, 0.0),
            Style::Scribble => (3.0, 0.25, 60.0, 0.0),
        };
        StyleParams { spacing, strength, detail, angle, ..p }
    }
}

/// What a style trace is made with. `Copy` and `PartialEq`, so a window sees when something changed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StyleParams {
    pub style: Style,
    pub spacing: f64,
    pub strength: f64,
    pub detail: f64,
    /// Degrees.
    pub angle: f64,
    /// The index into [`Style::variants`].
    pub variant: u8,
    /// The seed of the random styles.
    pub seed: u32,
    /// The tone: the darkest and the lightest grey kept (0..=1), the gamma (above 1 brightens the midtones), dark for light.
    pub black: f64,
    pub white: f64,
    pub gamma: f64,
    pub invert: bool,
}

impl Default for StyleParams {
    fn default() -> Self {
        StyleParams { style: Style::Squiggle, spacing: 1.5, strength: 0.9, detail: 1.0, angle: 0.0, variant: 0, seed: 1, black: 0.0, white: 1.0, gamma: 1.0, invert: false }
    }
}

impl StyleParams {
    pub fn get(&self, k: Knob) -> f64 {
        match k {
            Knob::Spacing => self.spacing,
            Knob::Strength => self.strength,
            Knob::Detail => self.detail,
            Knob::Angle => self.angle,
        }
    }

    pub fn set(&mut self, k: Knob, v: f64) {
        match k {
            Knob::Spacing => self.spacing = v,
            Knob::Strength => self.strength = v,
            Knob::Detail => self.detail = v,
            Knob::Angle => self.angle = v,
        }
    }

    /// Change the style: its own settings, the tone and the seed kept.
    pub fn with_style(&self, s: Style) -> StyleParams {
        StyleParams { black: self.black, white: self.white, gamma: self.gamma, invert: self.invert, seed: self.seed, ..s.defaults() }
    }

    /// Every setting within the range of its style (a file or an old window may hold anything).
    pub fn clamped(&self) -> StyleParams {
        let mut p = *self;
        for s in self.style.knobs() {
            let v = p.get(s.knob);
            p.set(s.knob, if v.is_finite() { v.clamp(s.min, s.max) } else { s.min });
        }
        let n = self.style.variants().len().max(1) as u8;
        p.variant = p.variant.min(n - 1);
        p.black = p.black.clamp(0.0, 0.99);
        p.white = p.white.clamp(p.black + 0.01, 1.0);
        p.gamma = if p.gamma.is_finite() { p.gamma.clamp(0.2, 5.0) } else { 1.0 };
        p
    }
}

/// The lines of a style trace.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleOut {
    pub lines: Vec<Polyline>,
    /// Stopped at the limit of points (the result is only a part).
    pub truncated: bool,
}

impl StyleOut {
    /// The length of all the lines, millimetres.
    pub fn length(&self) -> f64 {
        self.lines.iter().map(|l| l.length()).sum()
    }
}

/// The most points a result may have; more would make the drawing slow to edit and to undo.
pub const MAX_POINTS: usize = 2_000_000;

/// Draw the picture in the style. `stop` is asked now and then and ends the work early (a newer setting came).
pub fn style_trace(im: &ImageData, params: &StyleParams, stop: &dyn Fn() -> bool) -> StyleOut {
    let p = params.clamped();
    if im.w <= 0.0 || im.h <= 0.0 || im.px_w == 0 || im.px_h == 0 {
        return StyleOut::default();
    }
    let mut out = Out { lines: vec![], points: 0, truncated: false };
    let order = match p.style {
        Style::Squiggle | Style::Sawtooth | Style::Springs => {
            rows_wave(im, &p, &mut out);
            false
        }
        Style::Spiral => {
            spiral(im, &p, &mut out);
            false
        }
        Style::Waves => {
            waves(im, &p, &mut out);
            false
        }
        Style::Halftone => {
            halftone(im, &p, &mut out);
            true
        }
        Style::Etching => {
            etching(im, &p, &mut out);
            true
        }
        Style::Squares => {
            squares(im, &p, &mut out);
            true
        }
        Style::Needles => {
            needles(im, &p, &mut out);
            true
        }
        Style::Truchet => {
            truchet(im, &p, &mut out);
            true
        }
        Style::Contours => {
            contours(im, &p, &mut out);
            true
        }
        Style::Hilbert => {
            hilbert(im, &p, &mut out);
            false
        }
        Style::Stipple => {
            stipple(im, &p, &mut out, stop);
            true
        }
        Style::Tsp => {
            tsp(im, &p, &mut out, stop);
            false
        }
        Style::Delaunay => {
            delaunay(im, &p, &mut out, stop);
            true
        }
        Style::LineArt => {
            line_art(im, &p, &mut out);
            true
        }
        Style::FlowLines => {
            flow_lines(im, &p, &mut out, stop);
            true
        }
        Style::Sketch => {
            sketch(im, &p, &mut out, stop);
            true
        }
        Style::Scribble => {
            scribble(im, &p, &mut out, stop);
            false
        }
    };
    let mut lines = out.lines;
    lines.retain(|l| l.pts.len() >= 2 && l.pts.iter().all(|q| q.x.is_finite() && q.y.is_finite()));
    // Points closer to a straight line than a hundredth of a millimetre are left out: a wave of many small steps keeps its shape.
    for l in lines.iter_mut().filter(|l| l.pts.len() > 16) {
        l.pts = simplify(&l.pts, 0.01);
    }
    if order && !stop() {
        lines = order_nearest(lines);
    }
    StyleOut { lines, truncated: out.truncated }
}

// ---------------------------------------------------------------------------------------------------------------------------
// Output, random numbers, the darkness field
// ---------------------------------------------------------------------------------------------------------------------------

struct Out {
    lines: Vec<Polyline>,
    points: usize,
    truncated: bool,
}

impl Out {
    /// Room for more? Sets `truncated` once the limit is reached.
    fn room(&mut self) -> bool {
        if self.points >= MAX_POINTS {
            self.truncated = true;
        }
        !self.truncated
    }

    fn push(&mut self, pts: Vec<Pt>, closed: bool) {
        if pts.len() < 2 || !self.room() {
            return;
        }
        self.points += pts.len();
        self.lines.push(Polyline::new(pts, closed));
    }
}

/// A line drawn piece by piece: points outside the picture (or where `draw` is false) lift the pen.
struct Pen {
    cur: Vec<Pt>,
}

impl Pen {
    fn new() -> Pen {
        Pen { cur: vec![] }
    }

    fn to(&mut self, out: &mut Out, p: Pt, draw: bool) {
        if draw {
            self.cur.push(p);
        } else {
            self.lift(out);
        }
    }

    fn lift(&mut self, out: &mut Out) {
        let pts = std::mem::take(&mut self.cur);
        out.push(pts, false);
    }
}

/// A small random generator (xorshift64*): the same seed gives the same numbers everywhere.
struct Rng(u64);

impl Rng {
    fn new(seed: u32, stream: u64) -> Rng {
        let mut s = (seed as u64) ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
        // splitmix64 so that close seeds start far apart
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Rng(if z == 0 { 1 } else { z })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// 0..1
    fn f(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
}

/// The darkness of the picture on a grid of square cells (0 = white, 1 = black), with the tone settings applied.
#[derive(Clone)]
struct Field {
    w: f64,
    h: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    v: Vec<f32>,
}

impl Field {
    /// The picture averaged into cells of about `cell` millimetres (never smaller than its pixels, never more than
    /// `max_cells` cells).
    fn new(im: &ImageData, p: &StyleParams, cell: f64, max_cells: usize) -> Field {
        let px = (im.w / im.px_w as f64).max(im.h / im.px_h as f64);
        let mut cell = cell.max(px * 0.999).max(1e-3);
        let area = im.w * im.h;
        if area / (cell * cell) > max_cells as f64 {
            cell = (area / max_cells as f64).sqrt();
        }
        let nx = ((im.w / cell).ceil() as usize).max(1);
        let ny = ((im.h / cell).ceil() as usize).max(1);
        let mut sum = vec![0f64; nx * ny];
        let mut cnt = vec![0u32; nx * ny];
        let (sx, sy) = (im.w / im.px_w as f64, im.h / im.px_h as f64);
        for y in 0..im.px_h as usize {
            let cy = (((y as f64 + 0.5) * sy / cell) as usize).min(ny - 1);
            for x in 0..im.px_w as usize {
                let cx = (((x as f64 + 0.5) * sx / cell) as usize).min(nx - 1);
                sum[cy * nx + cx] += im.gray[y * im.px_w as usize + x] as f64;
                cnt[cy * nx + cx] += 1;
            }
        }
        // Cells no pixel fell into (cells smaller than a pixel along one side): sampled from the picture.
        let flip = im.invert ^ p.invert;
        let span = (p.white - p.black).max(1e-3);
        let v = (0..nx * ny)
            .map(|i| {
                let g = if cnt[i] > 0 {
                    sum[i] / cnt[i] as f64 / 255.0
                } else {
                    let (cx, cy) = ((i % nx) as f64 + 0.5, (i / nx) as f64 + 0.5);
                    let xx = ((cx * cell / sx) as usize).min(im.px_w as usize - 1);
                    let yy = ((cy * cell / sy) as usize).min(im.px_h as usize - 1);
                    im.gray[yy * im.px_w as usize + xx] as f64 / 255.0
                };
                let g = ((g - p.black) / span).clamp(0.0, 1.0).powf(1.0 / p.gamma);
                let d = 1.0 - g;
                (if flip { 1.0 - d } else { d }) as f32
            })
            .collect();
        Field { w: im.w, h: im.h, cell, nx, ny, v }
    }

    fn get(&self, x: usize, y: usize) -> f32 {
        self.v[y.min(self.ny - 1) * self.nx + x.min(self.nx - 1)]
    }

    fn inside(&self, q: Pt) -> bool {
        q.x >= 0.0 && q.y >= 0.0 && q.x <= self.w && q.y <= self.h
    }

    /// Darkness at a point (bilinear between the centres of the cells; 0 outside the picture).
    fn at(&self, q: Pt) -> f64 {
        if !self.inside(q) {
            return 0.0;
        }
        let fx = (q.x / self.cell - 0.5).clamp(0.0, (self.nx - 1) as f64);
        let fy = (q.y / self.cell - 0.5).clamp(0.0, (self.ny - 1) as f64);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let v = self.get(x0, y0) * (1.0 - tx) * (1.0 - ty) + self.get(x0 + 1, y0) * tx * (1.0 - ty) + self.get(x0, y0 + 1) * (1.0 - tx) * ty + self.get(x0 + 1, y0 + 1) * tx * ty;
        v as f64
    }

    /// The gradient of the darkness (per millimetre), towards darker.
    fn grad(&self, q: Pt) -> Pt {
        let e = self.cell;
        let c = |x: f64, y: f64| self.at(Pt::new(x.clamp(0.0, self.w), y.clamp(0.0, self.h)));
        Pt::new((c(q.x + e, q.y) - c(q.x - e, q.y)) / (2.0 * e), (c(q.x, q.y + e) - c(q.x, q.y - e)) / (2.0 * e))
    }

    /// The same field blurred by about `r` millimetres (three box blurs, close to a Gaussian).
    fn blurred(&self, r: f64) -> Field {
        let k = (r / self.cell).round() as usize;
        if k == 0 {
            return self.clone();
        }
        let mut f = self.clone();
        for _ in 0..3 {
            f.box_blur(k);
        }
        f
    }

    fn box_blur(&mut self, k: usize) {
        let (nx, ny) = (self.nx, self.ny);
        let mut tmp = vec![0f32; nx * ny];
        for y in 0..ny {
            let row = &self.v[y * nx..(y + 1) * nx];
            let mut acc = 0f32;
            let mut n = 0usize;
            for x in 0..k.min(nx) {
                acc += row[x];
                n += 1;
            }
            for x in 0..nx {
                if x + k < nx {
                    acc += row[x + k];
                    n += 1;
                }
                if x > k {
                    acc -= row[x - k - 1];
                    n -= 1;
                }
                tmp[y * nx + x] = acc / n as f32;
            }
        }
        for x in 0..nx {
            let mut acc = 0f32;
            let mut n = 0usize;
            for y in 0..k.min(ny) {
                acc += tmp[y * nx + x];
                n += 1;
            }
            for y in 0..ny {
                if y + k < ny {
                    acc += tmp[(y + k) * nx + x];
                    n += 1;
                }
                if y > k {
                    acc -= tmp[(y - k - 1) * nx + x];
                    n -= 1;
                }
                self.v[y * nx + x] = acc / n as f32;
            }
        }
    }
}

/// The mean darkness over rectangles, from a summed-area table of a field.
struct Sat {
    f: Field,
    s: Vec<f64>,
}

impl Sat {
    fn new(f: Field) -> Sat {
        let (nx, ny) = (f.nx, f.ny);
        let mut s = vec![0f64; (nx + 1) * (ny + 1)];
        for y in 0..ny {
            let mut row = 0f64;
            for x in 0..nx {
                row += f.v[y * nx + x] as f64;
                s[(y + 1) * (nx + 1) + x + 1] = s[y * (nx + 1) + x + 1] + row;
            }
        }
        Sat { f, s }
    }

    /// The mean darkness of the rectangle (millimetres), the part outside the picture counted as white.
    fn mean(&self, x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
        let (nx, ny, c) = (self.f.nx, self.f.ny, self.f.cell);
        let ix = |x: f64| ((x / c).round().max(0.0) as usize).min(nx);
        let iy = |y: f64| ((y / c).round().max(0.0) as usize).min(ny);
        let (a, b, cc, d) = (ix(x0), iy(y0), ix(x1), iy(y1));
        let full = ((x1 - x0) * (y1 - y0) / (c * c)).max(1e-9);
        if cc <= a || d <= b {
            return self.f.at(Pt::new((x0 + x1) / 2.0, (y0 + y1) / 2.0));
        }
        let w = nx + 1;
        let sum = self.s[d * w + cc] - self.s[b * w + cc] - self.s[d * w + a] + self.s[b * w + a];
        (sum / full.max(((cc - a) * (d - b)) as f64)).clamp(0.0, 1.0)
    }
}

/// Rows across the picture at `angle` degrees, `gap` apart: for each row its start, its direction and its normal, and the
/// length to walk along it.
fn rows(w: f64, h: f64, angle: f64, gap: f64) -> Vec<(Pt, Pt, Pt, f64)> {
    let a = angle.to_radians();
    let (u, n) = (Pt::new(a.cos(), a.sin()), Pt::new(-a.sin(), a.cos()));
    let corners = [Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(0.0, h), Pt::new(w, h)];
    let proj = |d: Pt| {
        let v: Vec<f64> = corners.iter().map(|c| c.x * d.x + c.y * d.y).collect();
        (v.iter().cloned().fold(f64::MAX, f64::min), v.iter().cloned().fold(f64::MIN, f64::max))
    };
    let (u0, u1) = proj(u);
    let (n0, n1) = proj(n);
    let count = ((n1 - n0) / gap).floor() as usize;
    let off = ((n1 - n0) - count as f64 * gap) / 2.0;
    (0..=count)
        .map(|i| {
            let t = n0 + off + i as f64 * gap;
            (Pt::new(u.x * u0 + n.x * t, u.y * u0 + n.y * t), u, n, u1 - u0)
        })
        .collect()
}

fn add(a: Pt, b: Pt, k: f64) -> Pt {
    Pt::new(a.x + b.x * k, a.y + b.y * k)
}

fn circle(c: Pt, r: f64, start: f64) -> Vec<Pt> {
    let n = ((r * 8.0).ceil() as usize).clamp(8, 96);
    (0..=n).map(|i| {
        let t = start + TAU * i as f64 / n as f64;
        Pt::new(c.x + r * t.cos(), c.y + r * t.sin())
    }).collect()
}

/// A dot a pen, a laser or a bit can make: a small closed circle (a single point would be lost on the way to the G-code).
fn dot(c: Pt, r: f64) -> Vec<Pt> {
    let r = r.max(0.05);
    let n = ((r * 6.0).ceil() as usize).clamp(6, 32);
    (0..=n).map(|i| {
        let t = TAU * i as f64 / n as f64;
        Pt::new(c.x + r * t.cos(), c.y + r * t.sin())
    }).collect()
}

// ---------------------------------------------------------------------------------------------------------------------------
// The styles
// ---------------------------------------------------------------------------------------------------------------------------

/// Squiggle, sawtooth and springs: rows whose wave grows with the darkness.
fn rows_wave(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Field::new(im, p, (p.spacing / 4.0).max(0.1), 1_500_000);
    let gap = p.spacing;
    let amp = p.strength * gap / 2.0;
    let freq = p.detail;
    let ds = (gap / 12.0).min(1.0 / (freq * 24.0)).max(0.02);
    let lift = p.variant == 1;
    for (start, u, n, len) in rows(f.w, f.h, p.angle, gap) {
        if !out.room() {
            return;
        }
        let mut pen = Pen::new();
        let mut phase = 0f64;
        let steps = (len / ds).ceil() as usize;
        for i in 0..=steps {
            let base = add(start, u, i as f64 * ds);
            let d = f.at(base);
            phase += TAU * freq * d * ds;
            let a = amp * d;
            let q = match p.style {
                Style::Sawtooth => add(base, n, a * tri(phase)),
                Style::Springs => add(add(base, u, a * phase.cos()), n, a * phase.sin()),
                _ => add(base, n, a * phase.sin()),
            };
            let draw = f.inside(base) && f.inside(q) && !(lift && d < 0.02);
            pen.to(out, q, draw);
        }
        pen.lift(out);
    }
}

/// A triangle wave of the phase, -1..1.
fn tri(phase: f64) -> f64 {
    let t = (phase / TAU).rem_euclid(1.0);
    if t < 0.25 {
        t * 4.0
    } else if t < 0.75 {
        2.0 - t * 4.0
    } else {
        t * 4.0 - 4.0
    }
}

/// One spiral from the middle, its wobble growing with the darkness.
fn spiral(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Field::new(im, p, (p.spacing / 4.0).max(0.1), 1_500_000);
    let c = Pt::new(f.w / 2.0, f.h / 2.0);
    let rmax = (f.w * f.w + f.h * f.h).sqrt() / 2.0 + p.spacing;
    let gap = p.spacing;
    let amp = p.strength * gap / 2.0;
    let ds = (gap / 12.0).min(1.0 / (p.detail * 24.0)).max(0.02);
    let mut pen = Pen::new();
    let (mut th, mut phase) = (0f64, 0f64);
    loop {
        let r = gap * th / TAU;
        if r > rmax || !out.room() {
            break;
        }
        let dir = Pt::new(th.cos(), th.sin());
        let base = add(c, dir, r);
        let d = f.at(base);
        phase += TAU * p.detail * d * ds;
        let wob = if p.variant == 1 { tri(phase) } else { phase.sin() };
        let q = add(base, dir, amp * d * wob);
        pen.to(out, q, f.inside(base) && f.inside(q));
        th += ds / r.max(gap);
    }
    pen.lift(out);
}

/// Rows lifted by the darkness; the rows in front hide what is behind them.
fn waves(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Field::new(im, p, (p.spacing / 3.0).max(0.1), 1_500_000);
    let gap = p.spacing;
    let lift = p.strength * gap;
    let dx = (gap / 4.0).clamp(0.05, 1.0);
    let n = (f.w / dx).ceil() as usize + 1;
    let mut horizon = vec![f64::MAX; n];
    let hide = p.variant == 0;
    let count = (f.h / gap).floor() as usize;
    for r in 0..=count {
        let y = f.h - (r as f64 + 0.5) * gap;
        if y < 0.0 || !out.room() {
            break;
        }
        let mut pen = Pen::new();
        for i in 0..n {
            let x = (i as f64 * dx).min(f.w);
            let yy = y - lift * f.at(Pt::new(x, y));
            let seen = !hide || yy < horizon[i] - 1e-6;
            if hide {
                horizon[i] = horizon[i].min(yy);
            }
            pen.to(out, Pt::new(x, yy.max(0.0)), seen);
        }
        pen.lift(out);
    }
}

/// The centres of a grid of square cells turned by the angle, covering the picture.
fn grid_cells(w: f64, h: f64, cell: f64, angle: f64) -> Vec<Pt> {
    let mut v = vec![];
    for (start, u, _, len) in rows(w, h, angle, cell) {
        let n = (len / cell).ceil() as usize;
        for i in 0..n {
            let c = add(start, u, (i as f64 + 0.5) * cell);
            if c.x >= -cell && c.y >= -cell && c.x <= w + cell && c.y <= h + cell {
                v.push(c);
            }
        }
    }
    v
}

fn inside_rect(w: f64, h: f64, pts: &[Pt]) -> bool {
    pts.iter().all(|q| q.x >= 0.0 && q.y >= 0.0 && q.x <= w && q.y <= h)
}

/// Circles on a grid: the area of a circle follows the darkness.
fn halftone(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Sat::new(Field::new(im, p, (p.spacing / 6.0).max(0.1), 1_500_000));
    let cell = p.spacing;
    let ring = p.detail;
    for c in grid_cells(f.f.w, f.f.h, cell, p.angle) {
        if !out.room() {
            return;
        }
        let d = f.mean(c.x - cell / 2.0, c.y - cell / 2.0, c.x + cell / 2.0, c.y + cell / 2.0);
        let r = 0.5 * cell * p.strength * d.sqrt();
        if r < 0.06 || !f.f.inside(c) {
            continue;
        }
        match p.variant {
            1 => {
                let mut rr = r;
                while rr > 0.05 {
                    let pts = circle(c, rr, 0.0);
                    if inside_rect(f.f.w, f.f.h, &pts) {
                        out.push(pts, true);
                    }
                    rr -= ring;
                }
            }
            2 => {
                let mut pts = vec![];
                let turns = (r / ring).max(1.0);
                let n = ((turns * 24.0) as usize).clamp(12, 2000);
                for i in 0..=n {
                    let t = i as f64 / n as f64;
                    let a = t * turns * TAU;
                    let q = Pt::new(c.x + r * t * a.cos(), c.y + r * t * a.sin());
                    pts.push(q);
                }
                if inside_rect(f.f.w, f.f.h, &pts) {
                    out.push(pts, false);
                }
            }
            _ => {
                let pts = circle(c, r, 0.0);
                if inside_rect(f.f.w, f.f.h, &pts) {
                    out.push(pts, true);
                }
            }
        }
    }
}

/// Hatching in layers: each darker tone adds lines in another direction.
fn etching(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Field::new(im, p, (p.spacing / 3.0).max(0.1), 1_500_000).blurred(p.spacing / 2.0);
    let levels = p.detail.round().clamp(1.0, 6.0) as usize;
    let gap = p.spacing;
    let ds = (gap / 4.0).clamp(0.05, 1.0);
    let turns = [0.0, 90.0, 45.0, 135.0, 0.0, 90.0];
    for k in 0..levels {
        let t = (k as f64 + 1.0) / (levels as f64 + 1.0);
        let shift = if k >= 4 { 0.5 } else { 0.0 };
        for (start, u, n, len) in rows(f.w, f.h, p.angle + turns[k], gap) {
            if !out.room() {
                return;
            }
            let start = add(start, n, shift * gap);
            let mut pen = Pen::new();
            let steps = (len / ds).ceil() as usize;
            for i in 0..=steps {
                let mut q = add(start, u, i as f64 * ds);
                let d = f.at(q);
                if p.variant == 1 {
                    q = add(q, n, gap * 0.25 * (i as f64 * ds / (gap * 3.0) * TAU).sin());
                }
                pen.to(out, q, f.inside(q) && d > t);
            }
            pen.lift(out);
        }
    }
    let min = gap;
    out.lines.retain(|l| l.length() >= min);
}

/// Squares one inside the other on a grid, more of them where it is darker.
fn squares(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Sat::new(Field::new(im, p, (p.spacing / 6.0).max(0.1), 1_500_000));
    let cell = p.spacing;
    let most = p.detail.round().clamp(1.0, 12.0);
    let mut rng = Rng::new(p.seed, 3);
    let base = p.angle.to_radians();
    for c in grid_cells(f.f.w, f.f.h, cell, p.angle) {
        if !out.room() {
            return;
        }
        let shake = rng.range(-1.0, 1.0) * p.strength * PI / 4.0;
        let d = f.mean(c.x - cell / 2.0, c.y - cell / 2.0, c.x + cell / 2.0, c.y + cell / 2.0);
        let n = (d * most).round() as usize;
        for i in 0..n {
            let half = cell / 2.0 * (1.0 - i as f64 / most) * 0.95;
            if half < 0.05 {
                break;
            }
            let a = base + shake;
            let (ca, sa) = (a.cos(), a.sin());
            let pts: Vec<Pt> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0)]
                .iter()
                .map(|(x, y)| Pt::new(c.x + half * (x * ca - y * sa), c.y + half * (x * sa + y * ca)))
                .collect();
            if inside_rect(f.f.w, f.f.h, &pts) {
                out.push(pts, true);
            }
        }
    }
}

/// Short strokes on a grid, longer where darker, along the shapes, at an angle or at random.
fn needles(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let raw = Field::new(im, p, (p.spacing / 6.0).max(0.1), 1_500_000);
    let smooth = raw.blurred(p.spacing);
    let f = Sat::new(raw);
    let cell = p.spacing;
    let mut rng = Rng::new(p.seed, 5);
    let fixed = p.angle.to_radians();
    for c in grid_cells(f.f.w, f.f.h, cell, 0.0) {
        if !out.room() {
            return;
        }
        let d = f.mean(c.x - cell / 2.0, c.y - cell / 2.0, c.x + cell / 2.0, c.y + cell / 2.0);
        let len = cell * p.strength * d;
        if len < 0.1 || !f.f.inside(c) {
            continue;
        }
        let a = match p.variant {
            0 => {
                let g = smooth.grad(c);
                if g.x.hypot(g.y) < 1e-4 {
                    fixed
                } else {
                    g.y.atan2(g.x) + PI / 2.0
                }
            }
            1 => fixed,
            _ => rng.range(0.0, PI),
        };
        let u = Pt::new(a.cos(), a.sin());
        let (a0, a1) = (add(c, u, -len / 2.0), add(c, u, len / 2.0));
        if f.f.inside(a0) && f.f.inside(a1) {
            out.push(vec![a0, a1], false);
        }
    }
}

/// Tiles of quarter circles turned at random; darker tiles have more arcs.
fn truchet(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Sat::new(Field::new(im, p, (p.spacing / 6.0).max(0.1), 1_500_000));
    let s = p.spacing;
    let most = p.detail.round().clamp(1.0, 10.0);
    let mut rng = Rng::new(p.seed, 7);
    let (nx, ny) = ((f.f.w / s).floor() as usize, (f.f.h / s).floor() as usize);
    let (ox, oy) = ((f.f.w - nx as f64 * s) / 2.0, (f.f.h - ny as f64 * s) / 2.0);
    for j in 0..ny {
        for i in 0..nx {
            if !out.room() {
                return;
            }
            let (x0, y0) = (ox + i as f64 * s, oy + j as f64 * s);
            let flip = rng.f() < 0.5;
            let d = f.mean(x0, y0, x0 + s, y0 + s);
            let n = (d * most).round() as usize;
            // Two corners opposite each other carry the arcs: (0, 0) and (s, s), or (s, 0) and (0, s).
            let corners: [(Pt, f64); 2] = if flip { [(Pt::new(x0, y0), 0.0), (Pt::new(x0 + s, y0 + s), PI)] } else { [(Pt::new(x0 + s, y0), PI / 2.0), (Pt::new(x0, y0 + s), -PI / 2.0)] };
            for (c, a0) in corners {
                for m in 1..=n {
                    let r = s * m as f64 / (n as f64 + 1.0);
                    let steps = ((r * 3.0).ceil() as usize).clamp(4, 48);
                    let pts: Vec<Pt> = (0..=steps).map(|t| {
                        let a = a0 + PI / 2.0 * t as f64 / steps as f64;
                        Pt::new(c.x + r * a.cos(), c.y + r * a.sin())
                    }).collect();
                    out.push(pts, false);
                }
            }
        }
    }
}

/// Lines of equal darkness (marching squares), joined into long lines.
fn contours(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Field::new(im, p, (p.spacing / 2.0).max(0.1), 1_000_000).blurred(p.spacing);
    let levels = p.detail.round().clamp(2.0, 40.0) as usize;
    for k in 1..=levels {
        if !out.room() {
            return;
        }
        let t = k as f32 / (levels as f32 + 1.0);
        for pts in iso_lines(&f, t) {
            out.push(pts, false);
        }
    }
}

/// The lines where the field crosses `t`, in millimetres. The edges of the picture count as white, so dark areas close.
fn iso_lines(f: &Field, t: f32) -> Vec<Vec<Pt>> {
    // A grid one cell bigger on every side, the border white.
    let (nx, ny) = (f.nx + 2, f.ny + 2);
    let val = |x: usize, y: usize| -> f32 {
        if x == 0 || y == 0 || x == nx - 1 || y == ny - 1 {
            0.0
        } else {
            f.v[(y - 1) * f.nx + (x - 1)]
        }
    };
    let pos = |x: f64, y: f64| Pt::new(((x - 0.5) * f.cell).clamp(0.0, f.w), ((y - 0.5) * f.cell).clamp(0.0, f.h));
    // Edge ids: horizontal edge (x, y)-(x+1, y) = 2 * (y * nx + x), vertical (x, y)-(x, y+1) = 2 * (y * nx + x) + 1.
    let point_on = |e: usize| -> Pt {
        let (cell, vert) = (e / 2, e % 2 == 1);
        let (x, y) = (cell % nx, cell / nx);
        let (a, b) = if vert { (val(x, y), val(x, y + 1)) } else { (val(x, y), val(x + 1, y)) };
        let s = if (b - a).abs() < 1e-9 { 0.5 } else { ((t - a) / (b - a)).clamp(0.0, 1.0) } as f64;
        if vert { pos(x as f64, y as f64 + s) } else { pos(x as f64 + s, y as f64) }
    };
    let mut segs: Vec<(usize, usize)> = vec![];
    for y in 0..ny - 1 {
        for x in 0..nx - 1 {
            let (a, b, c, d) = (val(x, y) > t, val(x + 1, y) > t, val(x + 1, y + 1) > t, val(x, y + 1) > t);
            let top = 2 * (y * nx + x);
            let bottom = 2 * ((y + 1) * nx + x);
            let left = 2 * (y * nx + x) + 1;
            let right = 2 * (y * nx + x + 1) + 1;
            let case = (a as u8) | (b as u8) << 1 | (c as u8) << 2 | (d as u8) << 3;
            let mut s = |p: usize, q: usize| segs.push((p, q));
            match case {
                1 | 14 => s(left, top),
                2 | 13 => s(top, right),
                3 | 12 => s(left, right),
                4 | 11 => s(right, bottom),
                6 | 9 => s(top, bottom),
                7 | 8 => s(left, bottom),
                5 => {
                    s(left, top);
                    s(right, bottom);
                }
                10 => {
                    s(top, right);
                    s(left, bottom);
                }
                _ => {}
            }
        }
    }
    chain(&segs).into_iter().map(|ids| ids.into_iter().map(point_on).collect()).collect()
}

/// Join segments that share ends into long chains (each end is shared by at most two segments).
fn chain(segs: &[(usize, usize)]) -> Vec<Vec<usize>> {
    use std::collections::HashMap;
    let mut at: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        at.entry(*a).or_default().push(i);
        at.entry(*b).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut out = vec![];
    let walk = |start_seg: usize, from: usize, used: &mut Vec<bool>, line: &mut Vec<usize>| {
        let (mut s, mut end) = (start_seg, from);
        loop {
            used[s] = true;
            let (a, b) = segs[s];
            let next_end = if a == end { b } else { a };
            line.push(next_end);
            end = next_end;
            match at[&end].iter().find(|&&o| !used[o]) {
                Some(&o) => s = o,
                None => break,
            }
        }
    };
    // Open chains first (from an end used once), then the loops.
    let mut starts: Vec<usize> = (0..segs.len()).collect();
    starts.sort_by_key(|&i| !(at[&segs[i].0].len() == 1 || at[&segs[i].1].len() == 1));
    for i in starts {
        if used[i] {
            continue;
        }
        let (a, b) = segs[i];
        let from = if at[&a].len() == 1 { a } else if at[&b].len() == 1 { b } else { a };
        let mut line = vec![from];
        walk(i, from, &mut used, &mut line);
        out.push(line);
    }
    out
}

/// One Hilbert curve whose cells are smaller where the picture is darker.
fn hilbert(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Sat::new(Field::new(im, p, (p.spacing / 2.0).max(0.1), 1_500_000));
    let smallest = p.spacing;
    let levels = p.detail.round().clamp(1.0, 6.0);
    let side = f.f.w.max(f.f.h);
    // The deepest level whose cells are not smaller than the smallest step.
    let max_depth = ((side / smallest).log2().floor() as i32).clamp(1, 14) as u32;
    let mut pts = vec![];
    let target = |d: f64| smallest * 2f64.powf((1.0 - d) * levels);
    hil(&f, Pt::new(0.0, 0.0), Pt::new(side, 0.0), Pt::new(0.0, side), 0, max_depth, &target, &mut pts, MAX_POINTS);
    let lift = p.variant == 1;
    let mut pen = Pen::new();
    for q in pts {
        let draw = f.f.inside(q) && !(lift && f.f.at(q) < 0.03);
        pen.to(out, q, draw);
    }
    pen.lift(out);
}

#[allow(clippy::too_many_arguments)]
fn hil(f: &Sat, o: Pt, x: Pt, y: Pt, depth: u32, max_depth: u32, target: &dyn Fn(f64) -> f64, pts: &mut Vec<Pt>, max: usize) {
    if pts.len() >= max {
        return;
    }
    let size = x.x.hypot(x.y);
    let c = Pt::new(o.x + (x.x + y.x) / 2.0, o.y + (x.y + y.y) / 2.0);
    let (lo, hi) = (Pt::new(o.x.min(o.x + x.x + y.x), o.y.min(o.y + x.y + y.y)), Pt::new(o.x.max(o.x + x.x + y.x), o.y.max(o.y + x.y + y.y)));
    let outside = lo.x >= f.f.w || lo.y >= f.f.h;
    let d = if outside { 0.0 } else { f.mean(lo.x, lo.y, hi.x.min(f.f.w), hi.y.min(f.f.h)) };
    if depth >= max_depth || outside || size <= target(d) {
        pts.push(c);
        return;
    }
    let (hx, hy) = (Pt::new(x.x / 2.0, x.y / 2.0), Pt::new(y.x / 2.0, y.y / 2.0));
    hil(f, o, hy, hx, depth + 1, max_depth, target, pts, max);
    hil(f, Pt::new(o.x + hx.x, o.y + hx.y), hx, hy, depth + 1, max_depth, target, pts, max);
    hil(f, Pt::new(o.x + hx.x + hy.x, o.y + hx.y + hy.y), hx, hy, depth + 1, max_depth, target, pts, max);
    hil(f, Pt::new(o.x + hx.x + y.x, o.y + hx.y + y.y), Pt::new(-hy.x, -hy.y), Pt::new(-hx.x, -hx.y), depth + 1, max_depth, target, pts, max);
}

/// Points spread by the darkness and moved to the weighted middles of their cells (weighted Voronoi stippling, Secord 2002).
fn stipple_points(im: &ImageData, p: &StyleParams, n: usize, iterations: usize, stop: &dyn Fn() -> bool) -> (Vec<Pt>, Field) {
    let f = Field::new(im, p, 0.0, 250_000);
    let mut rng = Rng::new(p.seed, 11);
    let total: f64 = f.v.iter().map(|v| *v as f64).sum();
    if total < 1e-6 {
        return (vec![], f);
    }
    // Rejection sampling by the darkness.
    let mut pts = Vec::with_capacity(n);
    let mut tries = 0usize;
    while pts.len() < n && tries < n * 400 {
        tries += 1;
        let q = Pt::new(rng.f() * f.w, rng.f() * f.h);
        if rng.f() < f.at(q) {
            pts.push(q);
        }
    }
    for _ in 0..iterations {
        if stop() || pts.is_empty() {
            break;
        }
        let grid = PointGrid::new(&pts, f.w, f.h);
        let mut acc = vec![(0f64, 0f64, 0f64); pts.len()];
        for cy in 0..f.ny {
            for cx in 0..f.nx {
                let w = f.v[cy * f.nx + cx] as f64;
                if w <= 1e-4 {
                    continue;
                }
                let q = Pt::new((cx as f64 + 0.5) * f.cell, (cy as f64 + 0.5) * f.cell);
                if let Some(i) = grid.nearest(&pts, q) {
                    acc[i].0 += w * q.x;
                    acc[i].1 += w * q.y;
                    acc[i].2 += w;
                }
            }
        }
        for (i, a) in acc.iter().enumerate() {
            if a.2 > 0.0 {
                pts[i] = Pt::new(a.0 / a.2, a.1 / a.2);
            }
        }
    }
    (pts, f)
}

/// Points in buckets, to find the nearest one quickly.
struct PointGrid {
    cell: f64,
    nx: usize,
    ny: usize,
    buckets: Vec<Vec<u32>>,
}

impl PointGrid {
    fn new(pts: &[Pt], w: f64, h: f64) -> PointGrid {
        let cell = ((w * h / pts.len().max(1) as f64).sqrt() * 1.5).max(1e-3);
        let nx = ((w / cell).ceil() as usize).max(1);
        let ny = ((h / cell).ceil() as usize).max(1);
        let mut buckets = vec![vec![]; nx * ny];
        for (i, q) in pts.iter().enumerate() {
            let (x, y) = (((q.x / cell) as usize).min(nx - 1), ((q.y / cell) as usize).min(ny - 1));
            buckets[y * nx + x].push(i as u32);
        }
        PointGrid { cell, nx, ny, buckets }
    }

    fn nearest(&self, pts: &[Pt], q: Pt) -> Option<usize> {
        let (cx, cy) = (((q.x / self.cell).max(0.0) as usize).min(self.nx - 1), ((q.y / self.cell).max(0.0) as usize).min(self.ny - 1));
        let mut best: Option<(usize, f64)> = None;
        for r in 0..self.nx.max(self.ny) {
            // Every point in ring r is at least (r - 1) cells away.
            if let Some((_, d)) = best {
                if (r as f64 - 1.0) * self.cell > d.sqrt() {
                    break;
                }
            }
            let (x0, x1) = (cx.saturating_sub(r), (cx + r).min(self.nx - 1));
            let (y0, y1) = (cy.saturating_sub(r), (cy + r).min(self.ny - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if x != x0 && x != x1 && y != y0 && y != y1 {
                        continue;
                    }
                    if (x as isize - cx as isize).unsigned_abs() != r && (y as isize - cy as isize).unsigned_abs() != r {
                        continue;
                    }
                    for &i in &self.buckets[y * self.nx + x] {
                        let p = pts[i as usize];
                        let d = (p.x - q.x).powi(2) + (p.y - q.y).powi(2);
                        if best.is_none_or(|(_, bd)| d < bd) {
                            best = Some((i as usize, d));
                        }
                    }
                }
            }
        }
        best.map(|(i, _)| i)
    }
}

fn stipple(im: &ImageData, p: &StyleParams, out: &mut Out, stop: &dyn Fn() -> bool) {
    let n = p.detail.round() as usize;
    let (pts, f) = stipple_points(im, p, n, 14, stop);
    let size = p.strength;
    for q in pts {
        if !out.room() {
            return;
        }
        let d = f.at(q);
        // The whole mark inside the picture.
        let m = size.min(f.w / 2.0).min(f.h / 2.0);
        let q = Pt::new(q.x.clamp(m, f.w - m), q.y.clamp(m, f.h - m));
        match p.variant {
            1 => {
                let r = size * d;
                if r >= 0.05 {
                    out.push(dot(q, r), true);
                }
            }
            2 => {
                let g = f.blurred(size).grad(q);
                let a = if g.x.hypot(g.y) < 1e-6 { 0.0 } else { g.y.atan2(g.x) + PI / 2.0 };
                let u = Pt::new(a.cos(), a.sin());
                out.push(vec![add(q, u, -size), add(q, u, size)], false);
            }
            3 => {
                let r = size;
                let pts: Vec<Pt> = (0..=3).map(|i| {
                    let a = -PI / 2.0 + TAU * i as f64 / 3.0;
                    Pt::new(q.x + r * a.cos(), q.y + r * a.sin())
                }).collect();
                out.push(pts, true);
            }
            _ => out.push(dot(q, size / 2.0), true),
        }
    }
}

/// The stippling joined into one tour: nearest neighbour, then 2-opt with neighbour lists.
fn tsp(im: &ImageData, p: &StyleParams, out: &mut Out, stop: &dyn Fn() -> bool) {
    let n = p.detail.round() as usize;
    let (pts, _) = stipple_points(im, p, n, 12, stop);
    if pts.len() < 2 {
        return;
    }
    let tour = tour_nearest(&pts);
    let tour = two_opt(&pts, tour, stop);
    out.push(tour.into_iter().map(|i| pts[i]).collect(), false);
}

fn tour_nearest(pts: &[Pt]) -> Vec<usize> {
    let n = pts.len();
    let mut left: Vec<usize> = (0..n).collect();
    // Start at the point most to the left.
    let start = (0..n).min_by(|&a, &b| pts[a].x.total_cmp(&pts[b].x)).unwrap_or(0);
    let mut alive = vec![true; n];
    let mut grid = MutGrid::new(pts);
    let mut tour = Vec::with_capacity(n);
    let mut cur = start;
    alive[cur] = false;
    grid.remove(pts, cur);
    tour.push(cur);
    while tour.len() < n {
        let next = grid.nearest(pts, pts[cur]).unwrap_or_else(|| {
            left.retain(|&i| alive[i]);
            left[0]
        });
        alive[next] = false;
        grid.remove(pts, next);
        tour.push(next);
        cur = next;
    }
    tour
}

/// A grid of buckets whose points can be taken out.
struct MutGrid {
    cell: f64,
    nx: usize,
    ny: usize,
    x0: f64,
    y0: f64,
    buckets: Vec<Vec<u32>>,
    count: usize,
}

impl MutGrid {
    fn new(pts: &[Pt]) -> MutGrid {
        let x0 = pts.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        let y0 = pts.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        let x1 = pts.iter().map(|p| p.x).fold(f64::MIN, f64::max);
        let y1 = pts.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        let (w, h) = ((x1 - x0).max(1e-3), (y1 - y0).max(1e-3));
        let cell = ((w * h / pts.len().max(1) as f64).sqrt() * 1.5).max(1e-3);
        let nx = ((w / cell).ceil() as usize).max(1);
        let ny = ((h / cell).ceil() as usize).max(1);
        let mut g = MutGrid { cell, nx, ny, x0, y0, buckets: vec![vec![]; nx * ny], count: pts.len() };
        for (i, q) in pts.iter().enumerate() {
            let b = g.bucket(*q);
            g.buckets[b].push(i as u32);
        }
        g
    }

    fn xy(&self, q: Pt) -> (usize, usize) {
        ((((q.x - self.x0) / self.cell).max(0.0) as usize).min(self.nx - 1), (((q.y - self.y0) / self.cell).max(0.0) as usize).min(self.ny - 1))
    }

    fn bucket(&self, q: Pt) -> usize {
        let (x, y) = self.xy(q);
        y * self.nx + x
    }

    fn remove(&mut self, pts: &[Pt], i: usize) {
        let b = self.bucket(pts[i]);
        if let Some(k) = self.buckets[b].iter().position(|&j| j as usize == i) {
            self.buckets[b].swap_remove(k);
            self.count -= 1;
        }
    }

    fn nearest(&self, pts: &[Pt], q: Pt) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        let (cx, cy) = self.xy(q);
        let mut best: Option<(usize, f64)> = None;
        for r in 0..=self.nx.max(self.ny) {
            if let Some((_, d)) = best {
                if (r as f64 - 1.0) * self.cell > d.sqrt() {
                    break;
                }
            }
            let (x0, x1) = (cx.saturating_sub(r), (cx + r).min(self.nx - 1));
            let (y0, y1) = (cy.saturating_sub(r), (cy + r).min(self.ny - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if (x as isize - cx as isize).unsigned_abs() != r && (y as isize - cy as isize).unsigned_abs() != r {
                        continue;
                    }
                    for &i in &self.buckets[y * self.nx + x] {
                        let p = pts[i as usize];
                        let d = (p.x - q.x).powi(2) + (p.y - q.y).powi(2);
                        if best.is_none_or(|(_, bd)| d < bd) {
                            best = Some((i as usize, d));
                        }
                    }
                }
            }
        }
        best.map(|(i, _)| i)
    }
}

/// 2-opt on an open tour, each point trying only its nearest neighbours, until nothing improves (or `stop`).
fn two_opt(pts: &[Pt], mut tour: Vec<usize>, stop: &dyn Fn() -> bool) -> Vec<usize> {
    let n = tour.len();
    if n < 4 {
        return tour;
    }
    const K: usize = 8;
    // Neighbour lists from a bucket grid.
    let grid = PointGrid::new(pts, pts.iter().map(|p| p.x).fold(0.0, f64::max) + 1e-3, pts.iter().map(|p| p.y).fold(0.0, f64::max) + 1e-3);
    let neigh: Vec<Vec<usize>> = (0..pts.len())
        .map(|i| {
            let q = pts[i];
            let (cx, cy) = (((q.x / grid.cell) as usize).min(grid.nx - 1), ((q.y / grid.cell) as usize).min(grid.ny - 1));
            let mut cand: Vec<(f64, usize)> = vec![];
            for y in cy.saturating_sub(1)..=(cy + 1).min(grid.ny - 1) {
                for x in cx.saturating_sub(1)..=(cx + 1).min(grid.nx - 1) {
                    for &j in &grid.buckets[y * grid.nx + x] {
                        let j = j as usize;
                        if j != i {
                            cand.push(((pts[j].x - q.x).powi(2) + (pts[j].y - q.y).powi(2), j));
                        }
                    }
                }
            }
            cand.sort_by(|a, b| a.0.total_cmp(&b.0));
            cand.into_iter().take(K).map(|c| c.1).collect()
        })
        .collect();
    let mut pos = vec![0usize; pts.len()];
    let d = |a: usize, b: usize| pts[a].dist(pts[b]);
    let mut improved = true;
    let mut rounds = 0;
    while improved && rounds < 50 {
        if stop() {
            break;
        }
        improved = false;
        rounds += 1;
        for (k, &c) in tour.iter().enumerate() {
            pos[c] = k;
        }
        for i in 0..n - 1 {
            let (a, b) = (tour[i], tour[i + 1]);
            let dab = d(a, b);
            for &c in &neigh[a] {
                let j = pos[c];
                if j <= i + 1 || j >= n {
                    continue;
                }
                // Reverse tour[i+1..=j]: edges (a,b) and (c,next) become (a,c) and (b,next).
                let next = if j + 1 < n { Some(tour[j + 1]) } else { None };
                let before = dab + next.map_or(0.0, |e| d(c, e));
                let after = d(a, c) + next.map_or(0.0, |e| d(b, e));
                if after + 1e-9 < before {
                    tour[i + 1..=j].reverse();
                    for (k, &t) in tour.iter().enumerate().take(j + 1).skip(i + 1) {
                        pos[t] = k;
                    }
                    improved = true;
                    break;
                }
            }
        }
    }
    tour
}

/// The Delaunay triangles of stippled points, or the Voronoi cells round them.
fn delaunay(im: &ImageData, p: &StyleParams, out: &mut Out, stop: &dyn Fn() -> bool) {
    let n = p.detail.round() as usize;
    let (pts, f) = stipple_points(im, p, n, 8, stop);
    if pts.len() < 3 || stop() {
        return;
    }
    let tris = triangulate(&pts);
    let mut edges: std::collections::HashMap<(usize, usize), Vec<usize>> = std::collections::HashMap::new();
    for (t, tr) in tris.iter().enumerate() {
        for (a, b) in [(tr[0], tr[1]), (tr[1], tr[2]), (tr[2], tr[0])] {
            edges.entry((a.min(b), a.max(b))).or_default().push(t);
        }
    }
    let mut keys: Vec<_> = edges.keys().cloned().collect();
    keys.sort();
    if p.variant == 1 {
        let centre = |t: usize| circumcentre(pts[tris[t][0]], pts[tris[t][1]], pts[tris[t][2]]);
        for k in keys {
            let ts = &edges[&k];
            if ts.len() == 2 {
                if let (Some(a), Some(b)) = (centre(ts[0]), centre(ts[1])) {
                    if let Some((a, b)) = clip_segment(a, b, f.w, f.h) {
                        out.push(vec![a, b], false);
                    }
                }
            }
        }
    } else {
        for (a, b) in keys {
            out.push(vec![pts[a], pts[b]], false);
        }
    }
}

fn circumcentre(a: Pt, b: Pt, c: Pt) -> Option<Pt> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        return None;
    }
    let (a2, b2, c2) = (a.x * a.x + a.y * a.y, b.x * b.x + b.y * b.y, c.x * c.x + c.y * c.y);
    Some(Pt::new((a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d, (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d))
}

/// Liang–Barsky clipping of a segment to the rectangle (0, 0)–(w, h).
fn clip_segment(a: Pt, b: Pt, w: f64, h: f64) -> Option<(Pt, Pt)> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut t0, mut t1) = (0f64, 1f64);
    for (p, q) in [(-dx, a.x), (dx, w - a.x), (-dy, a.y), (dy, h - a.y)] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    if t0 > t1 {
        return None;
    }
    Some((Pt::new(a.x + dx * t0, a.y + dy * t0), Pt::new(a.x + dx * t1, a.y + dy * t1)))
}

/// Bowyer–Watson triangulation; triangles as indices into `pts`.
fn triangulate(pts: &[Pt]) -> Vec<[usize; 3]> {
    let n = pts.len();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for q in pts {
        x0 = x0.min(q.x);
        y0 = y0.min(q.y);
        x1 = x1.max(q.x);
        y1 = y1.max(q.y);
    }
    let m = (x1 - x0).max(y1 - y0).max(1.0) * 20.0;
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let mut all = pts.to_vec();
    all.push(Pt::new(cx - m, cy - m));
    all.push(Pt::new(cx + m, cy - m));
    all.push(Pt::new(cx, cy + m));
    struct T {
        v: [usize; 3],
        c: Pt,
        r2: f64,
    }
    let mk = |v: [usize; 3], all: &[Pt]| -> Option<T> {
        let c = circumcentre(all[v[0]], all[v[1]], all[v[2]])?;
        let r2 = (all[v[0]].x - c.x).powi(2) + (all[v[0]].y - c.y).powi(2);
        Some(T { v, c, r2 })
    };
    let mut tris: Vec<T> = mk([n, n + 1, n + 2], &all).into_iter().collect();
    // Insert in a spatial order so the search for bad triangles stays local in practice.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| (pts[a].x + pts[a].y).total_cmp(&(pts[b].x + pts[b].y)));
    for i in order {
        let q = all[i];
        let mut poly: Vec<(usize, usize)> = vec![];
        let mut keep = Vec::with_capacity(tris.len());
        for t in tris.drain(..) {
            if (q.x - t.c.x).powi(2) + (q.y - t.c.y).powi(2) < t.r2 {
                for (a, b) in [(t.v[0], t.v[1]), (t.v[1], t.v[2]), (t.v[2], t.v[0])] {
                    if let Some(k) = poly.iter().position(|&(c, d)| (c == b && d == a) || (c == a && d == b)) {
                        poly.swap_remove(k);
                    } else {
                        poly.push((a, b));
                    }
                }
            } else {
                keep.push(t);
            }
        }
        tris = keep;
        for (a, b) in poly {
            if let Some(t) = mk([a, b, i], &all) {
                tris.push(t);
            }
        }
    }
    tris.into_iter().filter(|t| t.v.iter().all(|&v| v < n)).map(|t| t.v).collect()
}

/// The edges of the picture (Canny: gradient, thinning, two thresholds), traced into lines.
fn line_art(im: &ImageData, p: &StyleParams, out: &mut Out) {
    let f = Field::new(im, p, p.spacing, 1_000_000).blurred(p.spacing);
    let (nx, ny) = (f.nx, f.ny);
    if nx < 3 || ny < 3 {
        return;
    }
    let g = |x: usize, y: usize| f.v[y * nx + x];
    let mut mag = vec![0f32; nx * ny];
    let mut dir = vec![0u8; nx * ny];
    for y in 1..ny - 1 {
        for x in 1..nx - 1 {
            let gx = (g(x + 1, y - 1) + 2.0 * g(x + 1, y) + g(x + 1, y + 1)) - (g(x - 1, y - 1) + 2.0 * g(x - 1, y) + g(x - 1, y + 1));
            let gy = (g(x - 1, y + 1) + 2.0 * g(x, y + 1) + g(x + 1, y + 1)) - (g(x - 1, y - 1) + 2.0 * g(x, y - 1) + g(x + 1, y - 1));
            mag[y * nx + x] = gx.hypot(gy);
            let a = (gy.atan2(gx).to_degrees() + 180.0) % 180.0;
            dir[y * nx + x] = if !(22.5..157.5).contains(&a) { 0 } else if a < 67.5 { 1 } else if a < 112.5 { 2 } else { 3 };
        }
    }
    let maxm = mag.iter().cloned().fold(0f32, f32::max);
    if maxm <= 1e-6 {
        return;
    }
    let high = maxm * (0.04 + 0.5 * (1.0 - p.strength as f32));
    let low = high * 0.4;
    let mut thin = vec![false; nx * ny];
    for y in 1..ny - 1 {
        for x in 1..nx - 1 {
            let m = mag[y * nx + x];
            if m < low {
                continue;
            }
            let (a, b) = match dir[y * nx + x] {
                0 => (mag[y * nx + x - 1], mag[y * nx + x + 1]),
                1 => (mag[(y - 1) * nx + x - 1], mag[(y + 1) * nx + x + 1]),
                2 => (mag[(y - 1) * nx + x], mag[(y + 1) * nx + x]),
                _ => (mag[(y - 1) * nx + x + 1], mag[(y + 1) * nx + x - 1]),
            };
            thin[y * nx + x] = m >= a && m >= b;
        }
    }
    // Hysteresis: keep weak edges joined to strong ones.
    let mut keep = vec![false; nx * ny];
    let mut stack: Vec<usize> = (0..nx * ny).filter(|&i| thin[i] && mag[i] >= high).collect();
    for &i in &stack {
        keep[i] = true;
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % nx, i / nx);
        for (dx, dy) in NEIGH8 {
            let (xx, yy) = (x as isize + dx, y as isize + dy);
            if xx < 0 || yy < 0 || xx >= nx as isize || yy >= ny as isize {
                continue;
            }
            let j = yy as usize * nx + xx as usize;
            if thin[j] && !keep[j] {
                keep[j] = true;
                stack.push(j);
            }
        }
    }
    let at = |i: usize| Pt::new((i % nx) as f64 * f.cell + f.cell / 2.0, (i / nx) as f64 * f.cell + f.cell / 2.0);
    let shortest = p.detail;
    for line in trace_pixels(&mut keep, nx, ny) {
        if !out.room() {
            return;
        }
        let pts: Vec<Pt> = line.into_iter().map(at).collect();
        let pts = simplify(&pts, f.cell * 0.6);
        if Polyline::new(pts.clone(), false).length() >= shortest.max(f.cell * 2.0) {
            out.push(pts, false);
        }
    }
    if p.variant == 1 {
        for pts in iso_lines(&f, 0.6) {
            let pts = simplify(&pts, f.cell * 0.6);
            if Polyline::new(pts.clone(), false).length() >= shortest.max(f.cell * 2.0) {
                out.push(pts, false);
            }
        }
    }
}

const NEIGH8: [(isize, isize); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

/// Follow the set pixels into lines (ends first, then what is left in loops); the pixels are cleared on the way.
fn trace_pixels(on: &mut [bool], nx: usize, ny: usize) -> Vec<Vec<usize>> {
    let neighbours = |on: &[bool], i: usize| -> Vec<usize> {
        let (x, y) = (i % nx, i / nx);
        NEIGH8
            .iter()
            .filter_map(|(dx, dy)| {
                let (xx, yy) = (x as isize + dx, y as isize + dy);
                (xx >= 0 && yy >= 0 && xx < nx as isize && yy < ny as isize).then(|| yy as usize * nx + xx as usize).filter(|&j| on[j])
            })
            .collect()
    };
    let mut lines = vec![];
    for pass in 0..2 {
        for i in 0..nx * ny {
            if !on[i] || (pass == 0 && neighbours(on, i).len() != 1) {
                continue;
            }
            let mut line = vec![i];
            on[i] = false;
            let mut cur = i;
            while let Some(&next) = neighbours(on, cur).first() {
                on[next] = false;
                line.push(next);
                cur = next;
            }
            if line.len() > 1 {
                lines.push(line);
            }
        }
    }
    lines
}

/// Douglas–Peucker.
fn simplify(pts: &[Pt], tol: f64) -> Vec<Pt> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    let mut stack = vec![(0, pts.len() - 1)];
    while let Some((a, b)) = stack.pop() {
        let mut best = (0.0, 0);
        for i in a + 1..b {
            let d = crate::geom::seg_dist(pts[a], pts[b], pts[i]);
            if d > best.0 {
                best = (d, i);
            }
        }
        if best.0 > tol {
            keep[best.1] = true;
            stack.push((a, best.1));
            stack.push((best.1, b));
        }
    }
    pts.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// A smooth random angle field (value noise on a grid), for the flow field.
struct Noise {
    nx: usize,
    scale: f64,
    v: Vec<f64>,
}

impl Noise {
    fn new(w: f64, h: f64, scale: f64, rng: &mut Rng) -> Noise {
        let nx = (w / scale).ceil() as usize + 2;
        let ny = (h / scale).ceil() as usize + 2;
        Noise { nx, scale, v: (0..nx * ny).map(|_| rng.range(0.0, TAU)).collect() }
    }

    fn angle(&self, q: Pt) -> f64 {
        let (fx, fy) = ((q.x / self.scale).max(0.0), (q.y / self.scale).max(0.0));
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (tx, ty) = (fx - x0 as f64, fy - y0 as f64);
        let s = |t: f64| t * t * (3.0 - 2.0 * t);
        let (tx, ty) = (s(tx), s(ty));
        let ny = self.v.len() / self.nx;
        let g = |x: usize, y: usize| self.v[y.min(ny - 1) * self.nx + x.min(self.nx - 1)];
        // Interpolate the direction as a vector, so the angles do not jump at 0 / 2π.
        let mut vx = 0.0;
        let mut vy = 0.0;
        for (x, y, k) in [(x0, y0, (1.0 - tx) * (1.0 - ty)), (x0 + 1, y0, tx * (1.0 - ty)), (x0, y0 + 1, (1.0 - tx) * ty), (x0 + 1, y0 + 1, tx * ty)] {
            vx += g(x, y).cos() * k;
            vy += g(x, y).sin() * k;
        }
        vy.atan2(vx)
    }
}

/// Evenly spaced streamlines (Jobard and Lefer 1997) whose spacing follows the darkness.
fn flow_lines(im: &ImageData, p: &StyleParams, out: &mut Out, stop: &dyn Fn() -> bool) {
    let dmin = p.spacing;
    let dmax = p.detail.max(dmin);
    let f = Field::new(im, p, (dmin / 2.0).max(0.1), 1_000_000);
    let smooth = f.blurred(p.strength / 4.0);
    let mut rng = Rng::new(p.seed, 13);
    let noise = Noise::new(f.w, f.h, p.strength.max(1.0), &mut rng);
    let fixed = p.angle.to_radians();
    let dir = |q: Pt| -> Pt {
        let a = if p.variant == 1 {
            noise.angle(q)
        } else {
            let g = smooth.grad(q);
            if g.x.hypot(g.y) < 1e-5 { fixed } else { g.y.atan2(g.x) + PI / 2.0 }
        };
        Pt::new(a.cos(), a.sin())
    };
    let sep = |q: Pt| dmin + (dmax - dmin) * (1.0 - f.at(q));
    let dark = |q: Pt| f.at(q) > 0.03;
    // The points of the lines drawn, in buckets of the smallest gap.
    let cell = dmin;
    let gx = (f.w / cell).ceil() as usize + 1;
    let gy = (f.h / cell).ceil() as usize + 1;
    let mut grid: Vec<Vec<Pt>> = vec![vec![]; gx * gy];
    let bucket = |q: Pt| ((q.y / cell).max(0.0) as usize).min(gy - 1) * gx + ((q.x / cell).max(0.0) as usize).min(gx - 1);
    let free = |grid: &Vec<Vec<Pt>>, q: Pt, r: f64| -> bool {
        let k = (r / cell).ceil() as isize;
        let (cx, cy) = (((q.x / cell).max(0.0) as isize).min(gx as isize - 1), ((q.y / cell).max(0.0) as isize).min(gy as isize - 1));
        for y in (cy - k).max(0)..=(cy + k).min(gy as isize - 1) {
            for x in (cx - k).max(0)..=(cx + k).min(gx as isize - 1) {
                for o in &grid[y as usize * gx + x as usize] {
                    if (o.x - q.x).powi(2) + (o.y - q.y).powi(2) < r * r {
                        return false;
                    }
                }
            }
        }
        true
    };
    let step = dmin / 2.0;
    let max_steps = ((f.w + f.h) * 4.0 / step) as usize;
    // A seed beside a line carries the gap it was put at; a seed of the grid is checked with the gap of its place.
    let mut seeds: Vec<(Pt, f64)> = vec![];
    // Fallback seeds on a grid, in a random order.
    let mut grid_seeds: Vec<Pt> = vec![];
    let mut y = dmax / 2.0;
    while y < f.h {
        let mut x = dmax / 2.0;
        while x < f.w {
            grid_seeds.push(Pt::new(x + rng.range(-0.3, 0.3) * dmax, y + rng.range(-0.3, 0.3) * dmax));
            x += dmax / 2.0;
        }
        y += dmax / 2.0;
    }
    for i in (1..grid_seeds.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        grid_seeds.swap(i, j);
    }
    let mut lines_drawn = 0usize;
    loop {
        if stop() || !out.room() {
            return;
        }
        let (s, gap) = match seeds.pop().or_else(|| grid_seeds.pop().map(|g| (g, sep(g)))) {
            Some(s) => s,
            None => break,
        };
        if !f.inside(s) || !dark(s) || !free(&grid, s, gap.min(sep(s)) * 0.9) {
            continue;
        }
        // Grow both ways from the seed.
        let mut halves: [Vec<Pt>; 2] = [vec![], vec![]];
        for (h, sign) in [(0usize, 1.0), (1usize, -1.0)] {
            let mut q = s;
            let mut prev = dir(q);
            for _ in 0..max_steps {
                let mut d1 = dir(q);
                if d1.x * prev.x + d1.y * prev.y < 0.0 {
                    d1 = Pt::new(-d1.x, -d1.y);
                }
                let mid = add(q, d1, sign * step / 2.0);
                let mut d2 = dir(mid);
                if d2.x * d1.x + d2.y * d1.y < 0.0 {
                    d2 = Pt::new(-d2.x, -d2.y);
                }
                let nq = add(q, d2, sign * step);
                if !f.inside(nq) || !dark(nq) || !free(&grid, nq, sep(nq) * 0.5) {
                    break;
                }
                // Not into itself: the points of this line already taken count too, all but the last few.
                let own = &halves[h];
                let r = sep(nq) * 0.5;
                let recent = (r / step).ceil() as usize + 2;
                if own.len() > recent && own[..own.len() - recent].iter().any(|o| (o.x - nq.x).powi(2) + (o.y - nq.y).powi(2) < r * r) {
                    break;
                }
                // The second half stops where it meets the first one (a closed line), past the seed.
                if h == 1 && halves[0].len() > recent && halves[0][recent..].iter().any(|o| (o.x - nq.x).powi(2) + (o.y - nq.y).powi(2) < r * r) {
                    break;
                }
                halves[h].push(nq);
                prev = d2;
                q = nq;
            }
        }
        let [fwd, back] = halves;
        let mut pts: Vec<Pt> = back.into_iter().rev().collect();
        pts.push(s);
        pts.extend(fwd);
        if Polyline::new(pts.clone(), false).length() < dmin * 3.0 {
            continue;
        }
        for (i, q) in pts.iter().enumerate() {
            grid[bucket(*q)].push(*q);
            // New seeds beside the line, one gap away on both sides.
            if i % 4 == 0 {
                let a = if i + 1 < pts.len() { pts[i + 1] } else { pts[i.saturating_sub(1)] };
                let (dx, dy) = (a.x - q.x, a.y - q.y);
                let l = dx.hypot(dy).max(1e-9);
                let n = Pt::new(-dy / l, dx / l);
                let d = sep(*q);
                seeds.push((add(*q, n, d), d));
                seeds.push((add(*q, n, -d), d));
            }
        }
        out.push(simplify(&pts, step * 0.05), false);
        lines_drawn += 1;
        if lines_drawn > 200_000 {
            break;
        }
    }
}

/// Short straight strokes laid one by one where the picture is still darkest; each stroke takes some ink off the picture.
fn sketch(im: &ImageData, p: &StyleParams, out: &mut Out, stop: &dyn Fn() -> bool) {
    let len = p.spacing;
    let mut f = Field::new(im, p, (len / 12.0).max(0.1), 1_000_000);
    let mut rng = Rng::new(p.seed, 17);
    let strokes = p.detail.round() as usize;
    let ink = p.strength as f32;
    let chain = p.variant == 1;
    let mut pen = Pen::new();
    let mut at: Option<Pt> = None;
    for s in 0..strokes {
        if s.is_multiple_of(64) && stop() || !out.room() {
            break;
        }
        let start = match at {
            Some(q) if chain => q,
            _ => {
                pen.lift(out);
                // The darkest of a few random places.
                let mut best = (Pt::new(0.0, 0.0), -1.0);
                for _ in 0..48 {
                    let q = Pt::new(rng.f() * f.w, rng.f() * f.h);
                    let d = f.at(q);
                    if d > best.1 {
                        best = (q, d);
                    }
                }
                if best.1 < 0.03 {
                    break;
                }
                pen.to(out, best.0, true);
                best.0
            }
        };
        // The direction whose line covers the most darkness.
        let mut best = (start, -1.0);
        for _ in 0..24 {
            let a = rng.range(0.0, TAU);
            let l = len * rng.range(0.6, 1.0);
            let end = Pt::new((start.x + l * a.cos()).clamp(0.0, f.w), (start.y + l * a.sin()).clamp(0.0, f.h));
            let m = line_mean(&f, start, end);
            if m > best.1 {
                best = (end, m);
            }
        }
        if best.1 < 0.03 {
            at = None;
            if !chain {
                pen.lift(out);
            }
            continue;
        }
        if !chain {
            pen.lift(out);
            pen.to(out, start, true);
        }
        pen.to(out, best.0, true);
        lighten_line(&mut f, start, best.0, ink);
        at = Some(best.0);
        if !chain {
            pen.lift(out);
        }
    }
    pen.lift(out);
}

fn line_mean(f: &Field, a: Pt, b: Pt) -> f64 {
    let n = ((a.dist(b) / f.cell).ceil() as usize).max(1);
    (0..=n).map(|i| f.at(Pt::new(a.x + (b.x - a.x) * i as f64 / n as f64, a.y + (b.y - a.y) * i as f64 / n as f64))).sum::<f64>() / (n + 1) as f64
}

fn lighten_line(f: &mut Field, a: Pt, b: Pt, ink: f32) {
    let n = ((a.dist(b) / f.cell).ceil() as usize).max(1);
    let mut last = usize::MAX;
    for i in 0..=n {
        let q = Pt::new(a.x + (b.x - a.x) * i as f64 / n as f64, a.y + (b.y - a.y) * i as f64 / n as f64);
        let (x, y) = (((q.x / f.cell) as usize).min(f.nx - 1), ((q.y / f.cell) as usize).min(f.ny - 1));
        let k = y * f.nx + x;
        if k != last {
            f.v[k] = (f.v[k] - ink).max(0.0);
            last = k;
        }
    }
}

/// One wandering line of loops: it curls round and drifts towards the parts that are still dark, taking ink off as it goes.
fn scribble(im: &ImageData, p: &StyleParams, out: &mut Out, stop: &dyn Fn() -> bool) {
    let loop_size = p.spacing;
    let mut f = Field::new(im, p, (loop_size / 4.0).max(0.1), 1_000_000);
    let look = f.blurred(loop_size);
    let mut rng = Rng::new(p.seed, 19);
    let total = p.detail * 1000.0;
    let ink = p.strength as f32;
    let ds = loop_size / 8.0;
    let turn = TAU / 16.0;
    let mut pen = Pen::new();
    let mut walked = 0.0;
    let mut q: Option<Pt> = None;
    let mut heading = 0.0f64;
    let mut sign = 1.0;
    let mut pale = 0;
    let mut i = 0usize;
    while walked < total {
        i += 1;
        if i.is_multiple_of(512) && stop() || !out.room() {
            break;
        }
        let cur = match q {
            Some(c) => c,
            None => {
                pen.lift(out);
                let mut best = (Pt::new(0.0, 0.0), -1.0);
                for _ in 0..64 {
                    let c = Pt::new(rng.f() * f.w, rng.f() * f.h);
                    let d = f.at(c);
                    if d > best.1 {
                        best = (c, d);
                    }
                }
                if best.1 < 0.1 {
                    break;
                }
                heading = rng.range(0.0, TAU);
                pen.to(out, best.0, true);
                best.0
            }
        };
        // Curl, now and then the other way, and lean towards the remaining dark.
        if rng.f() < 0.02 {
            sign = -sign;
        }
        heading += sign * turn * rng.range(0.6, 1.4);
        let g = look.grad(cur);
        let pull = Pt::new(g.x, g.y);
        let (hx, hy) = (heading.cos() + pull.x * loop_size * 0.5, heading.sin() + pull.y * loop_size * 0.5);
        let a = hy.atan2(hx);
        let next = Pt::new((cur.x + ds * a.cos()).clamp(0.0, f.w), (cur.y + ds * a.sin()).clamp(0.0, f.h));
        walked += ds;
        pen.to(out, next, true);
        lighten_line(&mut f, cur, next, ink);
        if f.at(next) < 0.1 {
            pale += 1;
        } else {
            pale = 0;
        }
        q = if pale > 24 { None } else { Some(next) };
    }
    pen.lift(out);
}

/// Put the lines in the order a pen goes from one to the nearest next (turning a line round when its other end is nearer).
pub fn order_nearest(lines: Vec<Polyline>) -> Vec<Polyline> {
    let n = lines.len();
    if n < 3 {
        return lines;
    }
    // Ends: 2k = start of line k, 2k+1 = its end.
    let ends: Vec<Pt> = lines.iter().flat_map(|l| [l.pts[0], *l.pts.last().unwrap()]).collect();
    let mut grid = MutGrid::new(&ends);
    let mut lines: Vec<Option<Polyline>> = lines.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(n);
    let mut at = Pt::new(0.0, 0.0);
    for _ in 0..n {
        let Some(e) = grid.nearest(&ends, at) else { break };
        let k = e / 2;
        grid.remove(&ends, 2 * k);
        grid.remove(&ends, 2 * k + 1);
        let mut l = lines[k].take().unwrap();
        if e % 2 == 1 && !l.closed {
            l.pts.reverse();
        }
        at = *l.pts.last().unwrap();
        out.push(l);
    }
    out.extend(lines.into_iter().flatten());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(w: u32, h: u32, f: impl Fn(f64, f64) -> u8) -> ImageData {
        let gray = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| f(x as f64 / w as f64, y as f64 / h as f64)).collect();
        ImageData { name: "t".into(), px_w: w, px_h: h, gray, w: 60.0, h: 40.0, invert: false }
    }

    fn never() -> bool {
        false
    }

    /// Dark on the left, white on the right.
    fn gradient() -> ImageData {
        picture(120, 80, |x, _| (x * 255.0) as u8)
    }

    fn ink_in(out: &StyleOut, x0: f64, x1: f64) -> f64 {
        out.lines
            .iter()
            .flat_map(|l| l.pts.windows(2).map(|s| (s[0], s[1])).collect::<Vec<_>>())
            .filter(|(a, b)| (a.x + b.x) / 2.0 >= x0 && (a.x + b.x) / 2.0 < x1)
            .map(|(a, b)| a.dist(b))
            .sum()
    }

    fn small(s: Style) -> StyleParams {
        let mut p = s.defaults();
        // Keep the slow styles quick in the tests.
        if matches!(s, Style::Stipple | Style::Tsp | Style::Delaunay) {
            p.detail = 400.0;
        }
        if s == Style::Sketch {
            p.detail = 600.0;
        }
        if s == Style::Scribble {
            p.detail = 3.0;
        }
        p
    }

    #[test]
    fn white_gives_nothing() {
        let white = picture(60, 40, |_, _| 255);
        for s in Style::ALL {
            let o = style_trace(&white, &small(s), &never);
            // A wave or a spiral row is still drawn over white (flat), unless the pen is lifted there.
            if matches!(s, Style::Squiggle | Style::Sawtooth | Style::Springs | Style::Spiral | Style::Waves | Style::Hilbert) {
                continue;
            }
            assert!(o.lines.is_empty(), "{s:?} drew {} lines on white", o.lines.len());
        }
        for s in [Style::Squiggle, Style::Hilbert] {
            let mut p = small(s);
            p.variant = 1;
            assert!(style_trace(&white, &p, &never).lines.is_empty(), "{s:?} with the pen lifted over white");
        }
    }

    #[test]
    fn every_style_stays_inside_puts_more_ink_where_darker_and_repeats_itself() {
        let im = gradient();
        for s in Style::ALL {
            let p = small(s);
            let o = style_trace(&im, &p, &never);
            assert!(!o.lines.is_empty(), "{s:?} drew nothing");
            for l in &o.lines {
                for q in &l.pts {
                    assert!(q.x.is_finite() && q.y.is_finite(), "{s:?}");
                    assert!(q.x >= -1e-6 && q.y >= -1e-6 && q.x <= im.w + 1e-6 && q.y <= im.h + 1e-6, "{s:?} out of the picture: {q:?}");
                }
            }
            // The dark third has more ink than the light third. The exceptions: contour lines (one line per level wherever the
            // tone passes it, evenly spread over a gradient), line art (edges, not tone) and waves (the darkness lifts the rows,
            // it does not add lines).
            if !matches!(s, Style::Contours | Style::LineArt | Style::Waves) {
                let (dark, light) = (ink_in(&o, 0.0, 20.0), ink_in(&o, 40.0, 60.0));
                assert!(dark > light, "{s:?}: dark {dark:.1} mm, light {light:.1} mm");
            }
            assert_eq!(o, style_trace(&im, &p, &never), "{s:?} differs with the same seed");
        }
    }

    #[test]
    fn line_art_and_contours_find_the_edge_of_a_disc() {
        let disc = picture(120, 80, |x, y| if (x * 60.0 - 30.0).hypot(y * 40.0 - 20.0) < 12.0 { 0 } else { 255 });
        for s in [Style::LineArt, Style::Contours] {
            let o = style_trace(&disc, &s.defaults(), &never);
            let all: Vec<Pt> = o.lines.iter().flat_map(|l| l.pts.clone()).collect();
            assert!(!all.is_empty(), "{s:?}");
            // Every point lies near the circle of radius 12 round (30, 20).
            for q in &all {
                let r = (q.x - 30.0).hypot(q.y - 20.0);
                assert!((r - 12.0).abs() < 3.0, "{s:?}: a point {r:.2} mm from the centre");
            }
        }
    }

    #[test]
    fn tsp_is_one_line_without_crossings_worth_fixing() {
        let im = gradient();
        let mut p = Style::Tsp.defaults();
        p.detail = 300.0;
        let o = style_trace(&im, &p, &never);
        assert_eq!(o.lines.len(), 1);
        let pts = &o.lines[0].pts;
        assert!(pts.len() > 250);
        // 2-opt leaves very few crossings.
        let mut crossings = 0;
        for i in 0..pts.len() - 1 {
            for j in i + 2..pts.len() - 1 {
                if crate::geom::segments_cross(pts[i], pts[i + 1], pts[j], pts[j + 1]) {
                    crossings += 1;
                }
            }
        }
        assert!(crossings <= pts.len() / 50, "{crossings} crossings");
    }

    #[test]
    fn a_dot_is_a_small_closed_ring_that_survives_the_way_to_the_gcode() {
        let im = picture(60, 40, |_, _| 0);
        let mut p = Style::Stipple.defaults();
        p.detail = 200.0;
        let o = style_trace(&im, &p, &never);
        assert!(o.lines.len() >= 190);
        assert!(o.lines.iter().all(|l| l.closed && l.pts.len() >= 7 && l.length() > 0.5));
    }

    #[test]
    fn the_settings_are_kept_in_range_and_a_new_style_keeps_the_tone() {
        let mut p = StyleParams { spacing: -5.0, detail: f64::NAN, variant: 9, ..Style::Halftone.defaults() };
        p.black = 0.3;
        p.gamma = 2.0;
        let c = p.clamped();
        assert!(c.spacing >= 0.5 && c.detail >= 0.1 && c.variant == 2);
        let s = p.with_style(Style::Spiral);
        assert_eq!((s.style, s.black, s.gamma, s.spacing), (Style::Spiral, 0.3, 2.0, Style::Spiral.defaults().spacing));
    }

    #[test]
    fn stop_ends_the_slow_styles_early() {
        let im = gradient();
        let stop = || true;
        for s in [Style::Stipple, Style::Tsp, Style::FlowLines, Style::Sketch, Style::Scribble, Style::Delaunay] {
            // Must return at once without hanging; what it returns does not matter.
            let _ = style_trace(&im, &s.defaults(), &stop);
        }
    }

    #[test]
    fn the_order_puts_near_lines_one_after_the_other() {
        let lines: Vec<Polyline> = [0.0, 50.0, 10.0, 40.0, 20.0, 30.0].iter().map(|&x| Polyline::new(vec![Pt::new(x, 0.0), Pt::new(x, 5.0)], false)).collect();
        let o = order_nearest(lines);
        let xs: Vec<f64> = o.iter().map(|l| l.pts[0].x).collect();
        assert_eq!(xs, vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0]);
        // Every other one turned round, so the pen goes up and down.
        assert_eq!(o[1].pts[0].y, 5.0);
    }
}

#[cfg(test)]
mod sheet {
    use super::*;
    /// Draws every style of a test scene into one picture (STYLE_SHEET=path).
    #[test]
    #[ignore]
    fn contact_sheet() {
        let Some(path) = std::env::var_os("STYLE_SHEET") else { return };
        let (w, h) = (600u32, 400u32);
        let gray: Vec<u8> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| {
            let (fx, fy) = (x as f64, y as f64);
            // A lit sphere, a dark ring, a gradient bar.
            let (dx, dy) = ((fx - 200.0) / 150.0, (fy - 200.0) / 150.0);
            let r2 = dx * dx + dy * dy;
            let mut g = 235.0;
            if r2 < 1.0 {
                let z = (1.0 - r2).sqrt();
                let l = (-0.5 * dx - 0.6 * dy + 0.62 * z).max(0.0);
                g = 30.0 + 220.0 * l;
            }
            let rr = ((fx - 470.0).powi(2) + (fy - 130.0).powi(2)).sqrt();
            if (rr - 70.0).abs() < 18.0 {
                g = 20.0;
            }
            if fx > 380.0 && fy > 260.0 && fy < 360.0 {
                g = 255.0 * (fx - 380.0) / 200.0;
            }
            g.clamp(0.0, 255.0) as u8
        }).collect();
        let im = ImageData { name: "s".into(), px_w: w, px_h: h, gray, w: 150.0, h: 100.0, invert: false };
        let (cw, ch, cols) = (450u32, 300u32, 4u32);
        let rows = (Style::ALL.len() as u32 + 1).div_ceil(cols);
        let mut img = image::RgbImage::from_pixel(cw * cols, ch * rows, image::Rgb([255, 255, 255]));
        let k = cw as f64 / im.w;
        // The scene itself first.
        for y in 0..ch {
            for x in 0..cw {
                let g = im.gray[((y as f64 / ch as f64 * h as f64) as u32 * w + (x as f64 / cw as f64 * w as f64) as u32) as usize];
                img.put_pixel(x, y, image::Rgb([g, g, g]));
            }
        }
        for (i, s) in Style::ALL.iter().enumerate() {
            let (ox, oy) = (((i as u32 + 1) % cols) * cw, ((i as u32 + 1) / cols) * ch);
            let o = style_trace(&im, &s.defaults(), &|| false);
            for l in &o.lines {
                for seg in l.pts.windows(2) {
                    let (a, b) = (seg[0], seg[1]);
                    let n = ((a.dist(b) * k * 2.0).ceil() as usize).max(1);
                    for t in 0..=n {
                        let x = (a.x + (b.x - a.x) * t as f64 / n as f64) * k;
                        let y = (a.y + (b.y - a.y) * t as f64 / n as f64) * k;
                        if x >= 0.0 && y >= 0.0 && (x as u32) < cw && (y as u32) < ch {
                            img.put_pixel(ox + x as u32, oy + y as u32, image::Rgb([0, 0, 0]));
                        }
                    }
                }
            }
            // A frame between the cells.
            for x in 0..cw {
                img.put_pixel(ox + x, oy, image::Rgb([200, 0, 0]));
            }
            for y in 0..ch {
                img.put_pixel(ox, oy + y, image::Rgb([200, 0, 0]));
            }
        }
        img.save(path).unwrap();
    }
}
