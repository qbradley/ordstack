"""Summarise harness.py's JSON into Markdown tables and apply the reading
rule mechanically.

usage: python3 -I summarise.py RESULTS_JSON
"""
import json
import statistics
import sys


def q(xs):
    """(q1, median, q3, min, max) with inclusive quartiles."""
    xs = sorted(xs)
    q1, med, q3 = statistics.quantiles(xs, n=4, method="inclusive")
    return q1, med, q3, xs[0], xs[-1]


def main():
    r = json.load(open(sys.argv[1]))
    vs = r["variants"]
    print("Runs per (variant, input): %d timed after %d warm-up rounds, every run of a "
          "case pinned to one CPU. Re-run rule: worst variant's wall IQR > %.0f%% of its "
          "median → the case is re-run on a fresh core." % (
              r["runs"], r["warmup"], 100 * r.get("noise_iqr", 0)))
    print()
    summ = {}
    for name, c in r["cases"].items():
        print("### %s (`%s %s`)" % (name, c["program"], " ".join(
            a.split("/")[-1] for a in c["args"])))
        print()
        for i, a in enumerate(c.get("attempts", [])):
            ld = a["load"]
            print("- attempt %d: CPU %d; worst wall IQR %.1f%% of median; loadavg %s → %s; "
                  "chosen core busy before %s; SMT sibling busy during %s; machine busy "
                  "before %s%% / during %s%%%s" % (
                      i + 1, a["core"], 100 * a["max_rel_iqr"],
                      "/".join(ld["loadavg_before"]), "/".join(ld["loadavg_after"]),
                      ld["core_busy_before"], ld["siblings_busy_during"],
                      ld["system_busy_before"], ld["system_busy_during"],
                      " **(reported)**" if a["core"] == c["core"] and
                      abs(a["max_rel_iqr"] - c["max_rel_iqr"]) < 1e-12 else ""))
        print()
        src = ("`/usr/bin/time %%M`, %d runs" % len(c["rss_time_kb"]["std"])
               if "rss_time_kb" in c else "wait4 ru_maxrss, floored at harness RSS")
        print("| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | "
              "user+sys median ms | peak RSS median MiB (%s) | RSS min–max MiB | vs std | "
              "minor faults |" % src)
        print("|---|---|---|---|---|---|---|---|---|---|")
        s = {}
        for v in vs:
            runs = c["runs"][v]
            w = q([x["wall"] * 1e3 for x in runs])
            cpu = statistics.median([(x["user"] + x["sys"]) * 1e3 for x in runs])
            if "rss_time_kb" in c:  # /usr/bin/time pass (rss.py); see its docstring
                rss = q([kb / 1024 for kb in c["rss_time_kb"][v]])
            else:  # wait4 from the harness: floored at the harness's own RSS
                rss = q([x["maxrss_kb"] / 1024 for x in runs])
            flt = statistics.median([x["minflt"] for x in runs])
            s[v] = {"wall": w, "rss": rss, "cpu": cpu, "flt": flt}
        for v in vs:
            w, rss = s[v]["wall"], s[v]["rss"]
            print("| %s | %.1f | %.1f–%.1f | %.1f–%.1f | %+.1f%% | %.1f | %.1f | %.1f–%.1f | %+.1f%% | %d |" % (
                v, w[1], w[0], w[2], w[3], w[4], 100 * (w[1] / s["std"]["wall"][1] - 1),
                s[v]["cpu"], rss[1], rss[3], rss[4], 100 * (rss[1] / s["std"]["rss"][1] - 1),
                s[v]["flt"]))
        print()
        if c["runs"]["std"][0]["phases_us"]:
            print("In-program phase timer (medians, ms; `rest` = wall − the three phases: "
                  "process start, teardown, exit):")
            print()
            print("| variant | parse | transform | write | rest |")
            print("|---|---|---|---|---|")
            for v in vs:
                ph = {k: statistics.median([x["phases_us"][k] / 1e3 for x in c["runs"][v]])
                      for k in ("parse", "transform", "write")}
                rest = statistics.median([x["wall"] * 1e3 - sum(x["phases_us"].values()) / 1e3
                                          for x in c["runs"][v]])
                print("| %s | %.1f | %.1f | %.1f | %.1f |" % (
                    v, ph["parse"], ph["transform"], ph["write"], rest))
            print()
        if "perf" in c:
            print("perf stat (`%s`, %d rotated runs per variant on CPU %s; median, "
                  "min–max):" % (r["perf_events"], len(c["perf"]["std"]), c.get("perf_core")))
            print()
            keys = [k for k in ("instructions:u", "instructions", "cycles", "cycles:u")
                    if k in c["perf"]["std"][0]]
            print("| variant | " + " | ".join("%s (M) | min–max | vs std" % k for k in keys) + " |")
            print("|---|" + "---|---|---|" * len(keys))
            base = {k: statistics.median([p[k] for p in c["perf"]["std"]]) for k in keys}
            for v in vs:
                cells = []
                for k in keys:
                    xs = [p[k] for p in c["perf"][v]]
                    m = statistics.median(xs)
                    s[v][k] = m
                    cells.append("%.1f | %.1f–%.1f | %+.1f%%" % (
                        m / 1e6, min(xs) / 1e6, max(xs) / 1e6, 100 * (m / base[k] - 1)))
                print("| %s | %s |" % (v, " | ".join(cells)))
            print()
        summ[name] = s

    print("### Reading rule, applied mechanically")
    print()
    print("| input | ordstack time vs std | IQRs disjoint | time ≥10% better | "
          "ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo | "
          "instructions:u vs std / bumpalo | cycles vs std / bumpalo |" +
          (" time vs bumpalo-frozen | RSS vs bumpalo-frozen |" if "bumpalo-frozen" in vs else ""))
    print("|---|---|---|---|---|---|---|---|---|---|%s" % ("---|---|" if "bumpalo-frozen" in vs else ""))
    useful = {}
    for name, s in summ.items():
        o, st, b = s["ordstack"], s["std"], s["bumpalo"]
        dt = o["wall"][1] / st["wall"][1] - 1
        disjoint = o["wall"][2] < st["wall"][0]
        tb = dt <= -0.10 and disjoint
        dr = o["rss"][1] / st["rss"][1] - 1
        rb = dr <= -0.10
        bt = o["wall"][1] / b["wall"][1] - 1
        br = o["rss"][1] / b["rss"][1] - 1
        ck = "cycles" if "cycles" in o else "cycles:u"
        if "instructions:u" in o and ck in o:
            pc = "%+.1f%% / %+.1f%% | %+.1f%% / %+.1f%%" % (
                100 * (o["instructions:u"] / st["instructions:u"] - 1),
                100 * (o["instructions:u"] / b["instructions:u"] - 1),
                100 * (o[ck] / st[ck] - 1), 100 * (o[ck] / b[ck] - 1))
        else:
            pc = "n/a | n/a"
        if "bumpalo-frozen" in s:
            bf = s["bumpalo-frozen"]
            pc += " | %+.1f%% | %+.1f%%" % (100 * (o["wall"][1] / bf["wall"][1] - 1),
                                           100 * (o["rss"][1] / bf["rss"][1] - 1))
        print("| %s | %+.1f%% | %s | %s | %+.1f%% | %s | %+.1f%% | %+.1f%% | %s |" % (
            name, 100 * dt, "yes" if disjoint else "no", "yes" if tb else "no",
            100 * dr, "yes" if rb else "no", 100 * bt, 100 * br, pc))
        useful[name] = (tb, rb)
    print()
    for w, (a, b) in (("treebuild", ("treebuild-d7", "treebuild-d8")),
                      ("serde_json_xform", ("xform-120k", "xform-480k"))):
        if a in useful and b in useful:
            t = useful[a][0] and useful[b][0]
            m = useful[a][1] and useful[b][1]
            print("- %s: time criterion at both sizes: %s; RSS criterion at both sizes: %s"
                  % (w, "met" if t else "not met", "met" if m else "not met"))
    print()
    if r.get("stats"):
        print("### Arena statistics (ORDSTACK_STATS=1, one run)")
        print()
        for k, lines in r["stats"].items():
            print("- %s: %s" % (k, "; ".join(lines)))
        print()


main()
