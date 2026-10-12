#!/usr/bin/env python3
"""Regenerate crates/base-ui/src/texts.rs and crates/base-ui/src/texts/*.txt.

The shared interface texts are the English strings the code of crates/base-ui asks for with tr("...") / trf("..."),
the labels of the menu entries in menu.rs, the labels of the tools, and the six alignment labels. Their translations are taken from a
checkout of LightCreator (assets/lang/*.txt and the table in crates/lightcreator/src/i18n.rs).

Usage: tools/gen-texts.py /path/to/LightCreator
Every text must exist in LightCreator's table; a missing one is reported and stops the tool.
"""
import ast
import json
import os
import re
import sys

lc = sys.argv[1] if len(sys.argv) > 1 else "../LightCreator"
here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ui = os.path.join(here, "crates/base-ui/")
CODES = "bg ca hr cs da nl et fr el hu is ga lv lt mt nb pt ro sk sl es sv uk".split()


def literal(raw):
    return ast.literal_eval('"' + raw + '"')


def wanted():
    out = set()
    for root, _, files in os.walk(ui + "src"):
        for f in files:
            if not f.endswith(".rs") or f in ("i18n.rs", "texts.rs"):
                continue
            text = open(os.path.join(root, f)).read()
            for m in re.finditer(r'\btrf?\(\s*"((?:[^"\\]|\\.)*)"', text):
                out.add(literal(m.group(1)))
            if f == "mod.rs":
                for m in re.finditer(r'\(Tool::\w+,\s*"((?:[^"\\]|\\.)*)",\s*Key::', text):
                    out.add(literal(m.group(1)))
            if f == "menu.rs":
                for m in re.finditer(r'Item\(StdAct::[^,]*(?:\([^)]*\))?,\s*"((?:[^"\\]|\\.)*)"', text):
                    out.add(literal(m.group(1)))
                m = re.search(r'\[("Align left"[^\]]*)\]', text)
                for s in re.findall(r'"([^"]*)"', m.group(1)):
                    out.add(s)
    return sorted(out)


def lightcreator_rows():
    src = open(lc + "/crates/lightcreator/src/i18n.rs").read()
    a = src.index("const TABLE:")
    b = src.index("#[cfg(test)]\nmod tests {")
    rows = {}
    for line in src[a:b].split("\n"):
        line = line.strip()
        if line.startswith('["') and line.endswith("],"):
            try:
                r = ast.literal_eval(line[:-1])
            except Exception:
                continue
            if len(r) == 7:
                rows.setdefault(r[0], r)
    return rows


def lightcreator_languages():
    keys = {}
    for l in open(lc + "/assets/lang/keys.txt").read().split("\n"):
        if "\t" in l:
            i, t = l.split("\t", 1)
            keys[t] = int(i)
    langs = {}
    for c in CODES:
        d = {}
        for l in open(lc + "/assets/lang/%s.txt" % c).read().split("\n"):
            if "|" in l:
                i, t = l.split("|", 1)
                d[int(i)] = t
        langs[c] = d
    return keys, langs


def main():
    lits = wanted()
    rows = lightcreator_rows()
    keys, langs = lightcreator_languages()
    missing = [s for s in lits if s not in rows or s not in keys]
    if missing:
        sys.exit("not in LightCreator's table: %r" % missing)
    out = ui + "src/texts/"
    os.makedirs(out, exist_ok=True)
    open(out + "keys.txt", "w").write("".join("%d\t%s\n" % (i, s) for i, s in enumerate(lits)))
    for c in CODES:
        open(out + c + ".txt", "w").write("".join("%d|%s\n" % (i, langs[c][keys[s]]) for i, s in enumerate(lits)))
    lit = lambda s: json.dumps(s, ensure_ascii=False)
    table = "".join("    [%s],\n" % ", ".join(lit(x) for x in rows[s]) for s in lits)
    rs = open(ui + "src/texts.rs").read()
    head = rs[: rs.index("#[rustfmt::skip]")]
    tail = rs[rs.index("pub const KEYS"):]
    open(ui + "src/texts.rs", "w").write(head + "#[rustfmt::skip]\npub const TABLE: &[[&str; TABLE_LANGS]] = &[\n" + table + "];\n\n" + tail)
    print(len(lits), "texts")


main()
