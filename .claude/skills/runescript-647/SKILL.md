---
name: runescript-647
description: Port RuneScript scripts and configs from content/289 to content/647 (rev 647, cache 786), map names to 2011 ids, place server-only config keys, and avoid the known traps. Use when editing anything under content/647/scripts.
---

# RuneScript at 647

## Layout

| Path | What it holds |
|---|---|
| `content/647/scripts/<feature>/` | existing server layout, including `scripts/`, `configs/` and `interfaces/` |
| `content/647/scripts/<feature>/client/` | client scripts (`.cs2`) |
| `content/647/scripts/<feature>/global/` | shared procedures (`.gs2`) |
| `content/647/scripts/configs/_unpack/all.*` | cache 786 configs; add server-only keys to these sections |
| `content/647/pack/*.pack` | `id=name` registries |
| `content/647/maps/` | `l<mx>_<mz>.jm2` (locs: `level x z: id shape angle`), `n...` (npc spawns), `m...` (terrain) |

## Porting a file

1. Copy 289's script line for line into the same feature folder and relative path. Keep existing names such as `skill_combat` and `interface_bank`.
2. Map every name through `content/647/pack/<type>.pack`. Ids are mostly stable; names often
   differ.
3. Many 2005 locs, objs and npcs got new ids by 2011. Find the 2011 id in the maps or configs,
   and give it 289's categories and params.
4. Configs: put server keys (stats, params, categories, weight) on the cache's own section in
   `_unpack/all.<type>`. Never write a second definition.
5. Put each comment on its own line. A comment after a value on the same line breaks packing.
6. Lint: `runec lint --source content/647/scripts --pack content/647/pack`. It
   registers new script ids in `script.pack`. When you delete a script, remove its id line by
   hand.

## Look up the cache

```sh
S=.claude/skills/runescript-647/scripts U=content/647/scripts/configs/_unpack
python $S/config_sections.py $U/all.npc <name|id> ... [--keys name,op,param]
python $S/multivar_users.py $U <varbit or varp name> ...
```

Both scripts are in this skill's `scripts/` folder.
- `config_sections.py` prints chosen sections.
- `multivar_users.py` lists every npc and loc that changes form with a var, and each of its forms.

## Vars

- **No new varp or varbit ids.** Every var is one of cache 786's, its keys on its section in
  `_unpack/all.varp`. Take 289's id when 786 leaves it free (no clientscript, interface, quest,
  multiloc or varbit reads it another way), else an empty `varplayer_N`. The pack fails on a
  `.varp` or `.varbit` outside `_unpack`.
- **Read what the cache expects of a var** (quest config, clientscripts, multilocs) before choosing
  its values. Unstable Foundations, for example, completes at exactly 1000.
- **Never transmit a varp the client doesn't define.** It breaks login.

## Text and traps

- Line breaks are `<br>`. 289's `|` breaks were sometimes kept by 2011: check each against a
  recording.
- Dialogue wraps at 380 px in the dialogue font; message boxes at about 470.
- Menu titles ("Select an Option", "Choose an option:") are set per menu, not per date. Read each
  one off a recording.
- `examine.loc` was ported by id, so some examines belong to other locs. Fix any you touch.
- An npc's `readyanim` in the 647 unpack is a BAS id. Its combat anims must share its skeleton.

## Engine habits

- A modal opens at the end of the tick, after the script's packets: `p_delay(0)` before relying on
  it.
- Tab (non-modal) buttons run without protected access: take it with `p_finduid(uid)`.
- 647 op packets carry no route. The server paths when the op arrives, and re-paths each tick.
- Reuse a clientscript's logic as a globalscript proc rather than rewriting it.
