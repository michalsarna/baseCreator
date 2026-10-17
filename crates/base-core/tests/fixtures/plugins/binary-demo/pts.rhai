// Format: a count (u32) and then that many (x, y) pairs of f32.
fn form(data, ctx) {
    let n = data.parse_le_int(0, 4);
    let fields = [#{ id: "closed", label: "Closed", kind: "bool", "default": true }];
    for i in 0..n {
        fields.push(#{ id: "keep_" + i, label: "Keep point " + (i + 1), kind: "bool", "default": true });
    }
    fields
}

fn read(data, ctx) {
    let n = data.parse_le_int(0, 4);
    let pts = [];
    for i in 0..n {
        let keep = ctx.p["keep_" + i];
        if keep == () || keep {
            pts.push([read_f32(data, 4 + 8 * i) * ctx.p.scale, read_f32(data, 8 + 8 * i) * ctx.p.scale]);
        }
    }
    if ctx.p.closed == () || ctx.p.closed { polygon(pts) } else { polyline(pts) }
}
