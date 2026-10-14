"""Source cost of each port against its original.

usage: python3 -I srcstats.py   (from bench/)

Lines marked BENCH (a trailing `// BENCH`, or a `// BENCH{` ... `// BENCH}`
block) are instrumentation and are dropped from every file first, so the
original's phase timer does not count either. "Code lines" are non-blank
lines that are not comments. "Lifetime parameters declared" counts the
lifetimes in the generic parameter lists of `struct`, `enum`, `fn` and
`impl` headers (an `impl<'de, 'd>` header counts 2). A diff of code lines
(difflib) gives lines added and removed against the original.
"""
import difflib
import re
import subprocess

WORKLOADS = {
    "treebuild": ["orig.rs", "bin/ordstack.rs", "bin/bumpalo.rs"],
    "serde_json_xform": ["orig.rs", "bin/ordstack.rs", "bin/bumpalo.rs"],
}
DECL = re.compile(r"\b(?:struct|enum|fn|impl)\b\s*(?:\w+\s*)?<([^>{(]*)>")
LT = re.compile(r"'(?:[a-z_][a-z0-9_]*)\b(?!')")


def strip_bench(lines):
    out, skip = [], False
    for ln in lines:
        s = ln.strip()
        if s.startswith("// BENCH{"):
            skip = True
            continue
        if s.startswith("// BENCH}"):
            skip = False
            continue
        if skip or s.endswith("// BENCH"):
            continue
        out.append(ln)
    return out


def rustfmt(text):
    r = subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout"],
                       input=text, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else text


def stats(path):
    lines = strip_bench(open(path).read().splitlines())
    text = rustfmt("\n".join(lines) + "\n")
    all_lines = text.splitlines()
    code = [l.strip() for l in all_lines if l.strip() and not l.strip().startswith("//")]
    decl = sum(len(LT.findall(m.group(1))) for m in DECL.finditer(text))
    tokens = len(LT.findall(" ".join(code)))
    port = sum(1 for l in all_lines if "PORT" in l)
    return {"lines": len(all_lines), "code": code, "decl": decl, "tokens": tokens, "port": port}


def main():
    print("| workload | file | lines (rustfmt) | code lines | added / removed vs original "
          "(code lines) | lifetime params declared | lifetime tokens | PORT comments |")
    print("|---|---|---|---|---|---|---|---|")
    for w, files in WORKLOADS.items():
        base = None
        for f in files:
            s = stats("%s/src/%s" % (w, f))
            if base is None:
                base = s
                diff = "—"
            else:
                sm = difflib.SequenceMatcher(None, base["code"], s["code"], autojunk=False)
                ops = [o for o in sm.get_opcodes() if o[0] != "equal"]
                add = sum(j2 - j1 for _, _, _, j1, j2 in ops)
                rem = sum(i2 - i1 for _, i1, i2, _, _ in ops)
                diff = "+%d / −%d" % (add, rem)
            print("| %s | %s | %d | %d (%+.0f%%) | %s | %d | %d | %d |" % (
                w, f, s["lines"], len(s["code"]), 100 * (len(s["code"]) / len(base["code"]) - 1),
                diff, s["decl"], s["tokens"], s["port"]))


main()
