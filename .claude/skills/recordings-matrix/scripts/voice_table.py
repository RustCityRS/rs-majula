"""The voiced script as cache 786 stores it: every vorbis-output enum (one per speaker, lines in
alphabetical order, one clip per line), each entry with its clip's transcript and, where a
recording showed the line in a dialogue box, the matching canonical line id.

py -3 -I voice_table.py <all.enum> <canon.tsv> <out.md> <whisper.tsv> [<whisper.tsv> ...]"""
import difflib
import re
import sys

enum_p, canon_p, out_p, tsvs = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
FIX = {"roddick": "roddeck", "rodic": "roddeck", "zavant": "sir vant", "savant": "sir vant",
       "kid": "kit", "tinder boxes": "tinderboxes", "tinder box": "tinderbox", "armor": "armour",
       "gonna": "going to", "like them": "light them", "like yours": "light yours"}


def norm(s):
    s = s.lower().replace("<br>", " ")
    for k, v in FIX.items():
        s = s.replace(k, v)
    return re.sub(r"[^a-z0-9 ]+", "", s).split()


tx = {}
for p in tsvs:
    for line in open(p, encoding="utf-8"):
        a = line.rstrip("\n").split("\t")
        if len(a) >= 3:
            tx[int(re.sub(r"\D", "", a[0]))] = (float(a[1] or 0), a[2])
canon = []
for line in open(canon_p, encoding="utf-8"):
    if line.startswith("#") or not line.strip():
        continue
    cid, kind, speaker, text = line.rstrip("\n").split("\t")
    canon.append((cid, kind, speaker, text, norm(text)))
enums, cur, isv = {}, None, False
for line in open(enum_p, encoding="utf-8"):
    line = line.rstrip("\n")
    m = re.match(r"\[enum_(\d+)\]", line)
    if m:
        cur, isv = int(m.group(1)), False
        continue
    if line == "outputtype=vorbis":
        isv = True
        enums[cur] = []
        continue
    if isv and line.startswith("val=") and "vorbis_" in line:
        k, v = line[4:].split(",")
        enums[cur].append((int(k), int(v.split("_")[1])))
with open(out_p, "w", encoding="utf-8") as o:
    for e, vals in sorted(enums.items()):
        o.write(f"\n## enum {e} ({len(vals)} lines)\n\n| # | clip | s | canon | transcript (Whisper) |\n|---|---|---|---|---|\n")
        for k, v in sorted(vals):
            secs, text = tx.get(v, (0.0, ""))
            w = norm(text)
            best, br = "", 0.0
            for c in canon:
                r = difflib.SequenceMatcher(None, w, c[4]).ratio() if w else 0
                if r > br:
                    best, br = c[0], r
            mark = best if br >= 0.75 else ""
            o.write(f"| {k} | {v} | {secs:.1f} | {mark} | {text or '(not transcribed)'} |\n")
print("enums:", {e: len(v) for e, v in sorted(enums.items())})
