//! Triangle meshes (STL files) for plugins and programs: reading binary and ASCII STL, grouping the triangles that lie in
//! one plane, projecting a mesh (or some of its triangles) on the floor plane and cutting it with a horizontal plane. The loops over
//! hundreds of thousands of triangles are here, in native code; a plugin script only chooses what to ask for (a script
//! engine is too slow, and too strict about the size of its arrays, for that).
use crate::geom::{Polyline, Pt};
use std::collections::HashMap;

/// The most triangles that are read (a binary STL of 4 million triangles is 200 MB).
pub const MAX_TRIANGLES: usize = 4_000_000;

type V3 = [f64; 3];

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub tris: Vec<[V3; 3]>,
}

/// The triangles that lie in one plane (or nearly), with the plane: unit normal `n` and `d` with `n · p = d`.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub n: V3,
    pub d: f64,
    pub area: f64,
    pub tris: Vec<u32>,
    pub min_z: f64,
    pub max_z: f64,
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// The turn of a view: about Z by `yaw` degrees, then about X by `-pitch` degrees. The viewer looks down the Z axis of the result
/// (the image has X to the right and Y up).
pub fn view_rot(yaw: f64, pitch: f64) -> [[f64; 3]; 3] {
    let (sy, cy) = yaw.to_radians().sin_cos();
    let (sp, cp) = (-pitch).to_radians().sin_cos();
    let rz = [[cy, -sy, 0.0], [sy, cy, 0.0], [0.0, 0.0, 1.0]];
    let rx = [[1.0, 0.0, 0.0], [0.0, cp, -sp], [0.0, sp, cp]];
    let mut m = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            m[i][j] = (0..3).map(|k| rx[i][k] * rz[k][j]).sum();
        }
    }
    m
}

/// A point turned by a matrix of [`view_rot`].
pub fn rot(m: &[[f64; 3]; 3], p: V3) -> V3 {
    [0, 1, 2].map(|i| m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2])
}

impl Mesh {
    /// Read an STL file, binary or ASCII. The numbers must be finite.
    pub fn from_stl(bytes: &[u8]) -> Result<Mesh, String> {
        if bytes.len() >= 84 {
            let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
            if count <= MAX_TRIANGLES && 84 + 50 * count == bytes.len() {
                return Mesh::binary(bytes, count);
            }
        }
        let head = &bytes[..bytes.len().min(512)];
        if head.iter().any(|b| *b == 0) {
            return Err("this is not an STL file (a binary one has the wrong size)".into());
        }
        Mesh::ascii(&String::from_utf8_lossy(bytes))
    }

