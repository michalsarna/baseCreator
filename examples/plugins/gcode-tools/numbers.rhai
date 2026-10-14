fn post(lines, ctx) {
    let out = [];
    let n = 10;
    for l in lines {
        let t = l;
        t.trim();
        if t == "" { continue; }
        out.push("N" + n + " " + l);
        n += 10;
    }
    out
}
