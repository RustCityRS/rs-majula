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

## Beyond dialogue: closing the other gaps from the same recordings

- **Where a player stands.** Use this for start tiles, item spots and arrival tiles.
  `python $S/minimap_match.py <ours.mp4>@<s> <cx,cy> <recording>@<s> <cx,cy> dbg.png --tile x,z`
  - It fits the recording's minimap (rotation, scale, shift) onto our north-up minimap recorded
    at a known tile.
  - `cx,cy` are the player's dot in each video.
  - A ground item's tile is where the player stands as it enters the inventory.
  - Confirm the answer by recording ours at that tile: the shift comes back (0,0).
- **Cutscene beats.**
  - `python $S/shots.py <video> <t0> <t1> <x0,y0,x1,y1> sheet.png` finds cuts and fades.
  - `python $S/frames_at.py out.png <video>@<s>@<label> ...` tiles chosen frames. Tile one
    frame a second, so pans show too.
  - Overhead lines (yellow text over heads) turn up in the whole-frame OCR (`frames.sh`).
- **Music.** Record each candidate track from our client, audio only. Then run
  `python $S/music_match.py <video> <t0> <t1> name=ref.wav ...`.
  - Chroma tells tunes apart; `--spectrum` tells two arrangements of one tune apart.
  - Self-test first: each reference against itself should score about 0.9.
  - Expect many recordings to carry the uploader's own music. Use one where character creation
    or another known track scores high.
- **Voice-over.**
  - The cache's vorbis clips (`content/647/vorbis/vorbis_<id>.ogg`) are the voiced lines.
  - Enums with `outputtype=vorbis` list each speaker's lines alphabetically, one clip each: the
    take the game used.
  - Transcribe with `python $S/whisper_clips.py <wav dir> <out.tsv>`: Whisper through
    transformers. The model download needs the user's go-ahead.
  - `python $S/voice_table.py <all.enum> <canon.tsv> <out.md> <transcripts...>` lays out every
    enum with its transcripts and the matching recorded lines.
  - Recordings rarely carry the voices: the client plays speech only when the player's voice
    volume is on.
- **Forced line breaks.** Run `python $S/br_check.py <q8_full.fontmetrics> <canon.tsv> <states.tsv>
  <high-res labels>`, and check every line it can't settle by eye. A break where the next word
  would still fit is a `<br>`.
  - Chat and item boxes wrap at 380 px; message boxes at 470.

## Reading the output

- **Counts.** A line's `n/N` says how many recordings show it. Rare lines are branches: wrong
  items, optional talks, out-of-order actions.
- **Branches.** A state with two or more exits seen at least twice is a real branch, such as a
  menu pick or a skipped talk. An exit seen once is often a box the cutter missed.
- **Low resolution.** 320x240 is unreadable; 640x480 reads once scaled.
- **Colours.** Video compression mutes thin text colours: compare against a known word in the same
  video.