    fn binary(bytes: &[u8], count: usize) -> Result<Mesh, String> {
        let f = |o: usize| f32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]) as f64;
        let mut tris = Vec::with_capacity(count);
        for i in 0..count {
            let o = 84 + 50 * i + 12;
            let t = [[f(o), f(o + 4), f(o + 8)], [f(o + 12), f(o + 16), f(o + 20)], [f(o + 24), f(o + 28), f(o + 32)]];
            if t.iter().flatten().any(|v| !v.is_finite()) {
                return Err(format!("triangle {} has a number that is not finite", i + 1));
            }
            tris.push(t);
        }
        Ok(Mesh { tris })
    }

    fn ascii(text: &str) -> Result<Mesh, String> {
        if !text.trim_start().to_ascii_lowercase().starts_with("solid") {
            return Err("this is not an STL file".into());
        }
        let mut tris = vec![];
        let mut cur: Vec<V3> = vec![];
        for line in text.lines() {
            let mut w = line.split_whitespace();
            if w.next().map(|k| k.eq_ignore_ascii_case("vertex")) != Some(true) {
                continue;
            }
            let v: Vec<f64> = w.take(3).filter_map(|s| s.parse::<f64>().ok()).collect();
            if v.len() != 3 || v.iter().any(|x| !x.is_finite()) {
                return Err("a vertex has no three numbers".into());
            }
            cur.push([v[0], v[1], v[2]]);
            if cur.len() == 3 {
                tris.push([cur[0], cur[1], cur[2]]);
                cur.clear();
                if tris.len() > MAX_TRIANGLES {
                    return Err("too many triangles".into());
                }
            }
        }
        if tris.is_empty() {
            return Err("the file has no triangles".into());
        }
        Ok(Mesh { tris })
    }

    pub fn len(&self) -> usize {
        self.tris.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    /// The box round the mesh: min x, y, z and max x, y, z.
    pub fn bounds(&self) -> Option<[f64; 6]> {
        let mut it = self.tris.iter().flatten();
        let first = it.next()?;
        let mut b = [first[0], first[1], first[2], first[0], first[1], first[2]];
        for p in it {
            for k in 0..3 {
                b[k] = b[k].min(p[k]);
                b[k + 3] = b[k + 3].max(p[k]);
            }
        }
        Some(b)
    }

    /// The mesh turned for a view: first about Z by `yaw` degrees, then tilted by `pitch` degrees about X, so that the view from
    /// above of the result is the view of the mesh from the direction (0, 0 is the view from above, 0, 90 from the front). The
    /// triangles keep their order, so the planes found by `plane_groups` still name the same triangles.
    pub fn rotated(&self, yaw: f64, pitch: f64) -> Mesh {
        let r = view_rot(yaw, pitch);
        Mesh { tris: self.tris.iter().map(|t| [rot(&r, t[0]), rot(&r, t[1]), rot(&r, t[2])]).collect() }
    }

    /// The unit normal of triangle `i` and its area; `None` for a triangle with no area.
    pub fn normal_area(&self, i: usize) -> Option<(V3, f64)> {
        let t = self.tris.get(i)?;
        let c = cross(sub(t[1], t[0]), sub(t[2], t[0]));
        let l = len(c);
        (l > 1e-12).then(|| ([c[0] / l, c[1] / l, c[2] / l], l / 2.0))
    }

    /// Group the triangles that lie in one plane: the unit normals differ by less than about `angle_deg` and the planes are less
    /// than `dist` apart. The groups come largest first. (A plane whose normals straddle the border of two cells of the grid
    /// the normals are put in is split; an exact CAD export has no such planes.)
    pub fn plane_groups(&self, angle_deg: f64, dist: f64) -> Vec<Group> {
        let cell = (angle_deg.clamp(0.01, 45.0).to_radians()).max(1e-4);
        let dist = dist.max(1e-6);
        let mut index: HashMap<(i64, i64, i64, i64), usize> = HashMap::new();
        let mut groups: Vec<Group> = vec![];
        for i in 0..self.tris.len() {
            let Some((n, area)) = self.normal_area(i) else { continue };
            let d = dot(n, self.tris[i][0]);
            let key = ((n[0] / cell).round() as i64, (n[1] / cell).round() as i64, (n[2] / cell).round() as i64, (d / dist).round() as i64);
            let at = *index.entry(key).or_insert_with(|| {
                groups.push(Group { n, d, area: 0.0, tris: vec![], min_z: f64::INFINITY, max_z: f64::NEG_INFINITY });
                groups.len() - 1
            });
            let g = &mut groups[at];
            g.area += area;
            g.tris.push(i as u32);
            for p in &self.tris[i] {
                g.min_z = g.min_z.min(p[2]);
                g.max_z = g.max_z.max(p[2]);
            }
        }
        groups.sort_by(|a, b| b.area.total_cmp(&a.area));
        groups
    }

    /// The outline of what the triangles (all, or only those with these indices) cover when they are looked at from above:
    /// the union of their shadows on the floor, as closed polylines (outer rings counter-clockwise, holes clockwise).
    pub fn project(&self, only: Option<&[u32]>) -> Vec<Polyline> {
        let shadow = |t: &[V3; 3]| Polyline::new(t.iter().map(|p| Pt::new(p[0], p[1])).collect(), true);
        let rings: Vec<Polyline> = match only {
            None => self.tris.iter().map(shadow).collect(),
            Some(ids) => ids.iter().filter_map(|i| self.tris.get(*i as usize)).map(shadow).collect(),
        };
        // A triangle seen edge on has no shadow.
        let rings: Vec<Polyline> = rings.into_iter().filter(|r| r.area().abs() > 1e-12).collect();
        crate::ops::union_all(&rings)
    }

    /// The closed outlines where the plane at height `z` cuts the mesh. Open chains (a mesh with holes in it) are left out;
    /// the second value is how many there were.
    pub fn section(&self, z: f64) -> (Vec<Polyline>, usize) {
        // The plane is moved a hair so that a vertex lying on it is not counted twice.
        let z = z + 1e-9;
        let mut segs: Vec<(Pt, Pt)> = vec![];
        for t in &self.tris {
            let side: Vec<bool> = t.iter().map(|p| p[2] > z).collect();
            if side[0] == side[1] && side[1] == side[2] {
                continue;
            }
            let mut pts: Vec<Pt> = vec![];
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                if side[a] != side[b] {
                    let k = (z - t[a][2]) / (t[b][2] - t[a][2]);
                    pts.push(Pt::new(t[a][0] + k * (t[b][0] - t[a][0]), t[a][1] + k * (t[b][1] - t[a][1])));
                }
            }
            if pts.len() == 2 {
                segs.push((pts[0], pts[1]));
            }
        }
        link_segments(segs)
    }
}

