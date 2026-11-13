"""Read state streams other than dialogue out of whole-frame OCR (frame_ocr.sh): which tutorial task
the side panel shows, which tasks pop "TASK COMPLETE!", and which options the top-left action text
names ("Climb-over Low wall / 2 more options") -- every interaction players point at.

py -3 -I frame_streams.py <frameocr_dir> <tasks_786.txt> <out_dir> [<label> ...]
  tasks_786.txt: the "## <index> <struct>" + "param_1266 = <name>" dump of the tutorial tasks.
Writes out_dir/tasks.md (task x video: first time the panel shows it, * = also popped complete),
out_dir/ops.md (option x video: frames seen), out_dir/ops_raw.tsv (every action line, for review)."""
import difflib
import os
import re
import sys

src, tasks_path, out = sys.argv[1], sys.argv[2], sys.argv[3]
only = set(sys.argv[4:])


def norm(s):
    return re.sub(r"[^a-z0-9 ]+", "", s.lower()).strip()


def ratio(a, b):
    return difflib.SequenceMatcher(None, a, b, autojunk=False).ratio()


tasks = []
for line in open(tasks_path, encoding="utf-8"):
    m = re.match(r"## (\d+) ", line)
    if m:
        idx = int(m.group(1))
    elif line.startswith("param_1266 = "):
        tasks.append((idx, line.split(" = ", 1)[1].strip()))
tasks.sort()
task_norm = [(i, n, norm(n)) for i, n in tasks]

# Interface options the tutorial uses (top-left text is "<Option> <target>[ / N more options]").
VERBS = ["Talk-to", "Take", "Use", "Search", "Climb-over", "Climb-through", "Climb-up", "Climb-down",
         "Climb", "Attack", "Wield", "Wear", "Remove", "Examine", "Light", "Chop", "Chop down", "Mine",
         "Open", "Close", "Eat", "Drop", "Rip", "Soak", "Walk here", "Read", "Pick", "Pick-up",
         "Enter", "Squeeze-through", "Pull", "Push", "Destroy", "Empty", "Fill", "Continue", "Select",
         "Cancel", "Inspect", "Look-at", "Cast", "Give", "Hand over", "Light", "Exit", "Leave"]
verb_norm = sorted({norm(v) for v in VERBS}, key=len, reverse=True)

videos = sorted(f[:-4] for f in os.listdir(src) if f.endswith(".tsv"))
if only:
    videos = [v for v in videos if v in only]
seen_task, done_task, ops, raw = {}, {}, {}, []
for v in videos:
    for row in open(os.path.join(src, v + ".tsv"), encoding="utf-8-sig"):
        name, _, text = row.rstrip("\n").partition("\t")
        t = float(name[:-4])
        lines = [s.strip() for s in text.split(" / ") if s.strip()]
        complete = any(ratio(norm(s), "task complete") > 0.8 for s in lines)
        for s in lines:
            ns = norm(s)
            if 6 <= len(ns) <= 40:
                best = max(task_norm, key=lambda tn: ratio(ns, tn[2]))
                if ratio(ns, best[2]) >= 0.86:
                    seen_task.setdefault(best[0], {}).setdefault(v, t)
                    if complete:
                        done_task.setdefault(best[0], {}).setdefault(v, t)
            # Action text: an option verb first, then a target, often "N more options" after it.
            m = re.match(r"^([A-Za-z][A-Za-z\- ]+?) (.+?)(?: \S{0,3} ?\d+ more options?)?$", s)
            if not m or len(s) > 70:
                continue
            first = norm(s)
            verb = next((vn for vn in verb_norm if first.startswith(vn + " ") or first == vn), None)
            if verb is None or ("more option" not in s and verb not in ("walk here",)):
                continue
            target = norm(first[len(verb):].split("more option")[0])
            target = re.sub(r"\s*\d+$", "", target).strip()
            key = f"{verb} | {target}" if target else verb
            ops.setdefault(key, {}).setdefault(v, []).append(t)
            raw.append((v, t, s))

os.makedirs(out, exist_ok=True)
short = [v[:8] for v in videos]
with open(os.path.join(out, "tasks.md"), "w", encoding="utf-8") as f:
    f.write("| idx | task | seen | " + " | ".join(short) + " |\n" + "|---" * (3 + len(videos)) + "|\n")
    for i, n in tasks:
        s, d = seen_task.get(i, {}), done_task.get(i, {})
        cells = [(f"{s[v]:.0f}" + ("*" if v in d else "")) if v in s else ("*" if v in d else "")
                 for v in videos]
        f.write(f"| {i} | {n} | {len(s)}/{len(videos)} | " + " | ".join(cells) + " |\n")
with open(os.path.join(out, "ops.md"), "w", encoding="utf-8") as f:
    f.write("| option | videos | frames | " + " | ".join(short) + " |\n" + "|---" * (3 + len(videos)) + "|\n")
    for key in sorted(ops, key=lambda k: (-len(ops[k]), k)):
        cells = [str(len(ops[key][v])) if v in ops[key] else "" for v in videos]
        total = sum(len(x) for x in ops[key].values())
        f.write(f"| {key} | {len(ops[key])} | {total} | " + " | ".join(cells) + " |\n")
with open(os.path.join(out, "ops_raw.tsv"), "w", encoding="utf-8") as f:
    for v, t, s in raw:
        f.write(f"{v}\t{t:.0f}\t{s}\n")
print(f"{len(videos)} videos, {len(seen_task)}/{len(tasks)} tasks seen, {len(ops)} distinct options")
