#!/usr/bin/env python3
"""Regenerates the cache-derived tables of 647-summoning-inventory.md.

Run from the repo root:  python3 .claude/647-summoning-inventory.py > /tmp/tables.md
Reads only content/647/scripts/_unpack/all.* and content/647/maps/*.jm2.
"""
import glob
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "content", "647")
UNPACK = os.path.join(ROOT, "scripts", "_unpack")
MAPS = os.path.join(ROOT, "maps")


class Cfg:
    def __init__(self, cid, key):
        self.id, self.key, self.lines = cid, key, []

    def get(self, k, default=""):
        for line in self.lines:
            if line.startswith(k + "="):
                return line[len(k) + 1:]
        return default

    def all(self, k):
        return [line[len(k) + 1:] for line in self.lines if line.startswith(k + "=")]

    def param(self, n, default=""):
        vals = self.params(n)
        return vals[0] if vals else default

    def params(self, n):
        pre = "param=param_%s," % n
        return [line[len(pre):] for line in self.lines if line.startswith(pre)]

    def ops(self, prefix="op"):
        out = []
        for line in self.lines:
            m = re.match(r"^%s(\d)=(.*)$" % prefix, line)
            if m:
                out.append("%s:%s" % (m.group(1), m.group(2)))
        return ", ".join(sorted(out))


def load(name):
    by_id, by_key, cur, cid = {}, {}, None, None
    with open(os.path.join(UNPACK, "all." + name), encoding="utf-8", errors="replace") as f:
        for raw in f:
            line = raw.rstrip("\n")
            m = re.match(r"^// (\d+)$", line)
            if m:
                cid = int(m.group(1))
                continue
            if line.startswith("[") and line.endswith("]"):
                cur = Cfg(cid, line[1:-1])
                by_id[cid] = cur
                by_key[cur.key] = cur
                continue
            if cur is not None and line and not line.startswith("//"):
                cur.lines.append(line)
    return by_id, by_key


def enum(enums, n):
    out = {}
    for v in enums["enum_%d" % n].all("val"):
        k, _, val = v.partition(",")
        out[k] = val
    return out


def ranges(ids):
    ids = sorted(ids)
    out, i = [], 0
    while i < len(ids):
        j = i
        while j + 1 < len(ids) and ids[j + 1] == ids[j] + 1:
            j += 1
        out.append(str(ids[i]) if i == j else "%d-%d" % (ids[i], ids[j]))
        i = j + 1
    return ", ".join(out)


def placements(prefix, wanted):
    """name -> [(file, level, x, z)] for every map line whose config name passes `wanted`."""
    out = defaultdict(list)
    for path in sorted(glob.glob(os.path.join(MAPS, prefix + "*.jm2"))):
        m = re.match(r"^%s(\d+)_(\d+)\.jm2$" % prefix, os.path.basename(path))
        if not m:
            continue
        mx, mz = int(m.group(1)), int(m.group(2))
        with open(path, encoding="utf-8", errors="replace") as f:
            for line in f:
                lm = re.match(r"^(\d+) (\d+) (\d+): (\S+)", line)
                if lm and wanted(lm.group(4)):
                    out[lm.group(4)].append((os.path.basename(path), int(lm.group(1)),
                                             mx * 64 + int(lm.group(2)), mz * 64 + int(lm.group(3))))
    return out


