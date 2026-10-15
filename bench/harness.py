"""Benchmark harness for bench/. Run by run.sh on the train box.

usage: python3 -I harness.py BINDIR WORKDIR OUT_JSON

Environment: RUNS (timed runs per variant and input, default 25), WARMUP
(default 3), CORE (CPU the programs are pinned to; unset = before each case
pick the CPU whose physical core was least busy over 3 s, never CPU 0 or its
SMT sibling), PERF_RUNS (default 7; 0 disables perf), NOISE_IQR and RETRIES
(re-run rule, see main), CASES (comma list of case names, default all).
Machine load (loadavg, busy % of the chosen core's SMT siblings and of the
whole machine) is recorded before and during every case.

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
4. One untimed run of both ordstack and both bumpalo ports with ORDSTACK_STATS=1
   for arena statistics.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

VARIANTS = ["std", "std-mimalloc", "std-control", "ordstack", "ordstack-alloc", "bumpalo",
            "bumpalo-frozen"]


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
    # instructions:u (user mode only) is the most deterministic count; the
    # unqualified events also count kernel work (page faults, munmap) done on
    # the program's behalf.
    for ev in ("instructions:u,instructions,cycles", "instructions:u,cycles:u"):
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
            res[parts[2]] = int(float(parts[0]))
    os.unlink(path)
    return res


def cpu_times():
    """Per-CPU (busy, total) jiffies from /proc/stat."""
    res = {}
    for line in open("/proc/stat"):
        p = line.split()
        if p[0].startswith("cpu") and p[0] != "cpu":
            v = list(map(int, p[1:]))
            idle = v[3] + v[4]  # idle + iowait
            res[int(p[0][3:])] = (sum(v) - idle, sum(v))
    return res


def busy_pct(a, b):
    return {c: 100.0 * (b[c][0] - a[c][0]) / max(1, b[c][1] - a[c][1]) for c in a if c in b}


def siblings(c):
    try:
        txt = open("/sys/devices/system/cpu/cpu%d/topology/thread_siblings_list" % c).read()
    except OSError:
        return {c}
    out = set()
    for part in txt.strip().split(","):
        lo, _, hi = part.partition("-")
        out.update(range(int(lo), int(hi or lo) + 1))
    return out


def pick_core(sample_s=3.0):
    """The CPU whose physical core (it and its SMT siblings) was least busy
    over `sample_s` seconds, excluding CPU 0 and its siblings. Returns
    (cpu, {cpu: busy%} of its core, system-wide busy%)."""
    a = cpu_times()
    time.sleep(sample_s)
    b = cpu_times()
    pct = busy_pct(a, b)
    avoid = siblings(0)
    best = None
    for c in sorted(pct):
        if c in avoid:
            continue
        sib = siblings(c)
        load = max(pct.get(s, 100.0) for s in sib)
        if best is None or load < best[0]:
            best = (load, c, {s: round(pct.get(s, 0), 1) for s in sorted(sib)})
    allp = round(sum(pct.values()) / len(pct), 1)
    return best[1], best[2], allp


def loadavg():
    return open("/proc/loadavg").read().split()[:3]


def rel_iqr(xs):
    import statistics
    q1, _, q3 = statistics.quantiles(sorted(xs), n=4, method="inclusive")
    return (q3 - q1) / statistics.median(xs)


def timed_case(name, prog, args, bindir, ref, runs, warm, env, tmp, core_env):
    """One attempt at a case: pick a core (unless CORE is set), warm up, then
    `runs` rotated rounds. Records machine load before and after."""
    if core_env is not None:
        core, core_busy, sys_busy = int(core_env), None, None
    else:
        core, core_busy, sys_busy = pick_core()
    load = {"loadavg_before": loadavg(), "core_busy_before": core_busy,
            "system_busy_before": sys_busy}
    sibs = siblings(core)
    bins = {v: os.path.join(bindir, "%s-%s" % (prog, v)) for v in VARIANTS}
    rec = {v: [] for v in VARIANTS}
    for _ in range(warm):
        for v in VARIANTS:
            run([bins[v]] + args(v), core=core, env=env, tmp=tmp)
    t_a = cpu_times()
    for i in range(runs):
        k = i % len(VARIANTS)
        for v in VARIANTS[k:] + VARIANTS[:k]:
            wall, ru, out, err = run([bins[v]] + args(v), core=core, env=env, tmp=tmp)
            if out != ref[name]:
                sys.exit("FAIL: %s %s stdout changed in a timed run" % (name, v))
            rec[v].append({"wall": wall, "maxrss_kb": ru.ru_maxrss, "user": ru.ru_utime,
                           "sys": ru.ru_stime, "minflt": ru.ru_minflt,
                           "phases_us": parse_phases(err)})
    pct = busy_pct(t_a, cpu_times())
    # During the runs our own process keeps `core` busy; its SMT siblings
    # and the system-wide figure show the external load.
    load.update({"loadavg_after": loadavg(),
                 "siblings_busy_during": {s: round(pct[s], 1) for s in sorted(sibs) if s != core},
                 "system_busy_during": round(sum(pct.values()) / len(pct), 1)})
    worst = max(rel_iqr([x["wall"] for x in rec[v]]) for v in VARIANTS)
    return {"program": prog, "args": args("std"), "core": core, "load": load,
            "max_rel_iqr": worst, "runs": rec}


def main():
    bindir, work, out_json = sys.argv[1:4]
    runs = int(os.environ.get("RUNS", "25"))
    warm = int(os.environ.get("WARMUP", "3"))
    core_env = os.environ.get("CORE")  # unset: pick an idle core per attempt
    perf_runs = int(os.environ.get("PERF_RUNS", "7"))
    # Re-run rule, fixed before any run: if any variant's wall-time IQR is
    # wider than NOISE_IQR of its median, the whole case (all variants) is
    # re-run on a freshly chosen core, up to RETRIES more times; the attempt
    # with the smallest worst-variant IQR is reported and every attempt is
    # kept in the JSON.
    noise = float(os.environ.get("NOISE_IQR", "0.04"))
    retries = int(os.environ.get("RETRIES", "2"))
    only = os.environ.get("CASES")
    shm = "/dev/shm" if os.path.isdir("/dev/shm") else None
    tmp = tempfile.mkdtemp(prefix="ordstack-bench-", dir=shm)
    outdir = tmp
    cs = cases(work, outdir)
    if only:
        cs = [c for c in cs if c[0] in only.split(",")]
    env = {"XFORM_PHASES": "1"}

    ref = verify(bindir, cs, outdir, tmp)
    result = {"runs": runs, "warmup": warm, "variants": VARIANTS, "noise_iqr": noise,
              "cases": {}, "perf_events": None, "stats": {}}
    for name, prog, args in cs:
        attempts = []
        while True:
            a = timed_case(name, prog, args, bindir, ref, runs, warm, env, tmp, core_env)
            attempts.append(a)
            print("timed %s attempt %d: core %d, worst rel IQR %.1f%%, load %s"
                  % (name, len(attempts), a["core"], 100 * a["max_rel_iqr"], a["load"]),
                  flush=True)
            if a["max_rel_iqr"] <= noise or len(attempts) > retries:
                break
        best = min(attempts, key=lambda x: x["max_rel_iqr"])
        best = dict(best)
        best["attempts"] = [{k: v for k, v in x.items() if k != "runs"} for x in attempts]
        best["all_attempt_runs"] = [x["runs"] for x in attempts]
        result["cases"][name] = best

    ev = perf_ok() if perf_runs > 0 else None
    result["perf_events"] = ev
    if ev:
        for name, prog, args in cs:
            core = int(core_env) if core_env is not None else pick_core()[0]
            pr = {v: [] for v in VARIANTS}
            for i in range(perf_runs):  # rotated rounds, as for the timed runs
                k = i % len(VARIANTS)
                for v in VARIANTS[k:] + VARIANTS[:k]:
                    cmd = [os.path.join(bindir, "%s-%s" % (prog, v))] + args(v)
                    pr[v].append(perf_run(cmd, ev, core, tmp, env))
            result["cases"][name]["perf"] = pr
            result["cases"][name]["perf_core"] = core
        print("perf done (%s)" % ev, flush=True)
    else:
        print("perf: not available or not permitted; skipped", flush=True)

    for name, prog, args in cs:
        for v in ("ordstack", "ordstack-alloc", "bumpalo", "bumpalo-frozen"):
            _, _, _, err = run([os.path.join(bindir, "%s-%s" % (prog, v))] + args(v),
                               env={"ORDSTACK_STATS": "1"}, tmp=tmp)
            result["stats"]["%s/%s" % (name, v)] = [
                l for l in err.splitlines() if not l.startswith("phases_us")]

    shutil.rmtree(tmp)
    with open(out_json, "w") as f:
        json.dump(result, f, indent=1)


main()
