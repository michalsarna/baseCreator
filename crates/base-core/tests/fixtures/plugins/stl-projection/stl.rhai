// STL model -> outlines. The heavy loops over the triangles are native (read_stl, groups, project, section, xf_rings);
// this script decides what to ask for and builds the form from what the file holds.
//
// A model is looked at from above, Z up, unless the person turned it in the 3D view (view_yaw, view_pitch). The outlines come out in drawing coordinates (millimetres, Y down), with the top left
// corner of the box of the model at (0, 0); the outlines keep their place in it.

fn modes(ctx) { ["Shadow of the whole model", "Shadow of the chosen planes", "Slice at a depth"] }

fn facing_of(nz) {
    if nz > 0.5 { "up" } else if nz < -0.5 { "down" } else { "sideways" }
}

fn form(data, ctx) {
    let m = read_stl(data);
    let b = m.bounds();
    let height = b[5] - b[2];
    let fields = [];
    let first = if ctx.app == "millcreator" { 2 } else { 0 };
    fields.push(#{ id: "mode", label: "What to make", kind: "choice", options: modes(ctx), "default": modes(ctx)[first] });
    fields.push(#{ id: "below", label: "View from below", kind: "bool", "default": false });
    fields.push(#{ id: "depth", label: "Depth of the slice below the top", kind: "number", "default": height / 2.0, min: 0.0, max: height, unit: "mm" });
    fields.push(#{ id: "facing", label: "Show only planes facing", kind: "choice", options: ["up", "down", "sideways", "any way"], "default": "up" });
    fields.push(#{ id: "min_area", label: "Smallest plane", kind: "number", "default": 1.0, min: 0.0, max: 1000000.0, unit: "mm²" });
    let groups = m.groups(1.0, 0.05);
    // A form has at most 40 fields in all, five of them above: the biggest planes get a tick box, the rest follow the filters.
    groups.sort(|a, b| if a.area > b.area { -1 } else if a.area < b.area { 1 } else { 0 });
    let shown = 0;
    for g in groups {
        if shown >= 35 { break; }
        shown += 1;
        let arrow = if g.nz > 0.5 { "↑" } else if g.nz < -0.5 { "↓" } else { "↔" };
        let z0 = g.min_z.round().to_int();
        let z1 = g.max_z.round().to_int();
        let z = if z0 == z1 { "z " + z0 } else { "z " + z0 + "…" + z1 };
        let label = "#" + (g.id + 1) + "  " + arrow + "  " + g.area.round().to_int() + " mm²  " + z;
        fields.push(#{ id: "plane_" + g.id, label: label, kind: "bool", "default": g.nz > 0.5 });
    }
    fields
}

fn read(data, ctx) {
    let m = read_stl(data);
    let p = ctx.p;
    let mode = p.mode;
    if mode == () { mode = modes(ctx)[0]; }
    // The planes of the form are the planes of the model as it lies; the view (the turn the person gave the model in the 3D view,
    // 0 and 0 from above) comes after them, and the outlines are those of the model seen that way.
    let groups = if mode == "Shadow of the chosen planes" { m.groups(1.0, 0.05) } else { [] };
    let yaw = if p.view_yaw == () { 0.0 } else { p.view_yaw };
    let pitch = if p.view_pitch == () { 0.0 } else { p.view_pitch };
    let b0 = m.bounds();
    let turned = yaw != 0.0 || pitch != 0.0;
    let m = if turned { m.rotated(yaw, pitch) } else { m };
    let b = m.bounds();
    let rings = [];
    if mode == "Slice at a depth" {
        let depth = if p.depth == () { 0.0 } else { p.depth };
        // The depth of the form is a part of the height of the model as it lies; turned, the same part of its depth in the view.
        if turned && b0[5] > b0[2] { depth = depth / (b0[5] - b0[2]) * (b[5] - b[2]); }
        rings = m.section(b[5] - depth);
    } else if mode == "Shadow of the chosen planes" {
        let ids = [];
        let facing = if p.facing == () { "any way" } else { p.facing };
        let min_area = if p.min_area == () { 0.0 } else { p.min_area };
        for g in groups {
            let chosen = p["plane_" + g.id];
            if chosen == () { chosen = g.nz > 0.5; }
            let ok = chosen && g.area >= min_area && (facing == "any way" || facing_of(g.nz) == facing);
            if ok { ids.push(g.id); }
        }
        if ids.len() == 0 {
            warn("No plane is chosen");
            return [];
        }
        rings = m.project(ids);
    } else {
        rings = m.project();
    }
    // To drawing coordinates: Y turned over (Y is up in the model, down in the drawing), the box moved to the corner; from below
    // the model is seen mirrored.
    let from_below = p.below != () && p.below;
    let sx = if from_below { -1.0 } else { 1.0 };
    let dx = if from_below { b[3] } else { -b[0] };
    let rings = xf_rings(rings, sx, -1.0, dx, b[4]);
    if rings.len() == 0 {
        warn("Nothing to make: the chosen planes or the slice are empty");
    }
    let out = [];
    for r in rings { out.push(polygon(r)); }
    out
}
