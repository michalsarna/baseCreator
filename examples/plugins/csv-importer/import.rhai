// Reads "x,y" lines. A blank line ends a polyline; the word "closed" on a line of its own closes it.
fn read(text, ctx) {
    let shapes = [];
    let pts = [];
    let closed = false;
        for raw in text.split('\n') {
        let line = raw;
        line.trim();
        if line == "" || line == "closed" {
            if line == "closed" { closed = true; }
            if pts.len() >= 2 {
                shapes.push(if closed { polygon(pts) } else { polyline(pts) });
            }
            pts = [];
            closed = false;
            continue;
        }
        if line.starts_with("#") { continue; }
        let parts = line.split(',');
        if parts.len() < 2 {
            warn("skipped a line that is not x,y: " + line);
            continue;
        }
        parts[0].trim();
        parts[1].trim();
        let x = parse_float(parts[0]);
        let y = parse_float(parts[1]);
        // The file counts Y up from the bottom; the drawing counts down from the top.
        pts.push([x, ctx.area_h - y]);
    }
    if pts.len() >= 2 {
        shapes.push(if closed { polygon(pts) } else { polyline(pts) });
    }
    shapes
}
