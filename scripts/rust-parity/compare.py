#!/usr/bin/env python3
"""Compare a Rust dump and a Cyrius dump of the same cases, value by value.

Usage: compare.py rust.out cyrius.out [cases.cyr]
Prints one line per case that differs (values, how many differ, the largest ULP gap, and
which arrays), then a TOTAL line. A case whose differing arrays include 'signal' differs
in its Gaussian source, which comes from exp (see ADR-0010), not from the mesh.
"""
import sys


def load(path):
    toks = open(path).read().split()
    out, order, i = {}, [], 0
    while i < len(toks):
        assert toks[i] == '#', (path, i, toks[i])
        cid, label, idx, n = int(toks[i + 1]), toks[i + 2], int(toks[i + 3]), int(toks[i + 4])
        key = (cid, label, idx)
        out[key] = [int(t) for t in toks[i + 5:i + 5 + n]]
        order.append(key)
        i += 5 + n
    return out, order


def line_pos(u):
    # f32 bits onto a monotone integer line, so the difference is a ULP count
    return -(u & 0x7FFFFFFF) if u & 0x80000000 else u


def is_nan(u):
    return (u & 0x7F800000) == 0x7F800000 and (u & 0x7FFFFF) != 0


def kinds(path):
    out, cur = {}, None
    for line in open(path):
        t = line.strip()
        if t.startswith('fn case_'):
            cur = int(t.split('_')[1].split('(')[0])
        elif t.startswith('# ') and cur is not None and cur not in out:
            out[cur] = t[2:]
    return out


def main():
    a, order = load(sys.argv[1])
    b, _ = load(sys.argv[2])
    kind = kinds(sys.argv[3]) if len(sys.argv) > 3 else {}
    summary = {}
    for key in order:
        s = summary.setdefault(key[0], {'n': 0, 'diff': 0, 'ulp': 0, 'shape': '', 'labels': set()})
        if key not in b:
            s['shape'] += ' missing:%s%d' % key[1:]
            continue
        x, y = a[key], b[key]
        if len(x) != len(y):
            s['shape'] += ' len(%s%d):%d/%d' % (key[1], key[2], len(x), len(y))
        for u, v in zip(x, y):
            s['n'] += 1
            if u != v and not (is_nan(u) and is_nan(v)):
                s['diff'] += 1
                s['labels'].add(key[1])
                s['ulp'] = max(s['ulp'], abs(line_pos(u) - line_pos(v)))
    for key in b:
        if key not in a:
            summary.setdefault(key[0], {'n': 0, 'diff': 0, 'ulp': 0, 'shape': '', 'labels': set()})['shape'] += ' extra:%s%d' % key[1:]
    tot_n = tot_d = exact = from_exp = 0
    for cid in sorted(summary):
        s = summary[cid]
        tot_n += s['n']
        tot_d += s['diff']
        ok = s['diff'] == 0 and not s['shape']
        exact += ok
        if not ok:
            from_exp += 'signal' in s['labels']
            print('case %3d %-16s values %6d differ %6d maxulp %-10d %s %s' % (
                cid, kind.get(cid, ''), s['n'], s['diff'], s['ulp'], ','.join(sorted(s['labels'])), s['shape']))
    print('TOTAL cases %d bit-identical %d values %d differing %d (cases with a differing exp-made source: %d)'
          % (len(summary), exact, tot_n, tot_d, from_exp))


main()
