"""Find the dialogue box of each recording (the chatbox in fixed mode, the box over the bottom left
of the scene in resizable mode) and draw it on one frame per video for checking by eye.

A box is a light, low-saturation, nearly solid rectangle in the lower half, 25-65% of the frame wide
with an aspect of 2.4-5.5. Frames are sampled every --step seconds; the most common rectangle
(quantised to 1% of the frame) wins.

py -3 -I detect_box.py <out.png> <video> [<video> ...] [--step 2]
Prints per video: label x0 y0 x1 y1 hits; writes a composite with each box drawn in yellow."""
import os
import sys
from collections import Counter

import cv2
import numpy as np

args = [a for a in sys.argv[1:]]
step = 2.0
if "--step" in args:
    i = args.index("--step")
    step = float(args[i + 1])
    del args[i:i + 2]
out, videos = args[0], args[1:]
shots = []
for v in videos:
    cap = cv2.VideoCapture(v)
    fps = cap.get(cv2.CAP_PROP_FPS) or 30
    n = cap.get(cv2.CAP_PROP_FRAME_COUNT)
    dur = n / fps
    votes, sample = Counter(), {}
    t = 5.0
    while t < dur - 2:
        cap.set(cv2.CAP_PROP_POS_MSEC, t * 1000)
        ok, f = cap.read()
        t += step
        if not ok:
            continue
        H, W = f.shape[:2]
        hsv = cv2.cvtColor(f, cv2.COLOR_BGR2HSV)
        mask = ((hsv[:, :, 2] > 120) & (hsv[:, :, 1] < 90)).astype(np.uint8) * 255
        mask[: H // 2] = 0
        mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
        cnts, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
        for c in cnts:
            x, y, w, h = cv2.boundingRect(c)
            if not (0.25 * W <= w <= 0.65 * W and 0.08 * H <= h <= 0.35 * H):
                continue
            if not (2.4 <= w / h <= 5.5) or cv2.contourArea(c) < 0.85 * w * h:
                continue
            q = tuple(int(round(k / (W / 100))) for k in (x, y, x + w, y + h))
            votes[q] += 1
            sample.setdefault(q, (t - step, f.copy(), (x, y, x + w, y + h)))
    label = os.path.basename(os.path.dirname(v)) or os.path.basename(v)
    if not votes:
        print(label, "none")
        continue
    q, hits = votes.most_common(1)[0]
    _, f, (x0, y0, x1, y1) = sample[q]
    print(label, x0, y0, x1, y1, hits, flush=True)
    cv2.rectangle(f, (x0, y0), (x1, y1), (0, 255, 255), 3)
    f = cv2.resize(f, (480, round(f.shape[0] * 480 / f.shape[1])))
    cv2.putText(f, label[:20], (4, 16), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 255, 255), 1)
    shots.append(f)
if shots:
    h = max(s.shape[0] for s in shots)
    shots = [cv2.copyMakeBorder(s, 0, h - s.shape[0], 0, 0, cv2.BORDER_CONSTANT) for s in shots]
    while len(shots) % 4:
        shots.append(np.zeros_like(shots[0]))
    rows = [np.hstack(shots[i:i + 4]) for i in range(0, len(shots), 4)]
    cv2.imwrite(out, np.vstack(rows))
