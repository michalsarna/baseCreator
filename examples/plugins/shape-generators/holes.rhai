fn generate(p, ctx) {
    let n = p.count.to_int();
    let r = p.circle_d / 2.0;
    // Put the pattern in the middle of the work area.
    let cx = ctx.area_w / 2.0;
    let cy = ctx.area_h / 2.0;
    let out = [];
    for i in 0..n {
        let a = 2.0 * PI() * i.to_float() / n.to_float();
        let h = circle(cx + r * cos(a), cy + r * sin(a), p.hole_d / 2.0);
        h.layer = p.layer.to_int();
        out.push(h);
    }
    if p.center {
        let c = circle(cx, cy, p.hole_d / 2.0);
        c.layer = p.layer.to_int();
        out.push(c);
    }
    out
}
