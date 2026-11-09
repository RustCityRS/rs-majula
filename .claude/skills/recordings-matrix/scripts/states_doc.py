"""Render a state matrix (state_matrix.py output) as one readable page: every canonical line in story
order with how many recordings show it, the earliest dated sighting (video id @ seconds, to open at
that moment), and where players went next (each exit with its count).

py -3 -I states_doc.py <canon.tsv> <matrix_out_dir> <title> > STATES.md"""
import os
import sys
from collections import defaultdict

canon_path, out, title = sys.argv[1], sys.argv[2], sys.argv[3]
canon = []
for line in open(canon_path, encoding="utf-8"):
    if line.startswith("#") or not line.strip():
        continue
    cid, kind, speaker, text = line.rstrip("\n").split("\t")
    canon.append((cid, kind, speaker, text))
first = defaultdict(dict)
videos = []
for line in open(os.path.join(out, "states.tsv"), encoding="utf-8"):
    label, t, c = line.split("\t")[:3]
    if label not in videos:
        videos.append(label)
    if c != "?":
        first[c].setdefault(label, float(t))
exits = defaultdict(list)
for line in open(os.path.join(out, "edges.md"), encoding="utf-8").readlines()[2:]:
    p = [x.strip() for x in line.strip().strip("|").split("|")]
    exits[p[0]].append((int(p[2]), p[1]))
text = {c[0]: c[3] for c in canon}

print(f"# {title}\n")
print(f"{len(videos)} recordings, {videos[0][:8]} to {sorted(videos)[-1][:8]} (upload dates). "
      "Each line: how many recordings show it, the first one by date (YouTube id @ seconds), and "
      "the states players went to next (count). Exits seen once are often a missed box, not a "
      "branch.\n")
for cid, kind, speaker, t in canon:
    seen = first.get(cid, {})
    who = f"**{speaker}**: " if speaker else ""
    src = ""
    if seen:
        v = min(seen)
        src = f" (first: {v[:8]} `{v[9:]}` @{seen[v]:.0f}s)"
    print(f"- `{cid}` {kind} {len(seen)}/{len(videos)}{src}: {who}{t}")
    ex = sorted(exits.get(cid, []), reverse=True)
    if len(ex) > 1:
        print("  - next: " + "; ".join(f"`{b}` x{n}" for n, b in ex if n >= 2)
              + (f"; once: {', '.join(f'`{b}`' for n, b in ex if n < 2)}" if any(n < 2 for n, _ in ex) else ""))
