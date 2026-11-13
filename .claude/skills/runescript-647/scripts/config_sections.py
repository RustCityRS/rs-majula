"""Print chosen sections of a cache config dump (all.npc, all.loc, all.obj ...), by name or by id.

py -3 -I config_sections.py <all.xxx> <name|id> [<name|id> ...] [--keys name,op1,op2,...]
Ids match the "// <id>" line above each section; names match "[name]". With --keys only those
keys are printed (prefix match, so "op" prints op1..op5)."""
import re
import sys

args = sys.argv[1:]
keys = None
if "--keys" in args:
    i = args.index("--keys")
    keys = args[i + 1].split(",")
    del args[i:i + 2]
path, wanted = args[0], set(args[1:])
cur_id, cur_name, buf = None, None, []


def flush():
    if cur_name is None:
        return
    if cur_name in wanted or (cur_id is not None and cur_id in wanted):
        lines = buf if keys is None else [l for l in buf if any(l.startswith(k) for k in keys)]
        print(f"// {cur_id} [{cur_name}] " + " | ".join(lines))


for line in open(path, encoding="utf-8", errors="replace"):
    line = line.rstrip("\n")
    m = re.match(r"// (\d+)$", line)
    if m:
        flush()
        cur_id, cur_name, buf = m.group(1), None, []
        continue
    m = re.match(r"\[(.+)\]$", line)
    if m:
        cur_name = m.group(1)
        continue
    if line and cur_name is not None:
        buf.append(line)
flush()
