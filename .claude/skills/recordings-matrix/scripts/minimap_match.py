"""Where a recording's player stood, from its minimap, against our client's minimap at a known tile.

The minimap is the map around the player, turned with the camera and centred on the player. Our
reference is north-up with the player on a known tile. The recording's minimap disc is turned by
theta and scaled by s, then slid over ours; the best match's shift, at 4 minimap px per tile
(times s), is the recording's tile relative to ours.

py -3 -I minimap_match.py <ref video|png>@<s> <ref cx,cy> <rec video|png>@<s> <rec cx,cy> <debug.png>
    [--r 60] [--scales 0.85,1.25,0.02] [--tile x,z] [--ppt 4]
Prints theta, scale, shift (px, east/north) and tiles; with --tile, the recording's tile."""
import sys

import cv2
import numpy as np

a = sys.argv[1:]
opts = {"--r": "60", "--scales": "0.85,1.25,0.02", "--tile": None, "--ppt": "4"}
for k in list(opts):
    if k in a:
        i = a.index(k)
        opts[k] = a[i + 1]
        del a[i:i + 2]
R = int(opts["--r"])


def frame(spec):
    src, _, t = spec.partition("@")
    if src.endswith(".png"):
        return cv2.imread(src)
    cap = cv2.VideoCapture(src)
    cap.set(cv2.CAP_PROP_POS_MSEC, float(t or 0) * 1000)
    ok, f = cap.read()
    return f


PAD = 200


def padded(img):
    return cv2.copyMakeBorder(img, PAD, PAD, PAD, PAD, cv2.BORDER_CONSTANT)


def features(img):
    """Floor brightness and white wall lines, as one float image; dots and icons blurred away."""
    g = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY).astype(np.float32)
    g = cv2.medianBlur(g.astype(np.uint8), 3).astype(np.float32)
    return (g - g.mean()) / (g.std() + 1e-6)


ref = padded(frame(a[0]))
rcx, rcy = (int(v) + PAD for v in a[1].split(","))
rec = padded(frame(a[2]))
ccx, ccy = (int(v) + PAD for v in a[3].split(","))
big = int(R * 1.6)
ref_patch = features(ref[rcy - big:rcy + big, rcx - big:rcx + big])
rec_disc = rec[ccy - big:ccy + big, ccx - big:ccx + big]
lo, hi, step = map(float, opts["--scales"].split(","))
mask = np.zeros((2 * R, 2 * R), np.uint8)
cv2.circle(mask, (R, R), R - 2, 255, -1)
best = (-2, None)
for s in np.arange(lo, hi + 1e-9, step):
    for theta in range(0, 360, 2):
        M = cv2.getRotationMatrix2D((big, big), theta, s)
        rot = cv2.warpAffine(rec_disc, M, (2 * big, 2 * big))
        tmpl = features(rot[big - R:big + R, big - R:big + R])
        res = cv2.matchTemplate(ref_patch, tmpl, cv2.TM_CCOEFF_NORMED, mask=mask)
        _, mx, _, loc = cv2.minMaxLoc(res)
        if np.isfinite(mx) and mx > best[0]:
            best = (mx, (s, theta, loc))
score, (s, theta, loc) = best
# The template's centre sits at loc + R in ref_patch; ref's player is at big.
dx = loc[0] + R - big
dy = loc[1] + R - big
ppt = float(opts["--ppt"])
tx, tz = dx / ppt, -dy / ppt
print(f"score {score:.3f} scale {s:.2f} theta {theta} shift px ({dx},{dy}) tiles east {tx:+.2f} north {tz:+.2f}")
if opts["--tile"]:
    x0, z0 = map(int, opts["--tile"].split(","))
    print(f"recording's tile ~ ({x0 + tx:.1f}, {z0 + tz:.1f})")
M = cv2.getRotationMatrix2D((big, big), theta, s)
rot = cv2.warpAffine(rec_disc, M, (2 * big, 2 * big))
canvas = ref[rcy - big:rcy + big, rcx - big:rcx + big].copy()
over = canvas.copy()
over[loc[1]:loc[1] + 2 * R, loc[0]:loc[0] + 2 * R] = rot[big - R:big + R, big - R:big + R]
blend = cv2.addWeighted(canvas, 0.5, over, 0.5, 0)
cv2.imwrite(a[4],
            cv2.resize(np.hstack([canvas, blend]), None, fx=3, fy=3, interpolation=cv2.INTER_NEAREST))