/// Join segments end to end into closed rings. Returns the rings and the number of chains that did not close.
fn link_segments(segs: Vec<(Pt, Pt)>) -> (Vec<Polyline>, usize) {
    let q = |p: Pt| ((p.x * 1e4).round() as i64, (p.y * 1e4).round() as i64);
    let mut at: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, (a, b)) in segs.iter().enumerate() {
        at.entry(q(*a)).or_default().push(i);
        at.entry(q(*b)).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let (mut rings, mut open) = (vec![], 0);
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut pts = vec![segs[start].0, segs[start].1];
        let first = q(segs[start].0);
        loop {
            let end = q(*pts.last().unwrap());
            if end == first && pts.len() > 2 {
                pts.pop();
                rings.push(Polyline::new(pts, true));
                break;
            }
            let next = at.get(&end).and_then(|v| v.iter().copied().find(|j| !used[*j]));
            match next {
                Some(j) => {
                    used[j] = true;
                    let (a, b) = segs[j];
                    pts.push(if q(a) == end { b } else { a });
                }
                None => {
                    open += 1;
                    break;
                }
            }
        }
    }
    (rings, open)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box from (0,0,0) to (w,d,h) as 12 triangles.
    pub fn cube(w: f64, d: f64, h: f64) -> Mesh {
        let v = |x: f64, y: f64, z: f64| [x * w, y * d, z * h];
        let quad = |a: V3, b: V3, c: V3, e: V3| vec![[a, b, c], [a, c, e]];
        let mut tris = vec![];
        tris.extend(quad(v(0., 0., 0.), v(0., 1., 0.), v(1., 1., 0.), v(1., 0., 0.))); // bottom
        tris.extend(quad(v(0., 0., 1.), v(1., 0., 1.), v(1., 1., 1.), v(0., 1., 1.))); // top
        tris.extend(quad(v(0., 0., 0.), v(1., 0., 0.), v(1., 0., 1.), v(0., 0., 1.))); // front
        tris.extend(quad(v(0., 1., 0.), v(0., 1., 1.), v(1., 1., 1.), v(1., 1., 0.))); // back
        tris.extend(quad(v(0., 0., 0.), v(0., 0., 1.), v(0., 1., 1.), v(0., 1., 0.))); // left
        tris.extend(quad(v(1., 0., 0.), v(1., 1., 0.), v(1., 1., 1.), v(1., 0., 1.))); // right
        Mesh { tris }
    }

    pub fn binary_stl(m: &Mesh) -> Vec<u8> {
        let mut b = vec![0u8; 80];
        b.extend((m.tris.len() as u32).to_le_bytes());
        for t in &m.tris {
            b.extend([0u8; 12]);
            for p in t {
                for c in p {
                    b.extend((*c as f32).to_le_bytes());
                }
            }
            b.extend([0u8; 2]);
        }
        b
    }

    fn ascii_stl(m: &Mesh) -> String {
        let mut s = String::from("solid test\n");
        for t in &m.tris {
            s.push_str("facet normal 0 0 0\n outer loop\n");
            for p in t {
                s.push_str(&format!("  vertex {} {} {}\n", p[0], p[1], p[2]));
            }
            s.push_str(" endloop\nendfacet\n");
        }
        s.push_str("endsolid test\n");
        s
    }

    fn area(v: &[Polyline]) -> f64 {
        v.iter().map(|p| p.area()).sum::<f64>().abs()
    }

    #[test]
    fn binary_and_ascii_stl_are_read_and_nonsense_is_refused() {
        let m = cube(10.0, 20.0, 5.0);
        let b = Mesh::from_stl(&binary_stl(&m)).unwrap();
        assert_eq!(b.len(), 12);
        let a = Mesh::from_stl(ascii_stl(&m).as_bytes()).unwrap();
        assert_eq!(a.len(), 12);
        assert_eq!(a.bounds().unwrap(), [0.0, 0.0, 0.0, 10.0, 20.0, 5.0]);
        assert!(Mesh::from_stl(b"just some text").is_err());
        assert!(Mesh::from_stl(&[0u8; 100]).is_err(), "a binary file of the wrong size");
        assert!(Mesh::from_stl(b"solid x\nvertex 1 2\n").is_err());
        let mut nan = binary_stl(&m);
        nan[96..100].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(Mesh::from_stl(&nan).unwrap_err().contains("not finite"));
    }

    #[test]
    fn the_triangles_of_a_box_fall_into_six_planes() {
        let g = cube(10.0, 20.0, 5.0).plane_groups(1.0, 0.01);
        assert_eq!(g.len(), 6);
        assert!(g.iter().all(|x| x.tris.len() == 2));
        // The biggest are the top and the bottom (10 x 20), the smallest the ends (20 x 5 = 100 / 10 x 5 = 50).
        assert!((g[0].area - 200.0).abs() < 1e-9 && (g[1].area - 200.0).abs() < 1e-9);
        assert!((g[5].area - 50.0).abs() < 1e-9);
        // Two parallel planes at different heights are different groups, and one plane made of two parts is one group.
        let mut m = cube(10.0, 10.0, 1.0);
        let mut upper = cube(10.0, 10.0, 1.0);
        for t in &mut upper.tris {
            for p in t {
                p[2] += 5.0;
            }
        }
        m.tris.extend(upper.tris);
        let g = m.plane_groups(1.0, 0.01);
        assert_eq!(g.iter().filter(|x| x.n[2] > 0.99).count(), 2, "two tops at different heights");
    }

    #[test]
    fn a_mesh_is_projected_on_the_floor_whole_or_by_planes() {
        let m = cube(10.0, 20.0, 5.0);
        let all = m.project(None);
        assert_eq!(all.len(), 1);
        assert!((area(&all) - 200.0).abs() < 1e-6, "{}", area(&all));
        // Only the front wall (a plane standing on its edge) has no shadow; only the top is the whole rectangle.
        let groups = m.plane_groups(1.0, 0.01);
        let top = groups.iter().find(|g| g.n[2] > 0.99).unwrap();
        assert!((area(&m.project(Some(&top.tris))) - 200.0).abs() < 1e-6);
        let front = groups.iter().find(|g| g.n[1] < -0.99).unwrap();
        assert!(m.project(Some(&front.tris)).is_empty());
    }

    #[test]
    fn a_plane_cuts_a_mesh_into_rings() {
        let m = cube(10.0, 20.0, 5.0);
        let (rings, open) = m.section(2.5);
        assert_eq!((rings.len(), open), (1, 0));
        assert!((area(&rings) - 200.0).abs() < 1e-6);
        // Above and below the mesh there is nothing; exactly on a face is still a clean outline.
        assert!(m.section(9.0).0.is_empty() && m.section(-1.0).0.is_empty());
        let on_top = m.section(5.0);
        assert!(on_top.1 == 0);
        // A cube with a box-shaped hole through it: the plane cuts an outer ring and an inner one.
        let mut frame = cube(30.0, 30.0, 5.0);
        let mut hole = cube(10.0, 10.0, 5.0);
        for t in &mut hole.tris {
            for p in t {
                p[0] += 10.0;
                p[1] += 10.0;
                // Inside out: the walls of a hole look the other way, but a section does not care.
            }
        }
        frame.tris.extend(hole.tris);
        let (rings, open) = frame.section(2.0);
        assert_eq!((rings.len(), open), (2, 0));
    }

    #[test]
    fn a_view_turns_the_mesh_before_it_is_projected() {
        let m = cube(10.0, 20.0, 5.0);
        // From above nothing changes.
        assert_eq!(m.rotated(0.0, 0.0).bounds().unwrap(), m.bounds().unwrap());
        // From the front the shadow is the face seen from the front: X to the right, Z up (the height of the box is the new Y).
        let front = m.rotated(0.0, 90.0);
        let b = front.bounds().unwrap();
        assert!((b[3] - b[0] - 10.0).abs() < 1e-9 && (b[4] - b[1] - 5.0).abs() < 1e-9 && (b[5] - b[2] - 20.0).abs() < 1e-9, "{b:?}");
        assert!((area(&front.project(None)) - 50.0).abs() < 1e-6);
        // From the side (a quarter turn about Z first) it is the other face.
        assert!((area(&m.rotated(90.0, 90.0).project(None)) - 100.0).abs() < 1e-6);
        // The planes keep their triangles: the top of the box has the same two triangles after the turn.
        let g = m.plane_groups(1.0, 0.01);
        let top = g.iter().find(|g| g.n[2] > 0.99).unwrap();
        assert!((area(&front.project(Some(&top.tris))) - 0.0).abs() < 1e-6, "the top seen from the front is an edge");
    }
}
