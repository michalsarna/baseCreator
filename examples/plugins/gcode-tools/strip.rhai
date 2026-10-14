fn post(lines, ctx) {
    let out = [];
    for l in lines {
        let t = l;
        let i = t.index_of(";");
        if i >= 0 { t = t.sub_string(0, i); }
        // Remove (comments) in parentheses.
        let a = t.index_of("(");
        while a >= 0 {
            let b = t.index_of(")");
            if b < a { break; }
            t = t.sub_string(0, a) + t.sub_string(b + 1);
            a = t.index_of("(");
        }
        t.trim();
        if t != "" { out.push(t); }
    }
    out
}
