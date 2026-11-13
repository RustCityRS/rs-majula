#!/usr/bin/env bash
# Dialogue states -> OCR -> state matrix, for every recording listed in <work>/boxes.tsv.
#
#   matrix.sh <work>
#
# <work>/boxes.tsv   label<TAB>video file<TAB>x0 y0 x1 y1   (the dialogue box, in the video's pixels)
# <work>/canon.tsv   id<TAB>kind<TAB>speaker<TAB>text            (known lines, story order)
# Writes <work>/ds/<label>/ (state crops), <work>/ocr/<label>.tsv, <work>/out/ (matrix.md,
# paths.tsv, edges.md, review.tsv + review sheets) and <work>/STATES.md.
# OCR uses Windows' built-in engine (winocr.ps1, Windows PowerShell); elsewhere, write
# <work>/ocr/<label>.tsv yourself ("<state>.png<TAB>line / line ...") with any OCR, then rerun.
set -euo pipefail
work=${1:?usage: matrix.sh <work>}
here=$(cd "$(dirname "$0")" && pwd)
py=""; for c in python3 python "py -3"; do if $c -c "import cv2" >/dev/null 2>&1; then py=$c; break; fi; done; [ -n "$py" ] || { echo "need Python with opencv-python and numpy" >&2; exit 1; }
winpath() { if command -v cygpath >/dev/null; then cygpath -w "$1"; else echo "$1"; fi; }
mkdir -p "$work/ds" "$work/ocr" "$work/out"
: > "$work/manifest.tsv"
grep -v '^#' "$work/boxes.tsv" | while IFS=$'\t' read -r label video box; do
  [ -z "$label" ] && continue
  if [ ! -s "$work/ocr/$label.tsv" ]; then
    set -- $box
    # Chatheads take about a sixth of the box at each side, the continue line a seventh of its height.
    hw=$(( ($3 - $1) * 163 / 1000 )); ch=$(( ($4 - $2) * 156 / 1000 ))
    dur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$video" < /dev/null)
    $py "$here/dialogue_states.py" "$video" "$work/ds/$label" 0 "${dur%.*}" $box \
      --head-w "$hw" --continue-h "$ch" < /dev/null | tail -1 | sed "s/^/$label /"
    if command -v powershell.exe >/dev/null; then
      powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(winpath "$here/winocr.ps1")" \
        "$(winpath "$work/ds/$label")" "$(winpath "$work/ocr/$label.tsv")" < /dev/null | tail -1
    else
      echo "$label: no OCR here; write $work/ocr/$label.tsv and rerun" >&2
      continue
    fi
  fi
  # Paths inside a file aren't converted for a native Windows Python the way arguments are.
  printf '%s\t%s\t%s\n' "$label" "$(winpath "$work/ds/$label")" "$(winpath "$work/ocr/$label.tsv")" >> "$work/manifest.tsv"
done
$py "$here/state_matrix.py" "$work/manifest.tsv" "$work/canon.tsv" "$work/out"
$py "$here/states_doc.py" "$work/canon.tsv" "$work/out" "Every dialogue state in the recordings" > "$work/STATES.md"
echo "wrote $work/STATES.md"
