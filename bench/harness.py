"""Benchmark harness for bench/. Run by run.sh on the train box.

usage: python3 -I harness.py BINDIR WORKDIR OUT_JSON

Environment: RUNS (timed runs per variant and input, default 25), WARMUP
(default 3), CORE (CPU the programs are pinned to, default 11), PERF_RUNS
(default 5; 0 disables perf), CASES (comma list of case names, default all).

1. Checks every variant against `std`: byte-identical stdout, and for
   serde_json_xform a byte-identical output file. Exits 1 on any mismatch.
2. Timed runs: per input, WARMUP untimed rounds, then RUNS rounds; each
   round runs every variant once, in an order rotated by one each round, so
   drift hits all variants equally. Each run is pinned to CORE (the child
   calls sched_setaffinity before exec, which is what `taskset -c` does).
   Wall time is measured around spawn + wait4; peak RSS, user/sys time and
   minor faults come from the child's rusage (wait4). Every timed run's
   stdout is checked again.
3. perf stat (instructions, cycles), PERF_RUNS runs per variant and input,
   if perf works here; skipped cleanly otherwise.
4. One untimed run of the ordstack and bumpalo ports with ORDSTACK_STATS=1
   for arena statistics.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

VARIANTS = ["std", "std-mimalloc", "ordstack", "bumpalo"]


def cases(work, outdir):
    def xf(inp):
        return lambda v: [os.path.join(work, inp), os.path.join(outdir, "out_%s.json" % v)]
    return [
        ("treebuild-d7", "treebuild", lambda v: ["7"]),
        ("treebuild-d8", "treebuild", lambda v: ["8"]),
        ("xform-120k", "serde_json_xform", xf("big.json")),
        ("xform-480k", "serde_json_xform", xf("big4.json")),
    ]


def run(cmd, core=None, env=None, tmp=None):
    """Runs cmd; returns (wall_s, rusage, stdout_bytes, stderr_text)."""
    so = tempfile.TemporaryFile(dir=tmp)
    se = tempfile.TemporaryFile(dir=tmp)
    pre = (lambda: os.sched_setaffinity(0, {core})) if core is not None else None
    full_env = dict(os.environ)
    full_env.update(env or {})
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, stdout=so, stderr=se, stdin=subprocess.DEVNULL,
                         preexec_fn=pre, env=full_env)
    _, status, ru = os.wait4(p.pid, 0)
    t1 = time.perf_counter()
    p.returncode = os.waitstatus_to_exitcode(status)
    so.seek(0)
    se.seek(0)
    out, err = so.read(), se.read().decode(errors="replace")
    if p.returncode != 0:
        sys.exit("FAIL: %s exited %d\n%s" % (cmd, p.returncode, err))
    return t1 - t0, ru, out, err


def parse_phases(err):
    for line in err.splitlines():
        if line.startswith("phases_us "):
            return {k: int(v) for k, v in (kv.split("=") for kv in line.split()[1:])}
    return None


def file_bytes(path):
    with open(path, "rb") as f:
        return f.read()


def verify(bindir, cs, outdir, tmp):
    ref = {}
    for name, prog, args in cs:
        for v in VARIANTS:
            _, _, out, _ = run([os.path.join(bindir, "%s-%s" % (prog, v))] + args(v), tmp=tmp)
            if v == "std":
                ref[name] = out
                print("verify %s: std stdout %r" % (name, out.decode().strip()), flush=True)
            elif out != ref[name]:
                sys.exit("FAIL: %s %s stdout differs from std: %r vs %r" % (name, v, out, ref[name]))
            if prog == "serde_json_xform":
                a = file_bytes(args("std")[1])
                b = file_bytes(args(v)[1])
                if a != b:
                    sys.exit("FAIL: %s %s output file differs from std" % (name, v))
        if prog == "serde_json_xform":
            print("verify %s: output files identical (%d bytes)" % (name, len(a)), flush=True)
    return ref


def perf_ok():
    if shutil.which("perf") is None:
        return None
    for ev in ("instructions,cycles", "instructions:u,cycles:u"):
        r = subprocess.run(["perf", "stat", "-x,", "-e", ev, "true"],
                           capture_output=True, text=True)
        if r.returncode == 0 and "<not supported>" not in r.stderr and "<not counted>" not in r.stderr:
            return ev
    return None


def perf_run(cmd, ev, core, tmp, env):
    fd, path = tempfile.mkstemp(dir=tmp)
    os.close(fd)
    run(["perf", "stat", "-x,", "-o", path, "-e", ev, "--"] + cmd, core=core, tmp=tmp, env=env)
    res = {}
    for line in open(path):
        parts = line.strip().split(",")
        if len(parts) > 2 and parts[0].replace(".", "").isdigit():
            res[parts[2].split(":")[0]] = int(float(parts[0]))
    os.unlink(path)
    return res


def main():
    bindir, work, out_json = sys.argv[1:4]
    runs = int(os.environ.get("RUNS", "25"))
    warm = int(os.environ.get("WARMUP", "3"))
    core = int(os.environ.get("CORE", "11"))
    perf_runs = int(os.environ.get("PERF_RUNS", "5"))
    only = os.environ.get("CASES")
    shm = "/dev/shm" if os.path.isdir("/dev/shm") else None
    tmp = tempfile.mkdtemp(prefix="ordstack-bench-", dir=shm)
    outdir = tmp
    cs = cases(work, outdir)
    if only:
        cs = [c for c in cs if c[0] in only.split(",")]
    env = {"XFORM_PHASES": "1"}

    ref = verify(bindir, cs, outdir, tmp)
    result = {"runs": runs, "warmup": warm, "core": core, "variants": VARIANTS,
              "cases": {}, "perf_events": None, "stats": {}}
    for name, prog, args in cs:
        bins = {v: os.path.join(bindir, "%s-%s" % (prog, v)) for v in VARIANTS}
        rec = {v: [] for v in VARIANTS}
        for _ in range(warm):
            for v in VARIANTS:
                run([bins[v]] + args(v), core=core, env=env, tmp=tmp)
        for i in range(runs):
            k = i % len(VARIANTS)
            for v in VARIANTS[k:] + VARIANTS[:k]:
                wall, ru, out, err = run([bins[v]] + args(v), core=core, env=env, tmp=tmp)
                if out != ref[name]:
                    sys.exit("FAIL: %s %s stdout changed in a timed run" % (name, v))
                rec[v].append({"wall": wall, "maxrss_kb": ru.ru_maxrss, "user": ru.ru_utime,
                               "sys": ru.ru_stime, "minflt": ru.ru_minflt,
                               "phases_us": parse_phases(err)})
        result["cases"][name] = {"program": prog, "args": args("std"), "runs": rec}
        print("timed %s done" % name, flush=True)

    ev = perf_ok() if perf_runs > 0 else None
    result["perf_events"] = ev
    if ev:
        for name, prog, args in cs:
            pr = {}
            for v in VARIANTS:
                cmd = [os.path.join(bindir, "%s-%s" % (prog, v))] + args(v)
                pr[v] = [perf_run(cmd, ev, core, tmp, env) for _ in range(perf_runs)]
            result["cases"][name]["perf"] = pr
        print("perf done (%s)" % ev, flush=True)
    else:
        print("perf: not available or not permitted; skipped", flush=True)

    for name, prog, args in cs:
        for v in ("ordstack", "bumpalo"):
            _, _, _, err = run([os.path.join(bindir, "%s-%s" % (prog, v))] + args(v),
                               env={"ORDSTACK_STATS": "1"}, tmp=tmp)
            result["stats"]["%s/%s" % (name, v)] = [
                l for l in err.splitlines() if not l.startswith("phases_us")]

    shutil.rmtree(tmp)
    with open(out_json, "w") as f:
        json.dump(result, f, indent=1)


main()
