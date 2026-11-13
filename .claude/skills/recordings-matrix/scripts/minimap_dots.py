"""Where a recording's npcs (yellow minimap dots) stand, in tiles from the player, once minimap_match.py
has fitted the recording's minimap (its theta and scale) against our north-up one.

The recording's disc is turned by theta and scaled by s exactly as minimap_match.py turns it, so it
reads north-up at our scale; then each yellow dot's offset from the centre, at --ppt minimap px per
tile, is its tile offset (east, north) from the player. With --tile, the npc's tile.

py -3 -I minimap_dots.py <rec video|png>@<s> <rec cx,cy> <theta> <scale> [--r 60] [--ppt 4]
    [--tile x,z] [--debug out.png]
Prints one line per dot: offset in tiles, the tile with --tile, and the dot's size in px."""
import sys

import cv2
import numpy as np

a = sys.argv[1:]
opts = {"--r": "60", "--ppt": "4", "--tile": None, "--debug": None}
for k in list(opts):
    if k in a:
        i = a.index(k)
        opts[k] = a[i + 1]
        del a[i:i + 2]
R, ppt = int(opts["--r"]), float(opts["--ppt"])


def frame(spec):
    src, _, t = spec.partition("@")
    if src.endswith(".png"):
        return cv2.imread(src)
    cap = cv2.VideoCapture(src)
    cap.set(cv2.CAP_PROP_POS_MSEC, float(t or 0) * 1000)
    ok, f = cap.read()
    return f


img = frame(a[0])
cx, cy = (int(v) for v in a[1].split(","))
theta, s = float(a[2]), float(a[3])
big = int(R * 1.6)
img = cv2.copyMakeBorder(img, big, big, big, big, cv2.BORDER_CONSTANT)
cx, cy = cx + big, cy + big
disc = img[cy - big:cy + big, cx - big:cx + big]
M = cv2.getRotationMatrix2D((big, big), theta, s)
rot = cv2.warpAffine(disc, M, (2 * big, 2 * big))
hsv = cv2.cvtColor(rot, cv2.COLOR_BGR2HSV)
# Npc dots are yellow: hue ~30 on OpenCV's 0-180 scale, saturated and bright. Compression mutes
# them, so the bounds are loose; the disc mask keeps the compass and orbs out.
yellow = cv2.inRange(hsv, (20, 120, 150), (38, 255, 255))
mask = np.zeros_like(yellow)
cv2.circle(mask, (big, big), int(R * s) - 3, 255, -1)
yellow &= mask
n, labels, stats, cents = cv2.connectedComponentsWithStats(yellow)
dots = []
for i in range(1, n):
    area = stats[i, cv2.CC_STAT_AREA]
    if area < 3:
        continue
    dx, dy = cents[i][0] - big, cents[i][1] - big
    dots.append((dx / ppt, -dy / ppt, area))
for east, north, area in sorted(dots, key=lambda d: d[0] ** 2 + d[1] ** 2):
    line = f"east {east:+.2f} north {north:+.2f}  ({area} px)"
    if opts["--tile"]:
        x0, z0 = map(float, opts["--tile"].split(","))
        line += f"  tile ~ ({x0 + east:.1f}, {z0 + north:.1f})"
    print(line)
if opts["--debug"]:
    out = rot.copy()
    for east, north, _ in dots:
        cv2.circle(out, (int(big + east * ppt), int(big - north * ppt)), 4, (0, 0, 255), 1)
    cv2.circle(out, (big, big), 2, (255, 255, 255), -1)
    cv2.imwrite(opts["--debug"], cv2.resize(out, None, fx=3, fy=3, interpolation=cv2.INTER_NEAREST))
