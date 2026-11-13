"""Tile frames from several videos at chosen seconds, each labelled, for reading by eye.

py -3 -I frames_at.py <out.png> <video>@<seconds>[@label] ... [--w 560] [--cols 2]"""
import sys

import cv2
import numpy as np

args = sys.argv[1:]
w, cols = 560, 2
for flag in ("--w", "--cols"):
    if flag in args:
        i = args.index(flag)
        if flag == "--w":
            w = int(args[i + 1])
        else:
            cols = int(args[i + 1])
        del args[i:i + 2]
out, specs = args[0], args[1:]
tiles = []
for spec in specs:
    parts = spec.split("@")
    video, t = parts[0], float(parts[1])
    label = parts[2] if len(parts) > 2 else f"{video.rsplit('/', 1)[-1][:24]} @{t:.1f}"
    cap = cv2.VideoCapture(video)
    cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000)
    ok, f = cap.read()
    if not ok:
        f = np.zeros((315, w, 3), np.uint8)
    f = cv2.resize(f, (w, round(f.shape[0] * w / f.shape[1])), interpolation=cv2.INTER_AREA)
    cv2.rectangle(f, (0, 0), (w, 18), (0, 0, 0), -1)
    cv2.putText(f, label, (4, 14), cv2.FONT_HERSHEY_SIMPLEX, 0.45, (0, 255, 255), 1)
    tiles.append(f)
h = max(t.shape[0] for t in tiles)
tiles = [cv2.copyMakeBorder(t, 0, h - t.shape[0], 0, 0, cv2.BORDER_CONSTANT) for t in tiles]
while len(tiles) % cols:
    tiles.append(np.zeros_like(tiles[0]))
cv2.imwrite(out, np.vstack([np.hstack(tiles[i:i + cols]) for i in range(0, len(tiles), cols)]))
