#!/usr/bin/env bash
# Whole-frame OCR every 2 s of each recording in <work>/boxes.tsv, then the task-panel and
# option streams.
#
#   frames.sh <work> <tasks.txt>
#
# <tasks.txt>: the content's task structs ("## <index> <struct>" then "param_1266 = <name>" lines),
# e.g. dumped from content/647/scripts/configs/_unpack/all.struct.
# Writes <work>/frameocr/<label>.tsv and <work>/streams/{tasks.md,ops.md,ops_raw.tsv}. Frames are
# deleted after OCR.
set -euo pipefail
work=${1:?usage: frames.sh <work> <tasks.txt>}
tasks=${2:?usage: frames.sh <work> <tasks.txt>}
here=$(cd "$(dirname "$0")" && pwd)
py=""; for c in python3 python "py -3"; do if $c -c "import cv2" >/dev/null 2>&1; then py=$c; break; fi; done; [ -n "$py" ] || { echo "need Python with opencv-python and numpy" >&2; exit 1; }
winpath() { if command -v cygpath >/dev/null; then cygpath -w "$1"; else echo "$1"; fi; }
mkdir -p "$work/frameocr"
grep -v '^#' "$work/boxes.tsv" | cut -f1,2 | while IFS=$'\t' read -r label video; do
  [ -z "$label" ] || [ -s "$work/frameocr/$label.tsv" ] && continue
  d="$work/frames/$label"
  mkdir -p "$d"
  # Small recordings are scaled up to 1280 wide so the game's fonts can be read.
  ffmpeg -nostdin -v error -y -i "$video" -vf "fps=1/2,scale='max(1280,iw)':-2" -q:v 3 "$d/f_%05d.jpg"
  for f in "$d"/f_*.jpg; do
    n=${f##*/f_}; n=$((10#${n%.jpg}))
    mv "$f" "$d/$(printf '%07.2f' "$(( (n - 1) * 2 ))").jpg"
  done
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(winpath "$here/winocr.ps1")" \
    "$(winpath "$d")" "$(winpath "$work/frameocr/$label.tsv")" 1.0 < /dev/null | tail -1
  rm -rf "$d"
done
rmdir "$work/frames" 2>/dev/null || true
$py "$here/frame_streams.py" "$work/frameocr" "$tasks" "$work/streams"
