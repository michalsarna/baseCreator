fn generate(p, ctx) {
    let z = p.teeth.to_int();
    let m = p.size;
    let r_pitch = m * z.to_float() / 2.0;
    let r_out = r_pitch + m;
    let r_root = r_pitch - 1.25 * m;
    let pitch_angle = 2.0 * PI() / z.to_float();
    let cx = ctx.area_w / 2.0;
    let cy = ctx.area_h / 2.0;
    let pts = [];
    for i in 0..z {
        let a0 = pitch_angle * i.to_float();
        // The tooth takes a quarter of the pitch at the top and half at the root (trapezoid) or a quarter all the way.
        let top = pitch_angle * 0.25;
        let root = if p.style == "square" { top } else { pitch_angle * 0.5 };
        let gap = pitch_angle * 0.5;
        let angles = [a0 - root / 2.0, a0 - top / 2.0, a0 + top / 2.0, a0 + root / 2.0];
        let radii = [r_root, r_out, r_out, r_root];
        for k in 0..4 {
            pts.push([cx + radii[k] * cos(angles[k]), cy + radii[k] * sin(angles[k])]);
        }
        let _unused = gap;
    }
    let out = [polygon(pts)];
    if p.bore > 0.0 {
        out.push(circle(cx, cy, p.bore / 2.0));
    }
    if r_root <= p.bore / 2.0 {
        warn("the bore is larger than the root of the teeth");
    }
    out
}
