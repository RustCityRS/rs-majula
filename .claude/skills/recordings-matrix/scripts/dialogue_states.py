"""Every distinct dialogue box in a reference video, without the misses chat_changes.py has.

chat_changes.py compares whole chatbox crops against a mean-difference threshold, so two npc
boxes in a row (same portrait, new text) and boxes the player clicks through quickly fall
under it, while the cursor and the "Click here to continue" / "Please wait..." flicker trip it.
This compares only the text band (the box minus its bottom continue line) and counts a change
when enough pixels change strongly: a new line of text moves thousands, a cursor a few hundred.
Frames are sampled every --step seconds; a state is saved once it has held for --hold seconds.

py -3 -I dialogue_states.py <video> <out_dir> <t0> <t1> <x0> <y0> <x1> <y1>
       [--step 0.1] [--hold 0.2] [--frac 0.012] [--continue-h 30]
Writes <out_dir>/<time>.png per state and sheet_NN.png (6 per sheet, time-labelled)."""
import argparse
import os

import cv2
import numpy as np

ap = argparse.ArgumentParser()
ap.add_argument("video")
ap.add_argument("out")
ap.add_argument("t0", type=float)
ap.add_argument("t1", type=float)
ap.add_argument("box", type=int, nargs=4)
ap.add_argument("--step", type=float, default=0.1)
ap.add_argument("--hold", type=float, default=0.2)
ap.add_argument("--frac", type=float, default=0.012)
ap.add_argument("--continue-h", type=int, default=30)
ap.add_argument("--head-w", type=int, default=120)
a = ap.parse_args()
x0, y0, x1, y1 = a.box
os.makedirs(a.out, exist_ok=True)
cap = cv2.VideoCapture(a.video)


def band(img):
    # The text only: not the continue line at the bottom, and not the chatheads at either
    # side, which animate while they talk and would never let a state hold still.
    g = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    w = g.shape[1]
    return g[: max(1, g.shape[0] - a.continue_h), a.head_w: max(a.head_w + 1, w - a.head_w)]


def changed(p, q):
    d = cv2.absdiff(band(p), band(q))
    return (d > 48).mean() > a.frac


saved = []
last = None
cand, cand_t = None, None
fps = cap.get(cv2.CAP_PROP_FPS) or 30.0
every = max(1, round(a.step * fps))
cap.set(cv2.CAP_PROP_POS_MSEC, a.t0 * 1000)
i = 0
while True:
    ok, frame = cap.read()
    if not ok:
        break
    t = cap.get(cv2.CAP_PROP_POS_MSEC) / 1000
    if t > a.t1:
        break
    i += 1
    if i % every:
        continue
    f = frame[y0:y1, x0:x1]
    if last is None or changed(last, f):
        if cand is None or changed(cand, f):
            cand, cand_t = f, t
        elif t - cand_t >= a.hold:
            last = cand
            cv2.imwrite(os.path.join(a.out, f"{cand_t:07.2f}.png"), cand)
            saved.append((cand_t, cand))
            cand = None
    else:
        cand = None

h, w = saved[0][1].shape[:2] if saved else (0, 0)
for s in range(0, len(saved), 6):
    tiles = []
    for tt, img in saved[s:s + 6]:
        img = img.copy()
        cv2.putText(img, f"{tt:.1f}s", (4, 16), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 255, 255), 1)
        tiles.append(img)
    while len(tiles) < 6:
        tiles.append(np.zeros((h, w, 3), np.uint8))
    rows = [np.hstack(tiles[i:i + 2]) for i in range(0, 6, 2)]
    cv2.imwrite(os.path.join(a.out, f"sheet_{s // 6:02d}.png"), np.vstack(rows))
print(len(saved), "states")
