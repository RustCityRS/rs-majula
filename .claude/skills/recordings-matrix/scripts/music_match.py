"""Which music track a recording is playing, second by second, against reference recordings of
each candidate track (our client playing it).

Audio -> chroma (12 pitch classes, ~93 ms frames, log-compressed, normalised). Each window of the
recording (--win s, every --hop s) is slid over every reference track's chroma (cyclically, as
tracks loop) and scored by mean cosine similarity; the best track and its margin over the next are
printed, and runs of the same confident track are merged into segments.

py -3 -I music_match.py <recording> <t0> <t1> <name=ref.mp4> [<name=ref.mp4> ...]
    [--win 6] [--hop 2] [--min 0.55] [--margin 0.04]"""
import subprocess
import sys

import numpy as np

a = sys.argv[1:]
opts = {"--win": 6.0, "--hop": 2.0, "--min": 0.55, "--margin": 0.04}
FEATURE = "chroma"
if "--spectrum" in a:
    a.remove("--spectrum")
    FEATURE = "spectrum"
for k in list(opts):
    if k in a:
        i = a.index(k)
        opts[k] = float(a[i + 1])
        del a[i:i + 2]
SR, NFFT, HOP = 22050, 4096, 2048


def audio(path, t0=None, t1=None):
    cmd = ["ffmpeg", "-nostdin", "-v", "error"]
    if t0 is not None:
        cmd += ["-ss", str(t0), "-t", str(t1 - t0)]
    cmd += ["-i", path, "-ac", "1", "-ar", str(SR), "-f", "f32le", "-"]
    return np.frombuffer(subprocess.run(cmd, capture_output=True, check=True).stdout, np.float32)


freqs = np.fft.rfftfreq(NFFT, 1 / SR)
valid = (freqs > 55) & (freqs < 4000)
pc = np.zeros(len(freqs), int)
pc[valid] = np.round(12 * np.log2(freqs[valid] / 440.0)).astype(int) % 12
window = np.hanning(NFFT).astype(np.float32)


BANDS = np.geomspace(60, 6000, 65)
band_of = np.digitize(freqs, BANDS) - 1
band_ok = (band_of >= 0) & (band_of < 64)


def spectrum(x):
    """Log energy in 64 log-spaced bands (60-6000 Hz): pitch and timbre, to tell two
    arrangements of one tune apart (The Mentor's harpsichord, The Adventurer's brass)."""
    n = max(0, (len(x) - NFFT) // HOP + 1)
    out = np.zeros((n, 64), np.float32)
    for i in range(n):
        mag = np.abs(np.fft.rfft(x[i * HOP:i * HOP + NFFT] * window))
        out[i] = np.log1p(10 * np.bincount(band_of[band_ok], weights=mag[band_ok], minlength=64))
    out -= out.mean(axis=1, keepdims=True)
    out /= np.linalg.norm(out, axis=1, keepdims=True) + 1e-6
    return out


def chroma(x):
    if FEATURE == "spectrum":
        return spectrum(x)
    n = max(0, (len(x) - NFFT) // HOP + 1)
    out = np.zeros((n, 12), np.float32)
    for i in range(n):
        mag = np.abs(np.fft.rfft(x[i * HOP:i * HOP + NFFT] * window))
        c = np.bincount(pc[valid], weights=mag[valid], minlength=12)
        out[i] = np.log1p(10 * c)
    out -= out.mean(axis=1, keepdims=True)
    out /= np.linalg.norm(out, axis=1, keepdims=True) + 1e-6
    return out


rec, t0, t1 = a[0], float(a[1]), float(a[2])
refs = []
for spec in a[3:]:
    name, _, path = spec.partition("=")
    refs.append((name, chroma(audio(path))))
X = chroma(audio(rec, t0, t1))
fps = SR / HOP
win, hop = int(opts["--win"] * fps), int(opts["--hop"] * fps)
rows = []
for s in range(0, max(1, len(X) - win), hop):
    w = X[s:s + win]
    scores = []
    for name, R in refs:
        # cyclic slide: score at every offset of the reference
        RR = np.vstack([R, R[:win]])
        best = -1.0
        for off in range(0, len(R), 2):
            v = float(np.mean(np.sum(w * RR[off:off + win], axis=1)))
            if v > best:
                best = v
        scores.append((best, name))
    scores.sort(reverse=True)
    t = t0 + s / fps
    rows.append((t, scores))
    print(f"{t:7.1f}  " + "  ".join(f"{n}:{v:.2f}" for v, n in scores))
print("\nsegments (confident):")
seg = None
for t, scores in rows:
    (v1, n1), (v2, _) = scores[0], scores[1]
    name = n1 if v1 >= opts["--min"] and v1 - v2 >= opts["--margin"] else "?"
    if seg and seg[2] == name:
        seg[1] = t
    else:
        if seg:
            print(f"  {seg[0]:7.1f} - {seg[1]:7.1f}  {seg[2]}")
        seg = [t, t, name]
if seg:
    print(f"  {seg[0]:7.1f} - {seg[1]:7.1f}  {seg[2]}")
