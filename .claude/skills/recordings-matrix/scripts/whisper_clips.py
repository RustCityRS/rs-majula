"""Transcribe WAV clips with Whisper (transformers, GPU if present), one TSV line each:
<file name> TAB <seconds> TAB <text>

py -3 -I whisper_clips.py <wav dir> <out.tsv> [--model openai/whisper-small.en]"""
import os
import sys
import wave

import numpy as np
import torch
from transformers import pipeline

a = sys.argv[1:]
model = "openai/whisper-small.en"
if "--model" in a:
    i = a.index("--model")
    model = a[i + 1]
    del a[i:i + 2]
src, out = a[0], a[1]
dev = 0 if torch.cuda.is_available() else -1
asr = pipeline("automatic-speech-recognition", model=model, device=dev,
               torch_dtype=torch.float16 if dev == 0 else torch.float32)
files = sorted((f for f in os.listdir(src) if f.endswith(".wav")),
               key=lambda f: int("".join(c for c in f if c.isdigit()) or 0))
rows = []
for f in files:
    with wave.open(os.path.join(src, f)) as w:
        sr = w.getframerate()
        x = np.frombuffer(w.readframes(w.getnframes()), np.int16).astype(np.float32) / 32768
    secs = len(x) / sr
    text = asr({"raw": x, "sampling_rate": sr}, chunk_length_s=30)["text"].strip() if secs > 0.2 else ""
    rows.append(f"{f}\t{secs:.2f}\t{text}")
with open(out, "w", encoding="utf-8") as o:
    o.write("\n".join(rows) + "\n")
print(len(rows), "clips")
