"""Which npcs and locs change with a given set of vars: every config section whose multivar/multivarbit
names one of them, with its forms (multinpc/multiloc lists), resolved to names and ops.

py -3 -I multivar_users.py <_unpack dir> <var name> [<var name> ...]
Prints per var: each npc/loc that switches on it, and per value the form's name and ops."""
import os
import re
import sys

unpack, wanted = sys.argv[1], set(sys.argv[2:])


def sections(path):
    cur, sid, body = None, None, {}
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        m = re.match(r"// (\d+)$", line)
        if m:
            if cur:
                yield sid, cur, body
            sid, cur, body = m.group(1), None, {}
            continue
        m = re.match(r"\[(.+)\]$", line)
        if m:
            cur = m.group(1)
            continue
        if cur and "=" in line:
            k, v = line.split("=", 1)
            body.setdefault(k, []).append(v)
    if cur:
        yield sid, cur, body


for kind in ("npc", "loc"):
    path = os.path.join(unpack, f"all.{kind}")
    by_name = {}
    users = []
    for sid, name, body in sections(path):
        by_name[name] = body
        var = (body.get("multivar") or body.get("multivarbit") or [None])[0]
        if var in wanted:
            users.append((var, sid, name, body))
    for var, sid, name, body in sorted(users):
        print(f"{var}: {kind} {sid} [{name}]")
        forms = body.get(f"multi{kind}", [])
        for f in forms:
            val, _, target = f.partition(",")
            t = by_name.get(target, {})
            ops = [f"{k}={v[0]}" for k, v in sorted(t.items()) if re.match(r"op\d$", k)]
            nm = t.get("name", ["(no name)"])[0]
            print(f"    {val}: {target} '{nm}' {' '.join(ops)}")
        for k in ("multidefault",):
            if k in body:
                print(f"    default: {body[k][0]}")
