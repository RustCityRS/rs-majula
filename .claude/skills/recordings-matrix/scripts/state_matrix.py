"""A state matrix for one piece of content across many recordings: which dialogue states each video
shows, in story order, with the time each is first seen.

Per video: dialogue_states.py writes the dialogue-box crops (<dir>/<time>.png), winocr.ps1 reads
them (<ocr.tsv>: name TAB "line / line ..."). Each OCR'd state is matched to the canonical lines
(canon.tsv: id, kind, speaker, text, in story order) by difflib ratio on letters and digits, with
and without the speaker; a state below --min or too close to a second line is "?" and goes to the
review sheets, where it is read by eye and either added to canon.tsv (a new line) or left as junk.

  py -3 -I state_matrix.py <manifest.tsv> <canon.tsv> <out_dir> [--min 0.6]
      manifest rows: <label>\t<states dir>\t<ocr tsv>   (label = <yyyymmdd>_<video id>)
  writes out_dir/states.tsv, out_dir/matrix.md, out_dir/review_NN.png (+ review.tsv index)."""
import difflib
import os
import re
import sys

import cv2
import numpy as np


def norm(s):
    return re.sub(r"[^a-z0-9 ]+", "", s.lower().replace(" / ", " ")).strip()


def score(ocr, text):
    return difflib.SequenceMatcher(None, ocr, text, autojunk=False).ratio()


def variants(text):
    """The OCR'd box as text, minus its continue line ("Click here to continue", "Please wait..."),
    and that again without its first line, which may be a speaker: the player's name differs in
    every video."""
    segs = [s for s in text.split(" / ")
            if max(score(norm(s), "click here to continue"), score(norm(s), "please wait")) < 0.6]
    out = [norm(" / ".join(segs))]
    if len(segs) > 1 and len(segs[0].split()) <= 3:
        out.append(norm(" / ".join(segs[1:])))
    return [v for v in out if v]


