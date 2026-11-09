---
name: recordings-matrix
description: Find every dated 2010-11 YouTube recording of a piece of RuneScape content and decode them into a state matrix - every dialogue line, menu, message, task step and option players took, with branches and first sightings. Use before porting or checking content, or when asked what 2011 players saw.
---

# Recordings → state matrix

No single recording shows every state, so decode all of them and merge the results. Everything
here lives in this skill's `scripts/` folder (`S=.claude/skills/recordings-matrix/scripts`). It
needs yt-dlp, ffmpeg, Python with opencv-python and numpy, and an OCR engine (`winocr.ps1` uses
Windows' built-in one).

## Steps

1. **Pick a work folder** outside the repo, e.g. `target/refs/<content>/`. Downloaded videos
   never go in git.
2. **Search widely**: the name, the npcs, "episode 1", "new account", "<name> 2011".
   ```sh
   yt-dlp --flat-playlist --print "%(id)s %(title)s" "ytsearch100:<query>"
   yt-dlp --skip-download --print "%(upload_date)s %(id)s %(duration)ss %(title)s" -- <id>
   ```
   When you find one part of a series, list the uploader's channel
   (`.../@<uploader>/videos`) for the rest.
3. **Keep the content's build window** (skill `sources-2011`). Record what you drop and why in
   `<work>/excluded.tsv`.
4. **Check each video with a contact sheet** before using it. Same-named bands and lectures
   turn up.
   ```sh
   ffmpeg -i v.mp4 -vf "fps=1/15,scale=320:-2,tile=6x5" -frames:v 1 contact.png
   ```
5. **Download each video to `<work>/videos/<date>_<id>/`** at 720p at most, and treat it as
   untrusted:
   ```sh
   yt-dlp -f "bv*[height<=720][ext=mp4]+ba[ext=m4a]/b[height<=720]" -o "<dir>/%(id)s.%(ext)s" -- <id>
   ```
6. **Find each video's dialogue box.** `python $S/detect_box.py check.png <videos...>` guesses
   them; look at `check.png`. Fix wrong guesses by reading a gridded frame. Then write
   `<work>/boxes.tsv`: `label<TAB>video<TAB>x0 y0 x1 y1`.
7. **Write `<work>/canon.tsv`**, the lines you already know, in story order:
   `id<TAB>kind<TAB>speaker<TAB>text`.
   - Kinds: chatnpc, chatplayer, mesbox, objbox, menu, hint, game.
   - Use `PLAYER` as the player's speaker.
8. **Run `bash $S/matrix.sh <work>`.** It cuts the dialogue states, OCRs them, matches them to the
   canon, and writes `out/matrix.md`, `out/edges.md` and `STATES.md`.
9. **Read the leftovers by eye.**
   - `python $S/tile_review.py <work>/out <work>/manifest.tsv sheet.png r001 r002 ...` tiles
     chosen review crops.
   - Add each new line with its exact punctuation, then run step 8 again. Repeat until the
     review list is only noise.
10. **For task steps and options**, run `bash $S/frames.sh <work> <tasks.txt>`. It writes
    `streams/tasks.md` (which task each video shows, and when) and `streams/ops.md` (options
    players pointed at).
11. **Record the result.** Put STATES.md's lines, branches and sources into the content's design
    note in `docs/`.

## Reading the output

- **Counts.** A line's `n/N` says how many recordings show it. Rare lines are branches: wrong
  items, optional talks, out-of-order actions.
- **Branches.** A state with two or more exits seen at least twice is a real branch, such as a
  menu pick or a skipped talk. An exit seen once is often a box the cutter missed.
- **Low resolution.** 320x240 is unreadable; 640x480 reads once scaled.
- **Colours.** Video compression mutes thin text colours: compare against a known word in the same
  video.