def main():
    objs, objk = load("obj")
    npcs, npck = load("npc")
    locs, _ = load("loc")
    _, enums = load("enum")
    w = sys.stdout.write

    pouch_scroll, pouch_npc = enum(enums, 1283), enum(enums, 1320)

    # ---- pouches ----
    summon = [o for o in objs.values() if "iop4=Summon" in o.lines and not o.get("certtemplate")]
    standard = sorted((o for o in summon if o.params(538)), key=lambda o: int(o.param(394, "0")))
    w("### Standard pouches (%d)\n\n" % len(standard))
    w("| Lvl | Pouch obj | Name | Shards | Charm | Tertiary | Bogrog | Scroll | Familiar npc | Combat npc |\n")
    w("|---|---|---|---|---|---|---|---|---|---|\n")
    for o in standard:
        tert = " + ".join(t for t in (o.param(697), o.param(699)) if t)
        scroll = objk.get(pouch_scroll.get(o.key, ""))
        npc = npck.get(pouch_npc.get(o.key, ""))
        combat = npcs.get(npc.id + 1) if npc else None
        if combat is not None and (combat.get("name") != npc.get("name") or combat.get("vislevel", "0") == "0"):
            combat = None
        w("| %s | %d `%s` | %s | %s | `%s` | `%s` | %s | %s | %s | %s |\n" % (
            o.param(394), o.id, o.key, o.get("name"), o.param(541), o.param(542), tert, o.param(457),
            "%d `%s`" % (scroll.id, scroll.key) if scroll else "none",
            "%d `%s`" % (npc.id, npc.key) if npc else "none",
            "%d (cb %s)" % (combat.id, combat.get("vislevel")) if combat else "none"))
    other = sorted((o for o in summon if not o.params(538)), key=lambda o: o.id)
    w("\n### Other objs with `iop4=Summon` (%d)\n\n" % len(other))
    for o in other:
        w("- %d `%s` \"%s\" (level %s)\n" % (o.id, o.key, o.get("name"), o.param(394, "-")))

    # ---- scrolls ----
    scrolls = sorted({objk[s].id for s in pouch_scroll.values() if s in objk and s != "bones"})
    w("\n### Scrolls reachable from enum 1283 (%d)\n\n" % len(scrolls))
    w("| Obj | Name | Lvl | Bogrog shards (457) | Bogrog batch (599) |\n|---|---|---|---|---|\n")
    for sid in scrolls:
        o = objs[sid]
        w("| %d `%s` | %s | %s | %s | %s |\n" % (o.id, o.key, o.get("name"), o.param(394, "-"),
                                              o.param(457, "-"), o.param(599, "-")))

    # ---- familiar npcs ----
    fam = [n for n in npcs.values() if "familiar=yes" in n.lines]
    w("\n### NPCs with `familiar=yes` (%d)\n\n" % len(fam))
    groups = defaultdict(list)
    for n in fam:
        ops = n.ops()
        if "Pick-up" in ops:
            g = "pets (Pick-up op)"
        elif n.key.startswith("rand_"):
            g = "Daemonheim"
        elif 8240 <= n.id <= 8249:
            g = "Stealing Creation"
        elif not ops:
            g = "no ops"
        else:
            g = "familiars (Interact)"
        groups[g].append(n.id)
    for g in sorted(groups):
        w("- **%s**: %d — %s\n" % (g, len(groups[g]), ranges(groups[g])))

    w("\n### Standard familiar NPCs\n\n")
    w("| Lvl (394) | Npc | Name | Size | Ops | Combat npc | Combat lvl | BoB (1323) |\n|---|---|---|---|---|---|---|---|\n")
    for o in standard:
        n = npck.get(pouch_npc.get(o.key, ""))
        if not n:
            continue
        c = npcs.get(n.id + 1)
        if c is not None and (c.get("name") != n.get("name") or c.get("vislevel", "0") == "0"):
            c = None
        w("| %s | %d `%s` | %s | %s | %s | %s | %s | %s |\n" % (
            "/".join(n.params(394)) or "-", n.id, n.key, n.get("name"), n.get("size", "1"), n.ops(),
            "%d `%s`" % (c.id, c.key) if c else "none", c.get("vislevel") if c else "-", n.param(1323, "")))

    # ---- obelisks ----
    def is_obelisk(l):
        ops = l.ops()
        return "Infuse-pouch" in ops or "Renew-points" in ops
    ob = sorted((l for l in locs.values() if is_obelisk(l)), key=lambda l: l.id)
    names = {l.key for l in ob}
    placed = placements("l", lambda nm: nm in names)
    w("\n### Obelisk locs (%d configs with Infuse-pouch or Renew-points)\n\n" % len(ob))
    w("| Loc | Name | Ops | Placements |\n|---|---|---|---|\n")
    for l in ob:
        w("| %d `%s` | %s | %s | %d |\n" % (l.id, l.key, l.get("name"), l.ops(), len(placed.get(l.key, []))))
    for title, pred in (("Charged obelisk placements (overworld)", lambda l: l.key.startswith("wolf_obelisk")),
                        ("Small obelisk placements", lambda l: "mini_obelisk" in l.key)):
        rows = [(l, p) for l in ob if pred(l) for p in placed.get(l.key, [])]
        w("\n### %s (%d)\n\n| Loc | File | Level | x,z |\n|---|---|---|---|\n" % (title, len(rows)))
        for l, (fn, lvl, x, z) in sorted(rows, key=lambda r: (r[0].id, r[1][2], r[1][3], r[1][1])):
            w("| %d `%s` | %s | %d | %d,%d |\n" % (l.id, l.key, fn, lvl, x, z))

    # ---- skill npc spawns ----
    want = re.compile(r"piccup|^npc_6988$|bogrog|petshop_owner|pet_shop_owner|lore_penguin_keeper|puppy_pen|giant_wolpertinger")
    spawns = placements("n", lambda nm: bool(want.search(nm)))
    w("\n### Skill NPC spawns\n\n| Npc | File | Level | x,z |\n|---|---|---|---|\n")
    for nm in sorted(spawns, key=lambda k: npck[k].id if k in npck else 0):
        for fn, lvl, x, z in spawns[nm]:
            w("| %s `%s` | %s | %d | %d,%d |\n" % (npck[nm].id if nm in npck else "?", nm, fn, lvl, x, z))


if __name__ == "__main__":
    main()
