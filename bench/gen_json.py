"""Generate serde_json_xform's input: the generator from
malloc-stacks/stackfit_spike/README.md (run_rust.sh's gen_json()), seed 1,
with the record count as a parameter.

usage: python3 -I gen_json.py N OUT_PATH
"""
import json
import random
import sys


def main():
    n, path = int(sys.argv[1]), sys.argv[2]
    random.seed(1)
    recs = [{"id": i, "x": random.random()*10, "name": "n%d" % random.randrange(5000),
             "tags": [random.choice("abcdefgh") for _ in range(random.randrange(1, 6))],
             "nested": {"a": i % 7, "b": [i, i+1]}} for i in range(n)]
    json.dump(recs, open(path, "w"))


main()
