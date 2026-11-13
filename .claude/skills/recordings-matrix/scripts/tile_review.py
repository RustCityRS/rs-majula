"""Tile chosen review crops (by r-id from review.tsv, or label@time from states.tsv) into one sheet.

py -3 -I tile_review.py <matrix_out_dir> <manifest.tsv> <out.png> <id> [<id> ...]
An id is r123 (review.tsv) or <label>@<time> (the state crop named <time>.png in that video's dir)."""
import os
import sys

import cv2
import numpy as np

out_dir, manifest, out_png, ids = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
dirs = {}
for line in open(manifest, encoding="utf-8"):
    if line.strip() and not line.startswith("#"):
        p = line.rstrip("\n").split("\t")
        dirs[p[0]] = p[1]
review = {}
for line in open(os.path.join(out_dir, "review.tsv"), encoding="utf-8"):
    p = line.rstrip("\n").split("\t")
    review[p[0]] = (p[1], p[2])
tiles = []
for i in ids:
    label, t = review[i] if i in review else i.split("@")
    d = dirs[label]
    name = min((n for n in os.listdir(d) if n[0].isdigit() and n.endswith(".png")),
               key=lambda n: abs(float(n[:-4]) - float(t)))
    img = cv2.imread(os.path.join(d, name))
    img = cv2.resize(img, (700, max(80, min(240, round(img.shape[0] * 700 / img.shape[1])))),
                     interpolation=cv2.INTER_CUBIC)
    img = cv2.copyMakeBorder(img, 0, 240 - img.shape[0], 0, 0, cv2.BORDER_CONSTANT)
    cv2.putText(img, f"{i} {label[:8]} {name[:-4]}", (4, 16), cv2.FONT_HERSHEY_SIMPLEX, 0.5,
                (0, 160, 255), 1)
    tiles.append(img)
while len(tiles) % 2:
    tiles.append(np.zeros_like(tiles[0]))
cv2.imwrite(out_png, np.vstack([np.hstack(tiles[k:k + 2]) for k in range(0, len(tiles), 2)]))
