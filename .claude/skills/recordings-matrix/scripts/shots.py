"""Shot boundaries in part of a recording: camera cuts and fades in the game view.

Samples every --step s; a cut is a frame whose game view differs strongly from the last (mean abs
difference over --cut), a fade frame one that is nearly black. Prints each shot (start, end, kind)
and writes a contact sheet with one frame from the middle of each shot.

py -3 -I shots.py <video> <t0> <t1> <x0,y0,x1,y1 game view> <sheet.png> [--step 0.2] [--cut 28]"""
import sys

import cv2
import numpy as np

a = sys.argv[1:]
step, cut = 0.2, 28.0
for flag in ("--step", "--cut"):
    if flag in a:
        i = a.index(flag)
        if flag == "--step":
            step = float(a[i + 1])
        else:
            cut = float(a[i + 1])
        del a[i:i + 2]
video, t0, t1, box, sheet = a[0], float(a[1]), float(a[2]), a[3], a[4]
x0, y0, x1, y1 = map(int, box.split(","))
cap = cv2.VideoCapture(video)
fps = cap.get(cv2.CAP_PROP_FPS) or 30
cap.set(cv2.CAP_PROP_POS_MSEC, t0 * 1000)
every = max(1, round(step * fps))
prev, shots, frames = None, [], {}
start, kind = t0, None
i = 0
while True:
    ok, f = cap.read()
    if not ok:
        break
    t = cap.get(cv2.CAP_PROP_POS_MSEC) / 1000
    if t > t1:
        break
    i += 1
    if i % every:
        continue
    g = cv2.cvtColor(cv2.resize(f[y0:y1, x0:x1], (160, 100)), cv2.COLOR_BGR2GRAY).astype(np.float32)
    black = g.mean() < 12
    k = "black" if black else "scene"
    if prev is not None:
        d = np.abs(g - prev).mean()
        if k != kind or (not black and d > cut):
            shots.append((start, t, kind))
            start = t
    kind = k if prev is None or k != kind or True else kind
    frames.setdefault(len(shots), []).append((t, f[y0:y1, x0:x1].copy()))
    prev = g
shots.append((start, t1, kind))
for n, (s, e, k) in enumerate(shots):
    print(f"shot {n:2d} {s:7.2f} -> {e:7.2f} ({e - s:5.2f}s) {k}")
tiles = []
for n, (s, e, k) in enumerate(shots):
    fr = frames.get(n) or frames.get(n + 1)
    if not fr:
        continue
    t, img = fr[len(fr) // 2]
    img = cv2.resize(img, (320, round(img.shape[0] * 320 / img.shape[1])))
    cv2.rectangle(img, (0, 0), (320, 16), (0, 0, 0), -1)
    cv2.putText(img, f"{n} {s:.1f}-{e:.1f} {k}", (3, 12), cv2.FONT_HERSHEY_SIMPLEX, 0.4, (0, 255, 255), 1)
    tiles.append(img)
if tiles:
    h = max(x.shape[0] for x in tiles)
    tiles = [cv2.copyMakeBorder(x, 0, h - x.shape[0], 0, 0, cv2.BORDER_CONSTANT) for x in tiles]
    while len(tiles) % 4:
        tiles.append(np.zeros_like(tiles[0]))
    cv2.imwrite(sheet, np.vstack([np.hstack(tiles[j:j + 4]) for j in range(0, len(tiles), 4)]))
