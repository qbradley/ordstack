"""Peak RSS pass, added to an existing harness.py result file.

usage: python3 -I rss.py BINDIR WORKDIR RESULTS_JSON

Why a separate pass: `ru_maxrss` from wait4 on a child that Python forked
includes the forked interpreter's resident set before `exec`, so the
harness's own RSS (about 20 MB) is a floor under every reading. Here each
program runs under `/usr/bin/time -f %M` (itself forked from Python, but
time's own resident set is about 2 MB), RSS_RUNS (default 5) rotated rounds
per variant, pinned to one CPU. The JSON gains `rss_time_kb` per case.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile

VARIANTS = ["std", "std-mimalloc", "std-control", "ordstack", "ordstack-alloc", "alloc-control",
            "bumpalo", "bumpalo-frozen"]


def main():
    bindir, work, path = sys.argv[1:4]
    n = int(os.environ.get("RSS_RUNS", "5"))
    r = json.load(open(path))
    tmp = tempfile.mkdtemp(prefix="ordstack-rss-", dir="/dev/shm" if os.path.isdir("/dev/shm") else None)
    for name, c in r["cases"].items():
        core = c["core"]
        prog = c["program"]
        res = {v: [] for v in VARIANTS}
        for i in range(n):
            k = i % len(VARIANTS)
            for v in VARIANTS[k:] + VARIANTS[:k]:
                args = list(c["args"])
                if prog == "serde_json_xform":
                    args[1] = os.path.join(tmp, "out.json")
                    args[0] = os.path.join(work, os.path.basename(args[0]))
                out = os.path.join(tmp, "rss")
                subprocess.run(["/usr/bin/time", "-f", "%M", "-o", out,
                                os.path.join(bindir, "%s-%s" % (prog, v))] + args,
                               stdout=subprocess.DEVNULL, check=True,
                               preexec_fn=lambda: os.sched_setaffinity(0, {core}))
                res[v].append(int(open(out).read().split()[-1]))
        c["rss_time_kb"] = res
        print("rss %s: %s" % (name, {v: sorted(x) for v, x in res.items()}), flush=True)
    shutil.rmtree(tmp)
    # Written beside the original and renamed over it, so an interrupted
    # write cannot destroy the timed results.
    with open(path + ".tmp", "w") as f:
        json.dump(r, f, indent=1)
    os.replace(path + ".tmp", path)


main()