def main(manifest, canon_path, out, minimum):
    canon = []
    for line in open(canon_path, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        cid, kind, speaker, text = line.rstrip("\n").split("\t")
        canon.append((cid, kind, speaker, text, norm(text), norm(f"{speaker} {text}")))
    canon_by = {c[0]: c for c in canon}
    games = [c[0] for c in canon if c[1] == "game"]
    videos, states, review, new_segs, seen_game = [], [], [], [], {}
    for line in open(manifest, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        label, d, ocr_path = line.rstrip("\n").split("\t")[:3]
        videos.append(label)
        last = None
        for row in open(ocr_path, encoding="utf-8-sig"):
            name, _, text = row.rstrip("\n").partition("\t")
            t = float(name[:-4])
            vs = variants(text)
            if not vs or max(len(v) for v in vs) < 6:
                last = None
                continue
            scored = sorted(((max(score(v, c[4]) for v in vs) if c[1] == "chatplayer" else
                              max(max(score(v, c[4]), score(v, c[5])) for v in vs), c[0])
                             for c in canon), reverse=True)
            (s1, c1), (s2, _) = scored[0], scored[1]
            if s1 >= minimum and (s1 - s2 >= 0.04 or s1 >= 0.85) and c1 not in games:
                if c1 != last:
                    states.append((label, t, c1, s1, text))
                last = c1
                continue
            last = None
            # Chat history: several game messages a frame, matched line by line; a line that is
            # no known message goes to segments.tsv to be read.
            segs = [s for s in text.split(" / ") if len(norm(s)) >= 8]
            hits = 0
            for seg in segs:
                gs, gid = max((score(norm(seg), canon_by[g][4]), g) for g in games)
                if gs >= 0.75:
                    hits += 1
                    if gid not in seen_game.setdefault(label, set()):
                        seen_game[label].add(gid)
                        states.append((label, t, gid, gs, seg))
                else:
                    new_segs.append((label, t, seg))
            if not hits:
                states.append((label, t, "?", s1, text))
                review.append((label, t, os.path.join(d, name), text, c1, s1))
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(out, "segments.tsv"), "w", encoding="utf-8") as f:
        done = set()
        for label, t, seg in new_segs:
            if (label, norm(seg)) not in done:
                done.add((label, norm(seg)))
                f.write(f"{label}\t{t:.2f}\t{seg}\n")
    with open(os.path.join(out, "states.tsv"), "w", encoding="utf-8") as f:
        for label, t, c, s, text in states:
            f.write(f"{label}\t{t:.2f}\t{c}\t{s:.2f}\t{text}\n")
    first = {}
    for label, t, c, _, _ in states:
        if c != "?":
            first.setdefault(c, {}).setdefault(label, t)
    with open(os.path.join(out, "matrix.md"), "w", encoding="utf-8") as f:
        f.write("| id | kind | speaker | text | seen | " + " | ".join(v[:8] for v in videos) + " |\n")
        f.write("|---" * (5 + len(videos)) + "|\n")
        for cid, kind, speaker, text, _, _ in canon:
            seen = first.get(cid, {})
            cells = [f"{seen[v]:.0f}" if v in seen else "" for v in videos]
            f.write(f"| {cid} | {kind} | {speaker} | {text} | {len(seen)}/{len(videos)} | "
                    + " | ".join(cells) + " |\n")
    # Review sheets: 8 unmatched crops a sheet, each labelled with its video, time and number.
    with open(os.path.join(out, "review.tsv"), "w", encoding="utf-8") as f:
        for i, (label, t, path, text, c1, s1) in enumerate(review):
            f.write(f"r{i:03d}\t{label}\t{t:.2f}\t{c1}\t{s1:.2f}\t{text}\n")
    tiles = []
    for i, (label, t, path, _, _, _) in enumerate(review):
        img = cv2.imread(path)
        img = cv2.resize(img, (600, max(60, min(200, round(img.shape[0] * 600 / img.shape[1])))))
        img = cv2.copyMakeBorder(img, 0, 200 - img.shape[0], 0, 0, cv2.BORDER_CONSTANT)
        cv2.putText(img, f"r{i:03d} {label[:8]} {t:.1f}", (4, 16), cv2.FONT_HERSHEY_SIMPLEX, 0.5,
                    (0, 160, 255), 1)
        tiles.append(img)
    for n in range(0, len(tiles), 8):
        group = tiles[n:n + 8]
        while len(group) < 8:
            group.append(np.zeros_like(tiles[0]))
        sheet = np.vstack([np.hstack(group[r * 2:r * 2 + 2]) for r in range(4)])
        cv2.imwrite(os.path.join(out, f"review_{n // 8:02d}.png"), sheet)
    # Paths and transitions: each video's dialogue states in order (game messages left out, as the
    # chat history repeats them), and every from -> to step with the videos that took it, so the
    # branches (menu picks, optional talks, wrong actions) show as a state with several exits.
    kind = {c[0]: c[1] for c in canon}
    paths = {}
    for label, t, c, _, _ in states:
        if c == "?" or kind.get(c) == "game":
            continue
        p = paths.setdefault(label, [])
        if not p or p[-1] != c:
            p.append(c)
    edges = {}
    for label, p in paths.items():
        for a, b in zip(p, p[1:]):
            edges.setdefault((a, b), []).append(label)
    order = {c[0]: i for i, c in enumerate(canon)}
    text = {c[0]: c[3] for c in canon}
    with open(os.path.join(out, "paths.tsv"), "w", encoding="utf-8") as f:
        for label in videos:
            f.write(f"{label}\t{' '.join(paths.get(label, []))}\n")
    with open(os.path.join(out, "edges.md"), "w", encoding="utf-8") as f:
        f.write("| from | to | videos | to text |\n|---|---|---|---|\n")
        for (a, b), vs in sorted(edges.items(), key=lambda e: (order.get(e[0][0], 999), -len(e[1]))):
            f.write(f"| {a} | {b} | {len(vs)} | {text.get(b, '')[:70]} |\n")
    matched = sum(1 for s in states if s[2] != "?")
    print(f"{len(videos)} videos, {matched} matched states, {len(review)} to review, "
          f"{len(first)}/{len(canon)} canonical lines seen")


if __name__ == "__main__":
    a = sys.argv
    m = float(a[a.index("--min") + 1]) if "--min" in a else 0.6
    main(a[1], a[2], a[3], m)
