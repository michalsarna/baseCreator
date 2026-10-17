// Format: a count (u32) and then that many triangles of six f32 (x1 y1 x2 y2 x3 y3). They are merged into outlines.
fn read(data, ctx) {
    let n = data.parse_le_int(0, 4);
    let tris = [];
    for i in 0..n {
        let o = 4 + 24 * i;
        let v = [];
        for k in 0..6 { v.push(read_f32(data, o + 4 * k)); }
        tris.push([[v[0], v[1]], [v[2], v[3]], [v[4], v[5]]]);
    }
    let rings = union_polygons(tris);
    let out = [];
    for r in rings { out.push(polygon(r)); }
    out
}
