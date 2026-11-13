"""Forced line breaks in dialogue (2011's <br>), from where recordings break each canonical line.

Each OCR'd sighting (high-resolution recordings only) is split into its text lines; each line end
is mapped to a word of the canonical text. A break after word k is FORCED when the line so far plus
the next word still fits the box's wrap width: the client would not have wrapped there. Breaks
that votes agree on are printed as the line with <br> inserted.

py -3 -I br_check.py <fontmetrics> <canon.tsv> <states.tsv> <videos (comma list)>
    [--chat 380] [--obj 380] [--mes 470]"""
import difflib
import re
import sys
from collections import Counter, defaultdict

a = sys.argv[1:]
widths = {"--chat": 380, "--obj": 380, "--mes": 470}
for flag in list(widths):
    if flag in a:
        i = a.index(flag)
        widths[flag] = int(a[i + 1])
        del a[i:i + 2]
adv = [int(v) for v in [l for l in open(a[0], encoding="utf-8") if l.startswith("glyph_widths=")][0]
       .split("=", 1)[1].split(",")]
use = set(a[3].split(","))


def width(s):
    return sum(adv[b] for b in s.encode("cp1252", "replace"))


def norm(s):
    return re.sub(r"[^a-z0-9]", "", s.lower())


canon = {}
for line in open(a[1], encoding="utf-8"):
    if line.startswith("#") or not line.strip():
        continue
    cid, kind, speaker, text = line.rstrip("\n").split("\t")
    canon[cid] = (kind, speaker, text)
votes = defaultdict(Counter)
sightings = Counter()
for line in open(a[2], encoding="utf-8"):
    p = line.rstrip("\n").split("\t")
    if len(p) < 5 or p[0] not in use or p[2] not in canon:
        continue
    kind, speaker, text = canon[p[2]]
    if kind not in ("chatnpc", "chatplayer", "mesbox", "objbox"):
        continue
    segs = [s.strip() for s in p[4].split(" / ") if s.strip()
            and "click here" not in s.lower() and "please wait" not in s.lower()]
    if speaker and segs and (difflib.SequenceMatcher(None, norm(segs[0]), norm(speaker)).ratio() > 0.6
                             or (kind == "chatplayer" and len(segs[0].split()) <= 3 and len(segs) > 1)):
        segs = segs[1:]
    if not segs:
        continue
    words = text.split(" ")
    # cumulative normalised length at the end of each word
    cum, total = [], 0
    for w_ in words:
        total += len(norm(w_))
        cum.append(total)
    ocr_total = sum(len(norm(s)) for s in segs)
    if ocr_total == 0 or abs(ocr_total - total) > max(4, total * 0.15):
        continue
    sightings[p[2]] += 1
    acc = 0
    for s in segs[:-1]:
        acc += len(norm(s))
        target = acc * total / ocr_total
        k = min(range(len(cum)), key=lambda j: abs(cum[j] - target))
        votes[p[2]][k] += 1
W = {"chatnpc": widths["--chat"], "chatplayer": widths["--chat"], "objbox": widths["--obj"],
     "mesbox": widths["--mes"]}
for cid, (kind, speaker, text) in canon.items():
    n = sightings[cid]
    if not n:
        continue
    words = text.split(" ")
    breaks = sorted(k for k, c in votes[cid].items() if c * 2 > n)
    forced, start = [], 0
    for k in breaks:
        so_far = " ".join(words[start:k + 1])
        if k + 1 < len(words) and width(so_far + " " + words[k + 1]) <= W[kind]:
            forced.append(k)
        start = k + 1
    if forced:
        out = []
        for i, w_ in enumerate(words):
            out.append(w_ + ("<br>" if i in forced else " "))
        print(f"{cid}\t{n}\t{''.join(out).strip()}")
    else:
        print(f"{cid}\t{n}\t(natural)")
