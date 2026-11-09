---
name: sources-2011
description: Decide what 2011 (cache 786, May 2011) did for a piece of RuneScape content by cross-referencing dated sources, and cite them. Use before writing any 2011 change, when sources disagree, or when dating a recording.
---

# Sources for 2011

## Which evidence wins (highest first)

1. **Cache 786**: `content/647/scripts/globalscript/_unpack/all.*`, `content/647/maps/*.jm2`,
   `content/647/interfaces/`, clientscripts.
2. **The RuneScape Wiki** as of about May 2011, never the live wiki. Use the page revision nearest
   2011-05-17 and quote its timestamp.
3. **The Wayback Machine**:
   `http://archive.org/wayback/available?url=<url>&timestamp=20110517`. It only works for exact
   URLs; encode `'` as `%27`.
4. **Dated 2010-11 recordings**, read frame by frame (skill `recordings-matrix`).
5. **Dev blogs and forum posts** from the time.
6. **Void (rev 634)**. Often wrong on punctuation, menus and choice titles: check each line
   against a recording, character by character.
7. **289**: the starting point, not proof of 2011.

**Exception: dialogue text.** Dated 2011 recordings come first, then Void, then 289. The wiki
often paraphrases, or keeps an older version of a line, so a wiki quote that no recording shows
is a lead to check, not an answer.

## Per claim

1. Find it, highest source first.
2. Read the exact line (key, script line or frame).
3. Write the date: the cache, the wiki revision, the snapshot or the recording (id @ second).
4. Comment it above the change:
   - `// 2011's: <source, date>: <what it shows>`, or
   - `// Void's, unconfirmed: ...`, or
   - `// No source found: kept as 289.`
5. Report claims in three kinds: shown by the cache or client; from Void only; unknown.

## Dating footage

- **Upload date is not recording date.** Date the footage by what is on screen. Example: the Task
  System (from 24 Nov 2010) says "You have completed the Task '...'".
- **Content changes inside 2010-11.** Use the build that matches cache 786. For example, the
  tutorial:
  - had a chatbox guide until mid-December 2010;
  - had the Task System panel after that, which is what the cache holds;
  - had no cellar for new players by late August 2011.
