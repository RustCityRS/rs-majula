# Unstable Foundations (2011's tutorial) on 647: design note

Research for building Unstable Foundations (UF) as 647 content, end to end. Every
claim names its source. Kinds of claim are kept apart:

- **cache**: cache 786 (configs in `content/647/scripts/globalscript/_unpack/all.*`, maps in
  `content/647/maps/*.jm2`, interfaces in
  `content/647/interfaces/*.if3` (named components), decompiled
  clientscripts in `runescape/target/unpack-work/clientscript/_decompiled/<id>.cs2`),
  or the deob client `tools/client-src`.
- **wiki**: the local 2011 wiki (`wiki.py "<Title>"`), cited as `Title @ revision`.
- **void-634's**: `rs-native-old/ref/void-server` only.
- **inference**: my reading of cache facts, labelled as such.
- **no source found**: nothing shows it. The build stops at these.

Worktree HEAD when written: `7110b89d3`. Wiki pages read: Unstable Foundations @
2011-05-05T15:38:45Z, Sir Vant @ 2011-05-18T04:09:24Z, Roddeck @
2011-04-28T23:09:25Z, Tutorial Cellar @ 2011-05-01T00:48:56Z, Tutorial @
2011-05-05T21:48:15Z, Squire (Sir Vant) @ 2011-05-18T04:14:53Z, Goblin (Unstable
Foundations) @ 2011-04-16T22:20:18Z, Dragon (Unstable Foundations) @
2011-05-18T03:00:45Z, Drugged meat @ 2011-04-23T16:14:08Z, Dragon's food bowl @
2011-04-23T16:17:38Z, Plank (Unstable Foundations) @ 2011-04-23T16:14:00Z, Fuse
(Unstable Foundations) @ 2011-05-19T22:10:26Z, Rag @ 2011-05-19T22:15:40Z, Oily
rag @ 2011-05-19T22:10:31Z, Oily cloth @ 2011-04-23T16:14:47Z, Cloth @
2011-05-14T01:34:43Z, Sir vant's oil can @ 2011-04-23T16:14:45Z, Manual (Unstable
Foundations) @ 2011-04-14T12:24:02Z, Roddeck's diary @ 2011-03-13T20:32:26Z, Stone
clippings @ 2010-12-12T05:46:30Z, Root chippings @ 2011-04-16T22:16:31Z,
Update:Advisors and Objectives @ 2010-10-18T20:30:22Z (update dated 17 September
2009), Update:Slayer Challenges @ 2010-10-18T20:40:14Z (dated 21 April 2010),
Update:Blog - Voice Overs: Look Who's Talking @ 2010-05-07T02:01:47Z (29 April
2010), Update:Patch Notes (24 June 2010) @ 2011-04-12T18:51:33Z, Update:Patch
Notes (9 November 2009) @ 2011-04-12T18:41:56Z, Speech @ 2011-05-22T06:02:11Z,
Explorer Jack @ 2011-05-05T00:58:40Z, Account creation @ 2011-04-05T16:01:03Z,
Destiny @ 2011-05-07T21:16:41Z, The Adventurer @ 2011-05-22T19:03:15Z, The Mentor
@ 2011-05-18T20:29:00Z, Harmony @ 2011-05-06T22:52:08Z.

**The 2011 wiki has no transcript page for UF** (`Transcript:Unstable
Foundations`, `Transcript:Sir Vant`, `Transcript:Roddeck`: no page; `--search
Transcript` finds none for UF). This is the largest gap: see section 3.

**Version.** The wiki walkthrough describes the 2009 release (exit up Roddeck's
ladder). Cache 786's own task text (section 3) describes the May 2011 version:
the exit is the ladder under **Explorer Jack's** house and the last step is
talking to Explorer Jack. Where they differ, the cache wins. (The wiki infobox
gives release "17 November 2009"; its body, the Tutorial page and
Update:Advisors and Objectives give 17 September 2009.)

---

## 1. Entry and exit

### 1.1 What 647 content does now

- `content/647/scripts/runescript/login_logout/scripts/login.rs2:40`: `if
  (%learning_the_ropes < 1000) { ~cc_open_interface(); return; }`.
- `cc_setup.rs2` `[proc,cc_open_interface]`: opens `playerdesign4` (interface
  1028) top-level, `allowdesign(^true)`.
- `cc_name.rs2:114-121` `[queue,character_created]`: `%learning_the_ropes =
  1001`, applies the design, opens the game frame. **No teleport**: the player
  stays where the account was created.
- `%learning_the_ropes` is varp 281 (`content/647/pack/varp.pack:282`, perm in
  `content/647/scripts/runescript/player/configs/perm.varp:3`, `transmit=yes` in
  `all.varp`).

### 1.2 What cache 786 says varp 281 is

Varp 281 is UF's progress varp (cache):

| Source | What it shows |
|---|---|
| quest config 131 (`all.quest:669-672`) | `op1_str2=Unstable Foundations`, `op3_counted_g2_g4_g4=1,281,5,1000`: varp 281, started at 5, complete at 1000 (same layout as Cook's Assistant `1,29,1,2`, Dragon Slayer `1,176,1,10`). `op9_g1=1` (inference: quest points; it reads 1/2/3 for Cook's Assistant/Dragon Slayer/Desert Treasure, their QP). |
| cs2 2193 (quest list) | `case 147: return(~script_2157(%varp281, 1000))` (cs2 2158: <1 not started, <1000 started, else done) |
| cs2 4098 | maps varp 281 to the current tutorial Task id (table in 3.1); `case 1000 -> 3521`, `default -> 4094` |
| cs2 3227 | Task completion thresholds on varp 281 (table in 3.1) |
| loc 45604 (`all.loc`, multivarp=281) | varp 281 0..74 -> loc 45602, >=75 -> loc 45601 (the low-wall/ramp spot, section 2) |
| cs2 175, 192, 943, 1364, 1888, 2168, 2169, 2778, 3971, 973 | `< 1000` / `>= 1000` gates: "You can't do this while in the tutorial." (175), "More advanced grouping options will be available when you've finished the Tutorial." (943), "More advanced sorting options are available after the Tutorial." (2168), low-health message only `>= 1000` (1888), "You've now reached the highest skill level that you can achieve during the tutorial. ..." at stat base 3 (973) |
| cs2 1522, 1558, 2162, 3968, 3977 | **`= 1000` exactly** |
| cs2 2464 (`> 3`), 2773 (`< 6` returns), 2785 (`< 93`, `< 120`), 1886 (`< 125`), 3224 (`< 135`, `< 140`) | stage gates inside the tutorial |

**Done value is 1000** (quest config 131, cs2 2193, cs2 4098). **1001, which
`cc_name.rs2:116` sets, is not a value the cache uses**: it passes every `>=
1000` check but fails the `= 1000` checks, e.g. cs2 4098 returns 4094 (blank
task) instead of 3521 "The Journey Begins", and cs2 3977's task panel skips its
`%varp281 = 1000` branches. Recommendation for the build: UF sets 1000 at its
end; character creation sets UF's first stage instead of 1001 (stage values in
3.1). Whether any 647 content relies on 1001: grep finds only `cc_name.rs2:116`
and the `< 1000` checks in `login.rs2:40` and `character_creation.rs2:6`.

Other UF vars (cache, `all.varbit`):

| var | bits | used by |
|---|---|---|
| varbit 4895 | varp 1255 bits 0-1 | npc 8854 multinpc: value 3 -> `sir_vant_8848`, else hidden (Falador Sir Vant, spawn below); cs2 1886, 1661, 3971 test `= 3`; cs2 1522/1523 test `= 2` for interface 761 "Sir Vant's Requests" (Learning the Ropes, `761.if` text "This quest was originally known as 'Learning the Ropes'."). Inference: 3 marks an account that took UF, 2 a Learning the Ropes account. |
| varbit 6487 | varp 1254 bits 22-23 | loc 45795 (dragon's food bowl): 0 -> loc 1758 (no op), else -> loc 1761 (op1 Search) |
| varbit 6494 | varp 1261 bits 0-2 | cs2 3227: Task 3520 "Introducing...Explorer Jack" done when `> 2`. void-634 names it `introducing_explorer_jack_task`, 3 = completed (`data/achievement/achievement_task.varbits.toml:87`) |
| varbit 6495 | varp 1261 bits 3-4 | locs 45571/45574/45577/45580/45583 (rocks): 0 -> none, 1 -> mineable rock, else -> plain rock; locs 45586/45589/45592 (tree roots): 0-2 -> none, 3 -> choppable root, else plain. Inference: the escape-route choice (1 = mining, 3 = woodcutting). void-634's Tasks.kt:154-157 tests it the same way. |
| varbit 6496 | varp 1261 bits 14-16 | loc 45564 (pillar 1) state 0-4 -> locs 45566-45570 |
| varbit 6497 | varp 1261 bits 17-19 | loc 45565 (pillar 2) state 0-4 -> locs 45566-45570 |
| varbits 6488-6493, 6498, 6499 | varp 1254 bits 24-30, varp 1261 bits 20-31 | no cache reader found (no npc/loc multivar, no cs2 read): purpose unknown |
| varc 1007 | client var | cutscene step; drives the camera (section 4) |

void-634 calls varp 281 `unstable_foundations` with values unstarted=0,
incomplete=1, completed=1000 (`data/quest/free/unstable_foundations/unstable_foundations.varps.toml`).

### 1.3 Where the new player goes, and where they leave

- **Into**: the cellar's starting room (struct 1702 below: "Pan the camera round
  until you can see the knight at the door of the room you are in."). Exact start
  tile: **no source found**. The first cutscene camera (varc 1007 = 1) sits at
  local (32,13) of square 57_77 looking north to (32,20)-(32,25) (section 4).
- **Out of**: "Leave the cellar via the ladder under Explorer Jack's house."
  (struct 1723). The only climbable ladder in square 57_77 is loc 45784
  (op1 Climb) at `l57_77.jm2:2458` `0 6 21` = (3654,4949,0) (inference: the exit
  ladder). Explorer Jack's house trapdoor is loc 45800 (op1 Open) at
  `l50_50.jm2:5610` = (3207,3239,0); Explorer Jack (npc 13295) spawns at
  (3204,3241) (`content/647/maps/spawns/n50_50.jm2:3`; void-634 the same). The
  arrival tile after climbing: **no source found**.
- After UF: Sir Vant appears on Falador Castle's ground floor (wiki Sir Vant),
  matching npc 8854 (varbit 4895 = 3) at `spawns/n46_52.jm2:22` = (2960,3350).
  Roddeck lives in Lumbridge, npc 8867 at (3232,3237) (`spawns/n50_50.jm2:50`).

---

## 2. The area: square 57_77 (x 3648-3711, z 4928-4991)

Found by the UF locs' placements (`l57_77.jm2`); local (lx,lz) = (x-3648,
z-4928). No npc spawns exist for this square (no `spawns/n57_77.jm2`, no cache
`n57_77`): every UF npc is placed by script (inference: per player, see 8.3).

### 2.1 Npcs (cache `all.npc`, ids from `content/647/pack/npc.pack`)

The `readyanim=` field in the 647 npc unpack is opcode 127, the **BAS id**
(`rs-pack/src/pack647/npc.rs:1363-1367`, misnamed through `name_for_seq`).

| id | name | op | BAS (stand, walk) | notes |
|---|---|---|---|---|
| 8848 | Sir Vant | Talk-to | 1613 (9956, 9957) | model 49346, head 49344 |
| 8849 | Sir Vant | Talk-to | 1613 | model 49345 |
| 8850 | Sir Vant | Talk-to | 1614 (12235, 9957) | |
| 8851 | Sir Vant | Talk-to | 1615 (12234, 9957) | |
| 8852 | Sir Vant | Talk-to | 1616 (12233, 9957) | |
| 8853 | Sir Vant | Talk-to | 1617 (808, 12280) | |
| 8854 | (multi) | Talk-to | | varbit 4895: 3 -> 8848 |
| 8857 | Squire | Talk-to | 1621 (12228, 12227) | examine "A servant of Sir Vant." (wiki) |
| 8858, 8859 | Dragon | Attack | 1618 (12249, 12247) | size 5 |
| 8860 | Dragon | Attack | 1619 (12253) | size 5 |
| 8861 | Dragon | Attack | 1620 (12255) | size 5 |
| 8862 | Goblin | Attack | 86 | vislevel 2 (wiki: level 2, 50 LP, max hit 16) |
| 8863 | Roddeck | Talk-to | 1617 (808, 12280) | |
| 8867 | Roddeck | Talk-to | 1622 | the Lumbridge spawn |
| 13295 | Explorer Jack | | | Lumbridge spawn |

Which Sir Vant/Dragon/Roddeck variant serves which step: **no source found**
(the BAS stand anims differ; their meaning is unknown). Older Sir Vants 7938,
7939, 7942 (spawns `n39_78.jm2:1`, `n51_77.jm2:1`) are Learning the Ropes'
cellars, not UF (wiki Sir Vant: pre-UF accounts can still visit him; void-634
calls 7942 `sir_vant_unstable_foundations`, which the cache does not support).

### 2.2 Locs (cache `all.loc`, placements `l57_77.jm2`)

| loc | name / ops | where (local -> world) | line |
|---|---|---|---|
| 45783 | Sir Vant's bag, Search | (35,13) -> (3683,4941) | 2457 |
| 45785 | Ladder (no op) | (32,12) -> (3680,4940); (6,45) -> (3654,4973) | 2460, 2459 |
| 45786 | Ladder (no op, anim seq 12275, darkened recolours) | not placed | |
| 45784 | Ladder, Climb | (6,21) -> (3654,4949) | 2458 |
| 45787 | Ladder (no op) | (56,4) -> (3704,4932) | 2461 |
| 45603 | Low wall, Climb-over (forceapproach 10) | level 1 (35,16) -> (3683,4944,1); level 1 (59,8) | 111, 112 |
| 45604 | multivarp 281: <75 -> 45602, >=75 -> 45601 | (35,16); (59,8) | 113, 114 |
| 45605 | (no op) | (35,17); (59,9) | 115, 116 |
| 45606-45612 | wall pieces; 45608/45609/45610/45612 "Wall", Climb-over | (28..36,16); (57,8),(58,8),(60,8) | 117-126 |
| 3550 | inviswall_serverside | (32,16), (35,16), (59,8) | 39-41 |
| 45563 | Stone pillar | (32,11); (56,3) | 90, 91 |
| 45564 | pillar 1 (varbit 6496 -> 45566..45570 "Wooden pillar"; 45569 anim 12242, 45570 anim 12243) | (24,31) -> (3672,4959) | 92 |
| 45565 | pillar 2 (varbit 6497, same states) | (24,39) -> (3672,4967) | 93 |
| 45795 | Dragon's food bowl (varbit 6487 -> 1758 / 1761 Search) | (17,35) -> (3665,4963) | 2468 |
| 45789 | Cupboard door, Open (45790 Close) | (37,30) -> (3685,4958) | 2463 |
| 45791-45794 | cupboard frame pieces | (37,29..31), (38,30) | 2464-2467 |
| 45571, 45574, 45577, 45580 (x3), 45583 (5x5) | rocks (varbit 6495) | (17,14), (20,7), (16,8), (17,6)/(17,11)/(22,16), (18,9) | 94-103 |
| 45586 (x2), 45589 (x2), 45592 | tree roots (varbit 6495) | (28..32,45) | 104-108 |
| 45595 | Rock, Mine (blockage) -> 45596 (anim 12279) -> 45597 Hole, Climb-through | (14,11) -> (3662,4939) | 109 |
| 45598 | Roots, Chop (blockage) -> 45599 (anim 12281) -> 45600 Hole, Climb-through | (14,14) -> (3662,4942) | 110 |
| 45788 | Egg shell | (37,45) | 2462 |
| 49100 | Crate, Search | (42,35) | 2473 |
| 49101 | Bookcase, Search (void-634: `roddecks_bookcase`) | (49,36); also Lumbridge (3232,3236) `l50_50.jm2:5613` | 2474 |
| 45654, 45655 | Barrel | several | 1660-1670 |
| 46311-46316 | Planks (members ops Take-plank) | not placed in 57_77 | |

The rock/hole and root/hole chains are by id order and anim only (inference).
The plank the player carries is obj 15276; the plank loc on the floor before
pickup: **no source found** (no Plank loc is placed in 57_77; loc 3231 `plank`
is not placed there). The start room appears twice (around (32..36,11..16) and
(56..60,3..9)); why: **no source found**.

### 2.3 Objs (cache `all.obj`, ids `obj.pack`)

| id | name | inv op |
|---|---|---|
| 15276 | Plank | |
| 15277 | Drugged meat | Eat |
| 15278 | Cloth | Rip |
| 15279 | Oily cloth | Rip |
| 15280 | Rag | |
| 15281 | Oily rag | |
| 15282, 15283 | Fuse | |
| 15284 | Sir vant's oil can | |
| 15285 | Root chippings (stackable) | |
| 15286 | Stone clippings (stackable) | |
| 15671 | Manual | Read |
| 15672 | Roddeck's diary | Read |

The squire's gear (wiki: bronze platebody, bronze sq shield, bronze sword;
Sir vant's oil can page says "Bronze longsword") and tinderboxes: which obj ids
**no source found**.

---

## 3. Steps and dialogue

### 3.1 The steps, from cache 786

The client's Task System shows the tutorial as Tasks of area 60
(`param_1267=60` in structs 1702-1725; void-634 names area 60
`unstable_foundations`, `achievement_task.varbits.toml:49`). cs2 4098 picks the
current Task from varp 281; cs2 3224 decides which are shown, cs2 3227 when each
is done. Titles and texts are word for word from the structs (`all.struct`,
`param_1266` title, `param_1273` text, `param_1274..1278` hints); the varp 281
values are from cs2 4098 / 3227 / 3224.

| varp 281 | Task id | struct | Title | Done when (cs2 3227) |
|---|---|---|---|---|
| (always shown) | 3522 | 1702 | Rotate Your Camera | varp281 >= 7 |
| 7, 9 | 3500 | 1703 | Talk to Sir Vant | > 7 |
| 10 | 3501 | 1704 | Assist Sir Vant | > 10 |
| 15 | 3502 | 1705 | Gather the Squire's Gear | > 15 |
| 20, 25, 30 | 3503 | 1706 | Equip Yourself | > 30 |
| 35, 40 | 3504 | 1707 | Kill the Goblin | > 40 |
| 45-49 | 3505 | 1708 | Find Out Sir Vant's Plan | > 49 |
| 50, 55, 56, 57 | 3506 | 1709 | Check your Quest Journal | > 55 |
| 60 | 3508 | 1710 | Check your World Map | > 60 |
| 62 | 3509 | 1711 | Realign your Minimap | > 62 |
| 65, 75 | 3507 | 1712 | Climb the Wall | > 75 |
| 70 | 3523 | 1713 | Make a Ramp | > 70 |
| 80, 82, 83, 90 | 3510 | 1714 | Put the Dragon to Sleep | > 90 |
| 93, 95 | 3511 | 1715 | Talk to Sir Vant Again | > 95 |
| 96, 98, 100, 105, 110, 115 | 3512 | 1716 | Sabotage! | > 115 |
| 120, 123, 125, 126, 127 | 3513 | 1717 | Return the Squire's Gear | > 127 |
| 128..165 (130,135,...,165) | 3514 | 1718 | Discover an Exit | > 165 |
| shown >= 135 | 3515 | 1719 | Cutting a Path | > 160 |
| shown >= 135 | 3516 | 1720 | Breaking Through | > 160 |
| shown >= 140 | 3517 | 1721 | Skill in Woodcutting | stat_base(woodcutting) > 1 |
| shown >= 140 | 3518 | 1722 | Skill in Mining | decompile reads stat_base(woodcutting) > 1 (as written; likely a decompile or cache quirk, unverified) |
| 170 | 3519 | 1723 | To Freedom! | > 170 |
| 1000 | 3520 | 1724 | Introducing...Explorer Jack | varbit 6494 > 2 |
| 1000 | 3521 | 1725 | The Journey Begins | |

The exact values the server sets inside each band (e.g. 25 vs 30) and what
each sub-value means: **no source found**. Values 0-6 before "Talk to Sir
Vant": quest started at 5 (quest config 131), the camera-controls overlay needs
>= 6 (cs2 2773); 1-4: **no source found**. void-634's Tasks.kt:154-157 add
varbit 6495 conditions to 3515-3518 (void-634's; the cs2 3224 decompile shows
only the varp 281 tests).

Task texts (word for word, `<br>` as in the cache):

- **1702 Rotate Your Camera**: "Welcome to RuneScape.<br><br>You control your camera view using the arrow keys.<br><br>Pan the camera round until you can see the knight at the door of the room you are in."
- **1703 Talk to Sir Vant**: "The knight, Sir Vant, has information for you.<br><br>To talk to him and find out more, click on him (or click on him and select 'Talk To' if you have a one-button mouse)."
- **1704 Assist Sir Vant**: "Sir Vant needs your help - talk to him to find out more.<br><br>For more information on this Task, click on the Hints tab above." Hints: "The Hints tab offers further information that will help you complete many Tasks." / "Some Hints have a button next to them which you can click to point you to a useful place, thing or person." / "For now, simply click on Sir Vant to find out what he needs from you."
- **1705 Gather the Squire's Gear**: "Pick up the bronze armour and weapon from the spot where Sir Vant's squire was incinerated.<br><br>For more information on this Task, click on the Hints tab above." Hints: "To pick up an object, you may right-click on it where it lies and choose 'Take', or simply left-click on it." / "Items you pick up go into your inventory."
- **1706 Equip Yourself**: "Wield the bronze armour and weapon you have acquired.<br><br>For more information on this Task, click on the Hints tab above." Hints: "To wear an object, right-click on it in your inventory and choose 'Wear' or 'Wield'." / "Most weapons and armour can be equipped by simply left-clicking on them in your inventory." / "You can see what you are currently wearing, and what effect it has on your character, via the Worn Equipment tab on the side bar."
- **1707 Kill the Goblin**: "Engage and kill the goblin currently looting Sir Vant's supplies.<br><br>For more information on this Task, click on the Hints tab above." Hints: "You must have gathered and equipped all three pieces of the squire's gear to engage this goblin." / "Right-click on the goblin and choose 'Attack' to begin combat. You may also left-click the enemy to attack it." / "Once combat begins, you will continue to attack your opponent until one of you dies or flees." / "This goblin poses little threat, but other enemies will be considerably tougher. You can get a rough estimate of an enemy's strength by looking at the level displayed next to their name when you right-click on them."
- **1708 Find Out Sir Vant's Plan**: "Talk to Sir Vant again to find out what he needs you to do next."
- **1709 Check your Quest Journal**: "Open the Quest Journal interface to see your current quest progress. Click on an entry in the list to see what you should be doing to progress in that quest." Hints: "You are now on a quest to deal with the dragon. For more information, click on the Quest List tab on the side bar..." / "...and then on the Unstable Foundations entry to view your Quest Journal."
- **1710 Check your World Map**: "Before you go off to drug the dragon, check your surroundings. The minimap in the top-right of the screen gives a general picture; for more detail, open the world map by clicking on the globe icon." Hints: "Check the minimap in the top-right corner of the screen to see a plan of the cellar." / "For a more complete picture of the area, click on the globe button next to the minimap." / "The world map will show you many other places once you are free of this cellar."
- **1711 Realign your Minimap**: "Set your camera view and minimap alignment back to face North, by clicking on the compass icon next to your minimap." Hint: "Clicking on the compass arrow on the other side of the minimap from the World Map button will cause your camera and minimap to face due North."
- **1712 Climb the Wall**: "You must get past the dragon.<br><br>Take some drugged meat from Sir Vant's bag, then find a way to climb over the broken wall near Sir Vant into the main room of the cellar.<br><br>For more information on this Task, click on the Hints tab above." Hints: "Pick up some drugged meat from Sir Vant's bag by clicking on it." / "The wall is too high to climb without aid." / "There is a plank next to the wall; click on it to pick it up."
- **1713 Make a Ramp**: "The wall is too high to climb without aid. Click on the plank in your inventory to select it, then on the wall to construct a ramp.<br><br>For more information on this Task, click on the Hints tab above." Hints: "In your inventory, click on the plank to select it (or right-click and select 'Use')." / "With the plank selected, click on the low wall to place it. Once you have placed the plank down, you will be able to climb the wall by clicking on it."
- **1714 Put the Dragon to Sleep**: "Search the cellar to locate the dragon's food bowl and put the drugged meat in it." Hints: "Explore the cellar until you find the food bowl." / "Use the food on the bowl to aid Sir Vant with his plan (make sure to right-click and select 'Use')."
- **1715 Talk to Sir Vant Again**: "Sir Vant has joined you by the pillars. Talk to him to discover the next step of the plan." Hint: "Sir Vant has moved; you can now find him by the wooden pillar just south-east of the dragon's nest."
- **1716 Sabotage!**: "Set Sir Vant's trap by tying an oily rag around each pillar and adding a fuse.<br><br>Once you have done this, give Sir Vant a tinderbox and light the fuse on the right.<br><br>For more information on this Task, click on the Hints tab above." Hints: "To rip the cloth into strips, simply left-click on it in your inventory." / "Use each cloth on the oil can to soak it in oil, then on a pillar to set in place." / "Once the cloth is in place, use the fuse on it to ready the trap." / "You can then give a tinderbox to Sir Vant by clicking on it in your inventory and using it on him." / "Once all is ready, use the tinderbox on the pillar to light the fuse, then run for cover!"
- **1717 Return the Squire's Gear**: "Sir Vant will need to return the squire's weapons and armour to the family. Take them off and give them to him." Hints: "Take off your armour and weapons by opening the worn equipment tab and clicking on them." / "Right-click on the items in your inventory and choose Use, then click on Sir Vant to hand back the kit."
- **1718 Discover an Exit**: "The dragon's breath burned away the ladder to the surface.<br><br>Locate another way out - the man you rescued may be able to advise you." Hints: "The ladder in the starting room is gone, but you may be able to use the ladder in the cellar of the connected house, if you can reach it." / "You'll need to exploit a weak spot in the wall between the two cellars in order to make further progress." / "Talk to Roddeck, the man who just came out of the cupboard, to seek his input."
- **1719 Cutting a Path**: "Use a woodcutting hatchet to clear the roots blocking the right-hand path out of the cellar.<br><br>You now have two Tasks: click on the Back button above or the Task System icon to view both, then on one of the icons displayed to find out more." Hint: "The exit route you have chosen is blocked by thick roots. In order to clear them, you will need a hatchet; simply click on them to attempt to cut through."
- **1720 Breaking Through**: "Use a mining pickaxe to clear the rubble blocking the left-hand path out of the cellar.<br><br>You now have two Tasks: ..." (same second sentence). Hint: "The exit route you have chosen is blocked by rubble. In order to clear it, you will need a pickaxe; simply click on the rubble to attempt to break through it."
- **1721 Skill in Woodcutting**: "Clearing the roots requires level 2 Woodcutting. Find a way to attain this; Roddeck may have some useful advice." Hints: "It seems that cutting through a tangle of roots is a harder job than it looked. You'll need to get in some practice in order to be up to the task." / "On another wall nearby is a spot where weaker roots have breached the cellar walls. Click on them, or use your hatchet on them, to cut them and practice the art of woodcutting." / "Each root knole you successfully chop will yield some chippings and 42 experience. The chippings are useless to you and can be safely dropped on the floor using their right-click option." / "Gaining 83 experience from cutting the roots will allow you to reach level 2 and be ready to deal with the major root blockage."
- **1722 Skill in Mining**: as 1721 for mining: "Clearing the rubble requires level 2 Mining. ..." / "It seems that smashing through solid rock is a harder job than it looked. ..." / "On the ground nearby is a spot where stone has fallen from the ceiling. Click on it, or use your pick on it, to hack at it and practice the art of mining." / "Each lump of rock you successfully shatter will yield some chippings and 42 experience. ..." / "Gaining 83 experience from breaking the stone will allow you to reach level 2 and be ready to deal with the major rubble blockage."
- **1723 To Freedom!**: "Leave the cellar via the ladder under Explorer Jack's house." Hints: "By clearing a path into Explorer Jack's cellar you have opened a route to the outside world." / "Depart up the ladder to the surface."
- **1724 Introducing...Explorer Jack**: "Talk to Explorer Jack about the world outside." Hint: "Explorer Jack can tell you more about the Tasks and other possible goals available to you outside. Talk to him before you go."
- **1725 The Journey Begins**: "You may now leave Explorer Jack's house and begin your own explorations." Hint: "Good luck and have fun!"

All carry `param_1292` "Completing this Task will help you escape from the
cellar in which you find yourself." Interface 1080 adds "Once out of the
cellar, you may 'pin' a Task and refer to it later with this button." (cache
`1080:54`).

Hint buttons (`param_1282..1289`, type coord): every UF value decodes to
x=6336, z=6337..6383 (e.g. 103815399 -> (6336,6375)); 6337 and 6338 are given
to "click on the plank in your inventory" and "open the worn equipment tab", so
they are keys, not tiles (inference). What the server does for each key: **no
source found**.

The first-step "Controls" overlay: interfaces 897 and 1079 (identical; texts
"Controls", "Arrow keys rotate the camera.", "Click on the ground or minimap to
walk.", "CTRL + click to run.", "Please rotate the camera to begin.";
`1079:29`). Component 0 binds the four arrow keys (`opkey1..4=96..99,0,1`)
to ops 1-4 with `transmitop=1,2,3,4`; onop cs2 2773 lights the pressed key
(returns while varp281 < 6). How the server learns the camera was rotated
(inference: the ops reach the server as IF_BUTTON1-4 on 1079:0 / 897:0;
unverified), and which slot the overlay opens into: **no source found**.

### 3.2 Story (wiki, paraphrase; the cache steps above are the authority)

From Unstable Foundations @ 2011-05-05T15:38:45Z unless noted:

1. Character creation (Account creation @ 2011-04-05T16:01:03Z; 647 already has
   it).
2. Talk to Sir Vant. "A cutscene will follow, with Sir Vant attempting to defeat
   the dragon and a puff of stray dragonbreath killing Sir Vant's squire." The
   squire's gear "drops to the floor from where they used to be" (Squire (Sir
   Vant)). The squire, if talked to before, "will tell the player to speak to
   Sir Vant" (paraphrase only).
3. A goblin rummages in Sir Vant's pack; kill it in the squire's gear. Goblin:
   "He only hits back once per player's hit. He never moves out of his square. If
   you stop attacking him, he would go back to rummaging ... He could only be
   attacked while wearing all the deceased squire's gear ... otherwise Sir Vant
   would stop you." (Goblin (Unstable Foundations)).
4. Drugged meat in the dragon's food bowl, over the wall by plank ramp. "If you
   try to walk in front of the dragon Sir Vant would shout a warning and you
   would automatically run away." "After you use the drugged meat on the bowl,
   the dragon smells the meat and your character will run to a hole. Sir Vant
   will shoo the dragon away, causing it to walk up to the meat, eat it, and fall
   asleep." A wrong bowl makes Sir Vant scratch his head (Dragon's food bowl;
   that page also says only one bowl remains after an update, matching the one
   loc 45795).
5. Gather from the pack, oil and rip the cloth, rags and fuses on both pillars,
   tinderbox to Sir Vant, light the far pillar; "The ceiling will not collapse
   until the player has run to safety."
6. Roddeck comes out of his cupboard; choose mining or woodcutting, get a
   bronze pickaxe or bronze hatchet; train to level 2 on the side rocks/roots
   (42 xp each, two needed); clear the blockage, get stone clippings / root
   chippings (Roddeck has you drop them: Stone clippings).
7. Leave (cache: up the ladder under Explorer Jack's house, talk to Jack).

### 3.3 Dialogue: every line any source gives

| Speaker | Line (word for word) | When | Source |
|---|---|---|---|
| Sir Vant | "Are you mad? Don't eat that! It smells vile, it's completely raw, and I'm sure I mentioned that it's been treated with sleeping potion!" | eat or drop drugged meat | Drugged meat @ 2011-04-23T16:14:08Z |
| Sir Vant | "Perhaps you shouldn't throw that away" | drop fuse, rag, oily rag, oily cloth, cloth, oil can | the six item pages' `destroy` field |
| Sir Vant | "Hey, I'm over here" | unknown | Speech @ 2011-05-22T06:02:11Z (audio caption) |
| Goblin | "Ooh, shiny!" | goblin at the pack | Unstable Foundations (image caption) |
| Goblin | "Me get rich!" | unknown | Update:Blog - Voice Overs (29 April 2010) |
| Roddeck | "...an understanding chap; he won't mind us walking through his house" (fragment, about Explorer Jack) | exit | Explorer Jack @ 2011-05-05T00:58:40Z |
| (message) | "You won't be needing that again." | picking up a dropped Roddeck's diary | Roddeck's diary @ 2011-03-13T20:32:26Z |

**Dialogue sources, by the user's decision (AGENT_BRIEF.md, "Reworked
quests", 2026-11-04):** 2011-quoted lines first; then void-634's, commented as
void-634's; then 289's, commented as 289's. 289 (2005) has no UF. void-634's
lines touching UF, all `game/src/main/kotlin/content/area/misthalin/lumbridge/`:

| Speaker | Line (void-634's, word for word) | When (void-634's) | File |
|---|---|---|---|
| Explorer Jack | "Ah! Welcome to ${Settings["server.name"]}, lad. My name's Explorer jack. I'm an explorer by trade, and I'm one of the Taskmasters around these parts" (void's server-name placeholder; "jack" lower-case as void has it) | first talk, Task 3520 not done | `ExplorerJack.kt:34` |
| player | "Taskmaster? What Tasks are you Master of?" | then | `ExplorerJack.kt:35` |
| Explorer Jack | "Well, the Task System is a potent method of guiding yourself to useful things to do around the world." | then (`whatIsTaskSystem`) | `ExplorerJack.kt:100` |
| Explorer Jack | "You'll see up to six Tasks in your side bar if you click on the glowing Task List icon. You can click on one for more information about it, hints, waypoint arrows, that sort of thing." | then | `ExplorerJack.kt:101` |
| Explorer Jack | "Every Task you do will earn you something of value which you can claim from me. It'll be money, mostly, but the Rewards tab for a Task will tell you more.<br>Good luck!" | then; void sets varbit 6494 completed and varp 281 = 1000 | `ExplorerJack.kt:102-104` |
| Explorer Jack | "I say, there's nothing interesting in my cellar! Better go exploring elsewhere, eh?" / player "What's down there?" / "Crates, boxes, shelves - nothing you won't see in dozens of houses across Runescape. Go on, explore somewhere else!" | his trapdoor's Open (not UF's exit, which comes up from below) | `ExplorerJack.kt:93-95` |

void-634 also has a commented-out "What ho! Where did you come from?" exchange
about Roddeck's cellar (`ExplorerJack.kt:74-87`: commented in void, so not
void's running behaviour; it names `${World.name}` and an Achievement Diary
icon), Roddeck's Lumbridge talk after UF (`roddecks_house/Roddeck.kt`: "Greetings!
I am Roddeck. What can i do for you today?" and the Advisor lines; his house,
not the cellar), and the books' full text
(`data/quest/free/unstable_foundations/unstable_foundations.books.toml`:
Roddeck's Diary, "On The Care Of Dragons"). void-634 has no cellar dialogue:
no Sir Vant, squire, goblin, dragon or in-cellar Roddeck (grep of
`game/src/main/kotlin` and `data` for vant, roddeck, unstable).

**Every other line of UF dialogue:** no 2011 page or void-634 source; section 3.4 gives them
from 32 dated recordings. The dialogue was voiced
(Update:Slayer Challenges, 21 April 2010: "Voices have been implemented for all
NPCs (and the player) within the tutorial"), so cache 786's vorbis archive may
hold the spoken lines; no cache table maps lines to vorbis ids (no enum or
param of a vorbis type; void-634 has none either): **no source found** for
which vorbis ids are UF's. The build must stop at the dialogue until a source
is chosen.

The quest journal's text (server-sent in 647): **no source found**.

### 3.4 Dialogue from 32 dated recordings (2010-11-27 to 2011-07-28)

Every dialogue state 2010-11 players saw, from every dated YouTube recording found.
- 32 were decoded; 31 had readable dialogue (_HI5QDQYhrc is 322x240).
- 16 more were checked and left out: other content, character creation only, or no cellar. Method: the `recordings-matrix` skill
(`.claude/skills/recordings-matrix`). Each recording's dialogue box was cut into states, OCR'd,
and matched to the lines below. What didn't match was read by eye and added, then the matrix
was run again.

Each line gives:
- `n/N`: how many recordings show it;
- the first sighting by upload date (YouTube id @ second, to open at that moment);
- the states players went to next. A state with two or more common exits is a branch.

Text is word for word as recorded, with breaks joined; `<br>` placement still needs a frame per
line. Speakers are as the box names them (`PLAYER` is the player's name). This section
supersedes 3.3's "no source found" for the cellar dialogue.

- `s001` chatplayer 11/31 (first: 20101215 `kjgw48iBdc0` @72s): **PLAYER**: Hello!
- `s001a` chatnpc 1/31 (first: 20101223 `7_neJ6mCLXc` @41s): **Squire**: Oh dear, oh dear, it's coming. You'd better talk to Sir Vant. He's in charge.
- `s002` chatnpc 20/31 (first: 20101215 `kjgw48iBdc0` @75s): **Sir Vant**: Wait a moment - there's a dragon coming!
  - next: `s003` x19; once: `s005`
- `s003` chatnpc 23/31 (first: 20101127 `IwS6rBpkcyE` @39s): **Sir Vant**: I need some help over here!
  - next: `s004` x19; `s007` x2; once: `s055`, `s005`
- `s004` chatnpc 21/31 (first: 20101127 `IwS6rBpkcyE` @45s): **Sir Vant**: Thank goodness you're here! My squire's dead, and there's a goblin going through my bag!
  - next: `s005` x19; `s007` x2
- `s005` chatnpc 21/31 (first: 20101127 `IwS6rBpkcyE` @47s): **Sir Vant**: I can't turn my back on this dragon. I need you to take my squire's equipment and deal with the goblin!
  - next: `s007` x15; `s006` x3; once: `s010`, `s005a`
- `s005a` chatnpc 1/31 (first: 20110115 `E4MP65bl3sI` @200s): **Sir Vant**: Wait! You're not properly armed. Pick up the kit dropped by my squire, then attack the goblin when you've equipped it.
- `s006` mesbox 11/31 (first: 20101215 `kjgw48iBdc0` @244s): We'll come back to that in a moment.
  - next: `s013` x8; `s007` x3; once: `s016`, `s013b`, `s008`
- `s007` chatnpc 25/31 (first: 20101127 `IwS6rBpkcyE` @75s): **Sir Vant**: Thank you! Now to deal with the dragon. Come here, and I'll tell you my plan.
  - next: `s009` x21; once: `s016`, `s010`, `s008`, `s006`
- `s008` chatnpc 2/31 (first: 20110123 `fE0GdVcmJ9E` @285s): **Sir Vant**: I don't believe I gave you permission to go through my bag.
- `s009` chatnpc 23/31 (first: 20101127 `IwS6rBpkcyE` @81s): **Sir Vant**: This dragon's a tougher customer than I'd expected, but I know a way we can defeat it.
  - next: `s010` x22; once: `s011`
- `s010` chatnpc 24/31 (first: 20101212 `xqWnfqno_1M` @5s): **Sir Vant**: One corner of the ceiling is held up by wooden pillars. We're going to lure the dragon there and then collapse the roof on top of it.
  - next: `s011` x23; once: `s012`
- `s011` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @86s): **Sir Vant**: In my bag is some meat I've prepared with a sleeping potion. I need you to put the meat in the food bowl by the wooden pillars. The dragon will eat it and fall asleep.
  - next: `s012` x23; once: `s016`
- `s012` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @87s): **Sir Vant**: I'll keep the dragon's attention. Once you've got the drugged meat you'll need to climb over the wall next to me.
  - next: `s006` x10; `s013` x9; `s016` x2; once: `s020`, `s015`
- `s013` objbox 22/31 (first: 20101212 `xqWnfqno_1M` @31s): The bag contains several parcels of meat, all wrapped up in paper. You take one.
  - next: `s016` x15; `s015` x2; once: `s023a`, `s019`, `s018`, `s017`, `s014`
- `s013a` chatnpc 1/31 (first: 20110123 `fE0GdVcmJ9E` @328s): **Sir Vant**: Hey, don't just drop it on the floor! You're meant to be placing it in the food bowl.
- `s013b` mesbox 3/31 (first: 20110117 `VwpdbjfigD4` @239s): You should take some meat from Sir Vant's bag before you venture out.
- `s014` chatnpc 1/31 (first: 20110430 `4KO2h2xPtdM` @167s): **Sir Vant**: You'll never get over the wall there. Try going to the eastern end where it's lower.
- `s014a` chatplayer 1/31 (first: 20110117 `VwpdbjfigD4` @264s): **PLAYER**: The wall is too high for me to climb over, so I can't get past you.
- `s015` mesbox 4/31 (first: 20101127 `IwS6rBpkcyE` @140s): Even this low section of the wall is too high for you to climb over.
  - next: `s016` x3; once: `s014a`
- `s016` objbox 25/31 (first: 20101127 `IwS6rBpkcyE` @146s): You use the plank to construct a crude ramp.
  - next: `s017` x13; `s019` x6; `s013` x2; once: `s020`, `s016a`, `s013b`, `s013a`
- `s016a` chatplayer 1/31 (first: 20110115 `E4MP65bl3sI` @343s): **PLAYER**: There doesn't seem much point in messing around with this wall now that I've built a ramp.
- `s017` hint 15/31 (first: 20101127 `IwS6rBpkcyE` @161s): Your character can run if they have sufficient energy. You can make your character run automatically by turning on run mode. Click the boot icon by the minimap. If you hold down the CTRL key while clicking on the floor or minimap, your character will run to their destination.
  - next: `s019` x12; `s018` x3
- `s018` chatnpc 5/31 (first: 20110115 `E4MP65bl3sI` @312s): **Sir Vant**: Are you mad? Don't eat that! It's raw, and it's covered in sleeping potion!
  - next: `s019` x4; once: `s015`
- `s019` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @169s): **Sir Vant**: It's smelled the meat! Hide!
  - next: `s020` x22; once: `s022`
- `s019a` chatnpc 1/31 (first: 20110115 `E4MP65bl3sI` @354s): **Sir Vant**: I really don't think you should do that.
- `s020` chatnpc 25/31 (first: 20101212 `xqWnfqno_1M` @91s): **Sir Vant**: Well done. Come out of hiding and I'll tell you what we're doing next.
  - next: `s021` x23; `s022` x2
- `s021` chatnpc 23/31 (first: 20101212 `xqWnfqno_1M` @96s): **Sir Vant**: We need to destroy these pillars to collapse the ceiling on the dragon.
  - next: `s022` x21; once: `s026`, `s023`
- `s022` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @221s): **Sir Vant**: Could you get the rest of the supplies from my bag please?
  - next: `s023` x20; `s027` x3; once: `s022a`
- `s022a` chatplayer 1/31 (first: 20110117 `VwpdbjfigD4` @392s): **PLAYER**: I haven't searched it yet.
- `s022b` chatnpc 1/31 (first: 20110117 `VwpdbjfigD4` @395s): **Sir Vant**: Oh. Well, search my bag and bring the supplies to me before the dragon wakes up!
- `s023` objbox 23/31 (first: 20101127 `IwS6rBpkcyE` @234s): You search Sir Vant's bag and take a selection of supplies.
  - next: `s027` x15; `s024` x7; once: `s035a`, `s033c`
- `s023a` objbox 1/31 (first: 20101223 `7_neJ6mCLXc` @191s): The bag contains various supplies.
- `s024` chatnpc 8/31 (first: 20110117 `VwpdbjfigD4` @411s): **Sir Vant**: Now we're going to burn these wooden pillars and bring the ceiling down on this fat old worm!
- `s025` chatnpc 8/31 (first: 20110117 `VwpdbjfigD4` @417s): **Sir Vant**: First, we need to wrap the pillars in oily rags. There was a big cloth in my bag. Rip it in half, then soak the pieces in oil.
  - next: `s026` x7; once: `s032`
- `s026` chatnpc 8/31 (first: 20110117 `VwpdbjfigD4` @419s): **Sir Vant**: Wrap the oily rags around the pillars, then add a fuse to each one.
  - next: `s027` x6; once: `s029`, `s023`
- `s026a` chatnpc 1/31 (first: 20110128 `rX5vda5px3w` @66s): **Sir Vant**: How are you getting on?
- `s026b` menu 1/31 (first: 20110128 `rX5vda5px3w` @68s): What would you like to say? What am I meant to be doing? I'm getting there. Why don't you do it yourself?
- `s026c` chatnpc 1/31 (first: 20110128 `rX5vda5px3w` @73s): **Sir Vant**: Me? Touch oily rags? You've clearly never tried to clean oil off white armour!
- `s026d` chatnpc 1/31 (first: 20110128 `rX5vda5px3w` @78s): **Sir Vant**: No, I'll let you have all the fun.
- `s026e` menu 1/31 (first: 20110128 `rX5vda5px3w` @81s): What would you like to say? What am I meant to be doing? I'll get on with it.
- `s026f` chatplayer 0/31: **PLAYER**: I'm getting there.
- `s027` objbox 25/31 (first: 20101127 `IwS6rBpkcyE` @262s): You rip the cloth and end up with two rags.
  - next: `s028` x17; `s032` x4; `s033c` x2; once: `s059`, `s033`
- `s028` objbox 19/31 (first: 20101212 `xqWnfqno_1M` @115s): You soak the rag in oil.
  - next: `s032` x16; `s031` x2; once: `s033c`
- `s029` objbox 1/31 (first: 20110510 `Qh_-AMJpVd0` @308s): You soak the cloth in oil.
- `s030` objbox 0/31: You rip the oily cloth and end up with two oily rags.
- `s031` objbox 2/31 (first: 20110124 `Qzq-h8kLBHk` @186s): The rag has already been soaked in oil.
  - next: ; once: `s032`, `s028`
- `s031a` chatnpc 0/31: **Sir Vant**: You're supposed to soak the rag in oil first. Try using the rag with the oil can.
- `s032` objbox 24/31 (first: 20101127 `IwS6rBpkcyE` @268s): You wrap the oily rag around the pillar.
  - next: `s033` x39; once: `s033b`
- `s033` objbox 25/31 (first: 20101127 `IwS6rBpkcyE` @279s): You attach the fuse to the pillar.
  - next: `s032` x14; `s035` x13; `s034` x10; `s033b` x3; once: `s051`, `s035a`, `s034a`, `s026a`
- `s033a` chatnpc 1/31 (first: 20110307 `p7ufqPwAXLg` @464s): **Sir Vant**: Wait a minute! You're supposed to get both pillars prepared before we light them.
- `s033b` chatnpc 4/31 (first: 20101223 `7_neJ6mCLXc` @286s): **Sir Vant**: I don't think I need that just at the moment, thank you.
  - next: ; once: `s033c`, `s033a`, `s033`, `s032`
- `s033c` mesbox 5/31 (first: 20101223 `7_neJ6mCLXc` @267s): Nothing interesting happens.
  - next: `s032` x2; once: `s033`, `s028`, `s027`
- `s034` chatnpc 10/31 (first: 20101127 `IwS6rBpkcyE` @289s): **Sir Vant**: Thanks, that's perfect. Now, hand me a tinderbox.
  - next: `s035` x8; once: `s036`, `s023`
- `s034a` chatnpc 1/31 (first: 20101223 `7_neJ6mCLXc` @299s): **Sir Vant**: You'll need to give me a tinderbox first - otherwise, I can't light my pillar at the same time as you light yours.
- `s035` objbox 24/31 (first: 20101127 `IwS6rBpkcyE` @296s): You offer Sir Vant a tinderbox.
  - next: `s036` x23; once: `s037`
- `s035a` chatnpc 2/31 (first: 20110115 `E4MP65bl3sI` @483s): **Sir Vant**: Just hand me one of the tinderboxes from my bag.
- `s036` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @299s): **Sir Vant**: Thank you. When you're ready, set fire to the other pillar. I'll light this one at the same moment.
  - next: `s037` x19; `s038` x5
- `s037` chatnpc 20/31 (first: 20101212 `xqWnfqno_1M` @153s): **Sir Vant**: It's burning! Run!
  - next: `s038` x15; `s041` x5
- `s038` chatnpc 20/31 (first: 20101127 `IwS6rBpkcyE` @340s): **Sir Vant**: Ah, here you are, Roddeck. The White Knights got your message and sent me to slay your dragon.
  - next: `s039` x19; once: `s041`
- `s039` chatnpc 19/31 (first: 20101212 `xqWnfqno_1M` @184s): **Roddeck**: Thank you both very much! It's had me trapped in the cupboard for ages. I'd completely lost control of it.
  - next: `s040` x16; `s041` x3
- `s040` chatnpc 16/31 (first: 20101212 `xqWnfqno_1M` @185s): **Sir Vant**: Next time, get a puppy!
  - next: `s041` x15; once: `s043`
- `s041` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @357s): **Sir Vant**: Before we go any further, could I have my squire's kit back, please? I ought to return it to the poor chap's family. I'd like my tinderbox and oil can too.
  - next: `s043` x14; `s044` x3; `s041b` x3; `s041c` x2; once: `s042`, `s041a`
- `s041c` chatnpc 2/31 (first: 20101223 `7_neJ6mCLXc` @358s): **Sir Vant**: I'd like my squire's kit back so I can return it to the poor chap's family. Take it off and hand it to me, please. I'd like my tinderbox and oil can too.
  - next: ; once: `s043`, `s041a`
- `s041a` chatnpc 2/31 (first: 20101223 `7_neJ6mCLXc` @365s): **Sir Vant**: Thanks. Keep 'em coming, please.
- `s041b` chatnpc 3/31 (first: 20110117 `VwpdbjfigD4` @560s): **Roddeck**: I think you should be returning that item to Sir Vant, not to me.
  - next: `s042` x2; once: `s043`
- `s042` chatnpc 3/31 (first: 20110128 `rX5vda5px3w` @227s): **Sir Vant**: Please hand over my squire's kit.
- `s043` chatnpc 22/31 (first: 20101212 `xqWnfqno_1M` @218s): **Sir Vant**: Right, I think that's everything. Now we just need a new way out of the cellar.
  - next: `s044` x21; once: `s046`
- `s044` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @368s): **Sir Vant**: When the ceiling collapsed, it damaged that wall. It looks like there are two places where you could break through. One's blocked by rocks, the other by tree roots.
- `s045` chatnpc 24/31 (first: 20101127 `IwS6rBpkcyE` @369s): **Sir Vant**: I'm sure Roddeck can teach you how to clear the rocks or the tree roots out of the way.
  - next: `s046` x20; `s047` x2; once: `s048`
- `s046` menu 22/31 (first: 20101212 `xqWnfqno_1M` @223s): What would you like to say? Okay, I'll get on with it. Why don't you do it yourself?
  - next: `s047` x15; `s048` x6; once: `s055`, `s052`
- `s047` chatplayer 21/31 (first: 20101212 `xqWnfqno_1M` @224s): **PLAYER**: Okay, I'll get on with it.
  - next: `s052` x18; once: `s049`, `s046`, `s033`
- `s048` chatplayer 7/31 (first: 20101127 `IwS6rBpkcyE` @370s): **PLAYER**: Why don't you do it yourself?
  - next: `s048a` x4; `s052` x2; once: `s046`, `s026c`
- `s048a` chatnpc 4/31 (first: 20110115 `E4MP65bl3sI` @583s): **Sir Vant**: Well, I would, but my armour is terribly hard to clean when it gets dirty.
  - next: `s047` x3; once: `s052`
- `s049` objbox 1/31 (first: 20110430 `4KO2h2xPtdM` @480s): You find a couple of books and take them.
- `s050` objbox 1/31 (first: 20110430 `4KO2h2xPtdM` @514s): It's full of cabbages. You take one.
- `s051` objbox 3/31 (first: 20101223 `H-yIvqLsEhg` @620s): You eat the cabbage.
  - next: ; once: `s064`, `s052`, `s033`
- `s052` chatnpc 23/31 (first: 20101127 `IwS6rBpkcyE` @375s): **Roddeck**: We need you to break through one of the damaged wall sections by clearing some rocks or tree roots.
  - next: `s053` x21; once: `s061a`, `s054`
- `s053` chatnpc 21/31 (first: 20101127 `IwS6rBpkcyE` @376s): **Roddeck**: The choice is yours: would you like to mine through the rocks or chop through the roots?
  - next: `s054` x20; once: `s055`
- `s054` menu 21/31 (first: 20101212 `xqWnfqno_1M` @232s): What would you like to say? I'll mine through the rocks. I'll chop through the roots.
  - next: `s056` x11; `s055` x10; once: `s059`
- `s055` chatplayer 13/31 (first: 20101127 `IwS6rBpkcyE` @390s): **PLAYER**: I'll mine through the rocks.
  - next: `s057` x9; `s059` x2; once: `s051`, `s004`
- `s056` chatplayer 11/31 (first: 20101212 `xqWnfqno_1M` @235s): **PLAYER**: I'll chop through the roots.
  - next: `s058` x10; once: `s060`
- `s057` objbox 9/31 (first: 20101127 `IwS6rBpkcyE` @393s): Roddeck gives you a pickaxe.
  - next: `s059` x8; once: `s061`
- `s058` objbox 10/31 (first: 20101212 `xqWnfqno_1M` @236s): Roddeck gives you a hatchet.
- `s058a` chatnpc 1/31 (first: 20101215 `kjgw48iBdc0` @639s): **Roddeck**: Now you've got a hatchet, chop your way through the roots inside the damaged wall.
- `s059` mesbox 13/31 (first: 20101127 `IwS6rBpkcyE` @450s): You attempt to mine through the rocks.
  - next: `s061` x10; `s072` x9; `s074` x3; once: `s077`, `s054`
- `s060` mesbox 11/31 (first: 20101212 `xqWnfqno_1M` @243s): You attempt to chop through the roots.
  - next: `s062` x12; `s073` x9; once: `s083`, `s075a`, `s064`, `s062a`
- `s061` mesbox 11/31 (first: 20101127 `IwS6rBpkcyE` @403s): You'll need a Mining level of 2 to clear the rocks. Your Mining level is 1.
  - next: `s063` x6; `s061a` x3; `s061b` x2
- `s061a` chatnpc 4/31 (first: 20110123 `fE0GdVcmJ9E` @550s): **Roddeck**: So, your aim is to get a Mining level of 2? Okay, I'll find you something to train on.
- `s061b` chatnpc 6/31 (first: 20101127 `IwS6rBpkcyE` @422s): **Roddeck**: Ah, I have something! You can practice mining on the rocks that fell into my cellar when the roof collapsed.
- `s062` mesbox 9/31 (first: 20101212 `xqWnfqno_1M` @248s): You find that you'll need a Woodcutting level of 2 to clear the roots. Your Woodcutting level is 1.
  - next: `s064` x5; `s062a` x4; `s060` x3
- `s062a` chatnpc 5/31 (first: 20101212 `xqWnfqno_1M` @259s): **Roddeck**: So, your aim is to get a Woodcutting level of 2? Okay, I'll find you something to train on.
  - next: `s062b` x4; once: `s065`
- `s062b` chatnpc 4/31 (first: 20101212 `xqWnfqno_1M` @261s): **Roddeck**: Ah, I have something! You can practice chopping those roots growing through the wall.
- `s063` mesbox 12/31 (first: 20101127 `IwS6rBpkcyE` @426s): You swing your pick at the rock.
  - next: `s068` x10; `s065m` x8; `s065a` x7; `s066` x3; once: `s059`
- `s064` mesbox 12/31 (first: 20101212 `xqWnfqno_1M` @268s): You swing your hatchet at the root.
  - next: `s069` x10; `s065` x7; `s065a` x5; `s067` x2; once: `s078`, `s068`
- `s065` mesbox 8/31 (first: 20101212 `xqWnfqno_1M` @272s): You break off some bits of wood, gaining 42 Woodcutting experience. Once more should suffice.
  - next: `s067` x7; once: `s064`
- `s065m` mesbox 8/31 (first: 20110123 `fE0GdVcmJ9E` @559s): You break off some bits of rock, gaining 42 Mining experience. Once more should suffice.
- `s065a` mesbox 7/31 (first: 20101212 `xqWnfqno_1M` @281s): You break off some more bits of rock.
  - next: `s064` x5; `s063` x3; `s059` x3; once: `s075b`
- `s066` chatnpc 11/31 (first: 20110115 `E4MP65bl3sI` @623s): **Roddeck**: You've mined a few stone chippings from that rock. The chippings are now in your inventory - they're not particularly useful, but you'll get more useful goods from mining once you're out of this cellar.
  - next: `s063` x10; once: `s070`
- `s067` chatnpc 9/31 (first: 20101223 `7_neJ6mCLXc` @427s): **Roddeck**: You've chopped off a few wood chippings from that root. The chippings are now in your inventory - they're not particularly useful, but you'll get more useful goods from cutting wood once you're out of this cellar.
- `s068` mesbox 11/31 (first: 20110115 `E4MP65bl3sI` @634s): Congratulations: You have reached level 2 Mining.
  - next: `s070` x4; `s077` x2; `s063` x2; `s059` x2; once: `s071`
- `s069` mesbox 10/31 (first: 20101212 `xqWnfqno_1M` @300s): Congratulations: You have reached level 2 Woodcutting.
  - next: `s071` x8; `s077` x2
- `s070` chatnpc 5/31 (first: 20110115 `E4MP65bl3sI` @636s): **Roddeck**: Congratulations, you've reached level 2. You should now be able to mine through the rocks in the damaged wall.
  - next: `s077` x2; `s063` x2; once: `s059`
- `s071` chatnpc 9/31 (first: 20101212 `xqWnfqno_1M` @302s): **Roddeck**: Congratulations, you've reached level 2. You should now be able to chop through the tree roots in the damaged wall.
  - next: `s060` x6; `s077` x3; once: `s075`
- `s072` mesbox 9/31 (first: 20110123 `fE0GdVcmJ9E` @638s): You succeed in clearing the rocks from the hole.
  - next: `s074` x5; `s075a` x4
- `s073` mesbox 9/31 (first: 20101212 `xqWnfqno_1M` @317s): You succeed in clearing the roots from the hole.
  - next: `s075` x5; `s075a` x3; once: `s071`
- `s074` objbox 12/31 (first: 20101127 `IwS6rBpkcyE` @454s): Roddeck retrieves his pickaxe from you.
  - next: `s078` x5; `s077` x4; once: `s082`, `s078b`, `s076a`
- `s075` objbox 10/31 (first: 20101212 `xqWnfqno_1M` @320s): Roddeck retrieves his hatchet from you.
  - next: `s078` x5; `s077` x2; `s076` x2; once: `s078b`
- `s075a` chatnpc 8/31 (first: 20101215 `kjgw48iBdc0` @652s): **Sir Vant**: Oh, well done! Climb through the hole, and we can all get out of this cellar.
  - next: `s074` x4; `s075` x3; once: `s077`
- `s075b` chatnpc 1/31 (first: 20110123 `fE0GdVcmJ9E` @622s): **Sir Vant**: I think we've all been in this cellar long enough. Can we get on with the quest now, please?
- `s076` chatnpc 3/31 (first: 20101223 `7_neJ6mCLXc` @478s): **Roddeck**: You're still carrying a few wooden chippings. It's best to drop them on the floor; they'll just take up space in your inventory.
- `s076a` chatnpc 1/31 (first: 20110204 `WatjF4PlpfM` @364s): **Roddeck**: You're still carrying a few stone chippings. It's best to drop them on the floor; they'll just take up space in your inventory.
- `s077` hint 23/31 (first: 20101212 `xqWnfqno_1M` @326s): When you drop items on the ground, they will eventually vanish. It's a good way to get rid of junk, but don't drop anything you want to keep.
  - next: `s078` x9; `s059` x5; `s060` x4; once: `s086`, `s079`, `s075`, `s058a`
- `s078` chatnpc 20/31 (first: 20101212 `xqWnfqno_1M` @327s): **Sir Vant**: Thank you again for your help with the dragon. You'll receive a little reward for completing the quest when you get outside.
  - next: `s079` x6; `s082` x5; `s081` x3; once: `s087`
- `s078a` chatnpc 0/31: **Roddeck**: I think this ladder leads to the house of my neighbour, Explorer Jack. He's an understanding chap; he won't mind us walking through his house.
- `s078b` chatnpc 2/31 (first: 20101223 `7_neJ6mCLXc` @472s): **Sir Vant**: We've nearly finished the quest now. Once you climb that ladder, you'll be free to start exploring the world outside this cellar.
  - next: ; once: `s077`, `s076`
- `s079` chatnpc 7/31 (first: 20101223 `7_neJ6mCLXc` @503s): **Explorer Jack**: Good grief! Where did you three come from?
- `s080` chatnpc 7/31 (first: 20101223 `7_neJ6mCLXc` @505s): **Roddeck**: I'll explain later, Jack.
- `s081` chatnpc 3/31 (first: 20110510 `Qh_-AMJpVd0` @431s): **Roddeck**: At last, we're out! Now, Sir Vant has accomplished his mission - to rescue me from my pet dragon - so I suppose he'll be off.
- `s082` chatnpc 15/31 (first: 20101127 `IwS6rBpkcyE` @488s): **Sir Vant**: Well, my mission is accomplished now: Roddeck's dragon is no longer a threat.
  - next: `s083` x14; once: `s087`
- `s083` chatnpc 15/31 (first: 20101212 `xqWnfqno_1M` @344s): **Sir Vant**: I'd better head back to Falador. My squire's family will need to be informed of his death. It's been a pleasure working with you.
- `s084` chatnpc 15/31 (first: 20101212 `xqWnfqno_1M` @346s): **Roddeck**: Goodbye, Sir Vant, and thank you again!
  - next: `s085` x14; once: `s087`
- `s085` chatnpc 14/31 (first: 20101223 `7_neJ6mCLXc` @511s): **Explorer Jack**: Once you're out and about, you can use the Task System to suggest aims and endeavours to go for. Every one you do will get you a reward.
- `s086` chatnpc 15/31 (first: 20101223 `7_neJ6mCLXc` @517s): **Explorer Jack**: You'll always earn some money, sometimes additional goodies, and often progress towards a piece of armour with special effects. Look at the Rewards tab for a Task to learn more.
  - next: `s087` x10; `s087a` x5
- `s087` mesbox 17/31 (first: 20101127 `IwS6rBpkcyE` @500s): As you leave, Roddeck wanders off home.
- `s087a` chatnpc 5/31 (first: 20101223 `RUM_me2Dl9A` @392s): **Roddeck**: Thank you once again for your assistance.
- `g001` game 3/31 (first: 20110123 `fE0GdVcmJ9E` @711s): You can't do this while in the tutorial.
- `g002` game 4/31 (first: 20101224 `sJou6QLfVes` @412s): Please finish what you're doing before opening the world map.
- `g003` game 3/31 (first: 20110409 `KYxDa0aq2q0` @492s): You have completed the Task 'How To Train and Level'!
- `g004` game 16/31 (first: 20101212 `xqWnfqno_1M` @372s): Congratulations! Quest complete!
- `g005` game 13/31 (first: 20101212 `xqWnfqno_1M` @336s): You have unlocked a new music track: Harmony.
- `g006` game 8/31 (first: 20101127 `IwS6rBpkcyE` @483s): You've just advanced 2 Mining levels! You have reached level 4.
- `g007` game 6/31 (first: 20101212 `xqWnfqno_1M` @338s): You've just advanced 2 Woodcutting levels! You have reached level 4.
- `g008` game 2/31 (first: 20110430 `4KO2h2xPtdM` @677s): You have completed the Task 'On the Run'!
- `g009` game 2/31 (first: 20101212 `xqWnfqno_1M` @385s): You have now completed 2 Tasks in total.
- `g010` game 4/31 (first: 20101215 `kjgw48iBdc0` @82s): Please close the interface you have open before setting your audio options.

Notes:
- **Builds.** Four recordings are an older build: IwS6rBpkcyE (2010-11-27 upload), xqWnfqno_1M
  (2010-12-12), kjgw48iBdc0 (2010-12-15) and H-yIvqLsEhg (2010-12-23).
  - That build has step guides in the chatbox, and Roddeck adds an advisor button: "I'll add a
    button to your display. Use it when you want my advice."
  - Every other recording is the Task System build, which cache 786 holds (tasks 3500-3523).
  - The dialogue lines are shared: no line is seen only in the old build. 55 are seen only in
    the Task System build, including the whole exit talk with Explorer Jack.
- **Read by eye where OCR missed:**
  - `s030` (Qh_-AMJpVd0 @310);
  - `s031a` and `s078a` (promi6pWj8k @627 and @978);
  - `s058a` in 7_neJ6mCLXc @395.
  - `s026f` is an option in `s026b`'s menu, not seen as a chosen line.
  - `s001a`'s last full stop is unreadable.
- **Wiki conflict on eating the meat.** Drugged meat @ 2011-04-23 gives "Are you mad? Don't eat that!
  It smells vile, it's completely raw, and I'm sure I mentioned that it's been treated with
  sleeping potion!" for eat or drop. The recordings disagree:
  - eating gives `s018` (6 sightings, 2011-01-15 to 2011-05-21);
  - dropping gives `s013a`;
  - fE0GdVcmJ9E shows "Eat Drugged meat" and "Drop Drugged meat" hovered just before each.
  The recordings win: for dialogue text, dated recordings are the authority. The wiki often
  paraphrases, or keeps an older version of a line.
- **Gone by late August 2011.** Bfg3ivxRmZ4 (2011-08-22) and KsJlgeXVtiE (2011-10-15) go from
  character creation straight to Explorer Jack's house, with no cellar.
- **Outputs kept locally, with the downloaded videos:** the list of excluded videos with
  reasons, and the task-panel and option streams. All 24 tutorial tasks are seen in the panel
  stream. None of these are in git.
- **Rerunning:** the `recordings-matrix` skill's `matrix.sh` and `frames.sh` rebuild them from
  the video list.

---

## 4. Cutscenes and camera (cache 786: client-side camera paths)

647 runs UF's cutscene camera in the client. Interface 899 (one layer,
`899:0`: `onload=tutorial3_cutscenes(event_com)`,
`onvarctransmit=tutorial3_cutscenes(event_com){varclient_1007}`) runs
cs2 2765 whenever varc 1007 changes:

| varc 1007 | cs2 |
|---|---|
| <= 0 | cam_clear (op 5503/5507-5510/5512: the decompiler names all of these `cam_clear`; 5503 calls the CAM_RESET routine, 5512 the CAM_SMOOTHRESET routine, `client-src Class47.java:3884,3940`) |
| 1-100 | 2770 |
| 101-200 | 2762 |
| 201-300 | 2763 |
| > 300 | 2826 |

Op semantics (client `Class47.java:3647-3668, 3854-3882`, `Class22.method203`):
`cam_allocpath(path, n)`; `cam_addwaypoint(path, i, coordA, heightA, coordB,
heightB, roll)`: a Bezier point (A) and handle (B); `cam_startpath(posPath,
posStart, speedFrom, speedTo, lookPath, lookStart)`: the camera rides path 0
from waypoint posStart to posStart+1 while looking at path 1, speed easing
speedFrom -> speedTo (units of 1/65536 of the segment per client cycle).

Coordinates go through cs2 2764: `movecoord(base, 16 + x - 3648, 0, 16 + z -
4928)` with base = the player's current mapsquare origin (cs2 284). So the cs2
waypoints are tiles of square 57_77, **shifted 16 tiles in**: the client expects
the cellar copied into a dynamic map with its (0,0) at (16,16) of the player's
mapsquare (inference, but it follows from the arithmetic: in the static 57_77
every camera point lands 16 tiles off).

Waypoints, as 57_77-local (lx,lz), height in the cs2's units:

| varc | camera path (0) | looks at (1) | startpath | nearest cache locs |
|---|---|---|---|---|
| 1 | (32,13) h600 -> (32,13) h450 | (32,20) -> (32,25) h100 | 0,0,100,140,1,0 | start room (bag 35,13; ladder 32,12) |
| 2 | 11 points (25,37) -> (32,27), diagonal then up x=32, h400-420, roll 0/±128 alternating | (32,26) -> (32,17), h400 -> 350 | 0,0,3000,2500,1,0; synth 6645; cs2 2772 steps on with synth 6643/6644/6645 at random | from the pillar/bowl area toward the start room door |
| 3 | (32,22) h350 -> 300 | (32,30) | 0,0,60,50,1,0 | |
| 4 | (36,18) h450 -> (34,12) h600 | (32,17) -> (32,19) | 0,0,250,200,1,0 | |
| 5 | (36,17) h700 | (32,14) h350 -> (32,13) h150 | 0,0,100,100,1,0 | |
| 6 | (35,13) -> (33,14) h450 | (32,17) -> (32,19) | 0,0,60,60,1,0 | bag (35,13) |
| 7 | (33,14) -> (33,12) h400 | (29,13) h350 | 0,0,30,30,1,0 | |
| 8 | (36,21) -> (35,19) h450 | (34,20) -> (32,21) | 0,0,150,150,1,0 | |
| 9 | (36,17) h500 -> (29,16) h700 | (34,12) -> (35,12) h250 | 0,0,30,30,1,0 | walls (28..36,16) |
| 10 | (36,9) h800 -> (36,7) h600 | (32,16) -> (30,18) | 0,0,400,80,1,0 | |
| 101 | (35,13) -> (36,13) h450-600 | (29,19) -> (32,17) | 0,0,300,400,1,0 | |
| 102 | (29,25) -> (30,30) h600-700 | (30,15) -> (30,18) | 0,0,250,200,1,0 | |
| 103 | (31,21) -> (32,26) h650-700 | (32,35) -> (32,40) h5 | 0,0,200,200,1,0 | |
| 104 | (21,37) -> (23,34) h300 | (32,29) h250 | 0,0,100,50,1,0 | near pillar 1 (24,31) |
| 105 | (32,31) -> (27,35) h700 | (18,35) h5 | 0,0,80,50,1,0 | food bowl (17,35) |
| 106, 107 | (31,30) -> (27,37) h350-550 | (27,30) -> (20,30) | 0,0,120,40,1,0; fade (cs2 2768, 25) | |
| 201 | (33,35) -> (36,35) h550-600 | (18,35) h200 | 0,0,200,75,1,0 | food bowl |
| 202, 203 | (36,35) h550 -> (35,35) h750 | (18,35) h200 | 0,0,50,25,1,0; fade (cs2 2768, 75) | food bowl |
| 204 | (31,38) -> (34,39) h550 | (37,30) -> (25,34) h400 | 0,0,150,220,1,0 | cupboard door (37,30) -> pillars |
| 301 | 4 points: (29,29), (34,42), (31,35), (31,31) | 4 points: (29,36), (43,43), (31,53), (19,36) -> (2,36) | 0,0,500,300,1,0 | |
| 302 | segment 1 | segment 1 | 0,1,600,300,1,1 | |
| 303 | segment 2 | segment 2 | 0,2,600,300,1,2 | |

Which story beat each varc value belongs to, and when the server sets it:
**no source found** (the nearest-loc column suggests 105/201-203 = the dragon at
the bowl and 204 = Roddeck's cupboard, but that is inference). Which component
interface 899 opens into: **no source found**.

Server-side camera packets (CAM_LOOKAT etc.) for UF: no source shows any.
`cam_shake` for the collapse: **no source found**.

Effects that are cache facts but unplaced in time: pillar states 45569/45570
(anims 12242/12243); blockage anims 12279 (rock) and 12281 (roots); ladder
45786 (anim 12275, darkened: inference: the burnt start-room ladder, struct 1718
"The dragon's breath burned away the ladder"); spotanims with UF-range models
(inference by model id range 49336-49343): 2148 (model 49336, seq 12271), 2149
(49342, seq 12245), 2150 (49343, seq 12244), 2151 (49339, seq 12236, carries the
squire 8857's recolour pairs reversed), 2271 (49342, seq 12245). Which is the
dragonfire: **no source found**.

---

## 5. Other cache 786 evidence

- **Interfaces**: 897/1079 (controls overlay), 899 (cutscene camera driver), 1056
  `task_side` (Task System tab; already opened at `login.rs2:120`), 1080 (Task
  pin text), 761 (Learning the Ropes' "Sir Vant's Requests", not UF), 1028
  `playerdesign4` (character design; ported). Names 371 `tutorial_progress`, 372
  `tutorial_text`, 421 `tutorial_text2`, 893 `advisor_tutorial_side`, 894
  `advisor_tutorial_chat` exist in `interface.pack`; whether UF uses them: **no
  source found**.
- **Clientscripts**: 2193, 2157/2158 (quest list), 3224/3227/4098/3977 (Task
  System), 2765/2770/2762/2763/2826/2764/2772/2768 (cutscene camera), 2773-2777
  (controls overlay), 973/175/192/943/2168 (tutorial gating).
- **Seqs**: BAS stand/walk anims 12227-12280 (table 2.1); loc anims 12242,
  12243, 12275, 12279, 12281, 12298 (locs 45630-45632), 12396 (Planks).
  Animation names: **no source found**.
- **Voice-over (vorbis)**: 5120 vorbis groups in the cache
  (`content/647/vorbis`), unnamed. Which are UF's: **no source
  found**. Engine: `rs-protocol/src/network/game/server/vorbis_sound.rs` exists
  (fields `unk1`, `unk2`), nothing sends it, no wire test.
- **Music** (wiki locations; 647 ids from `content/647/pack/midi.pack` and
  `content/647/scripts/runescript/music/configs/music.dbrow`):

| Track | When (wiki) | midi | music list (playbutton) |
|---|---|---|---|
| Destiny | character creation (Destiny @ 2011-05-07; UF Music) | 859 | 931 |
| The Adventurer | "while helping Sir Vant defeat the dragon" (UF Music); "first track a player gets" (The Adventurer) | 401 | 520 |
| The Mentor | Tutorial Cellar; "after defeating the dragon" (UF Music) | 552 | 611 |
| Harmony | after escaping (Lumbridge Castle) | 76 | 58 |

  Tutorial Cellar's infobox gives music "Scape Main" (midi 0): it conflicts with
  the UF page. No `musicregion` row exists for 57_77. Switch points: **no source
  found** beyond the wiki's wording.

---

## 6. void-634

What it has (all void-634's): item ids/examines matching the cache objs
(`unstable_foundations.items.toml`); book texts (`unstable_foundations.books.toml`);
varp 281 = `unstable_foundations` 0/1/1000 (`.varps.toml`); Task structs
1703-1724 named in `data/client/structs.toml:37-58`; Task visibility per varp
281 (`game/.../achievement/Tasks.kt:138-160`, mirrors cs2 3224 plus varbit 6495
tests); `TaskSystem.kt:28-33` marks UF "incomplete" and pins Task 3520 when the
Task tab opens (a void shortcut, not UF); Explorer Jack completes UF
(`ExplorerJack.kt:100-105`); `vants_requests` = interface 761. It has **no**
cellar, no Sir Vant/dragon/goblin/squire logic, no dialogue for them, no camera.
Its UF npc spawn (7942 at (3294,4937)) and teles (`.teles.toml`, region 14669 /
13133 comments with tiles outside those regions) do not match the cache.

---

## 7. Rewards

| Reward | Source |
|---|---|
| 1 Quest Point | wiki UF Reward; quest config 131 `op9_g1=1` (inference: QP field) |
| "Level 4 Mining/Woodcutting (depends on which way you escaped the cellar)" | wiki UF Reward. Conflicts: cs2 973 tells a tutorial player at stat base 3 it is "the highest skill level that you can achieve during the tutorial"; the cache Tasks need level 2 (structs 1721/1722). Unresolved. |
| 42 xp per practice root/rock, 83 xp for level 2 | structs 1721/1722; wiki UF. Stone clippings page says 21 Mining xp: conflicts. |
| "The ability to chat freely if you are age 13 and up", "The ability to explore RuneScape", "The services of Roddeck, an advisor" | wiki UF Reward |
| "20 coins (From the tutorial tasks)" | wiki UF Reward; amount per Task and who pays: **no source found** (void-634's Explorer Jack pays Task rewards: void-634's) |
| The Adventurer unlocked | wiki The Adventurer |
| Sir Vant in Falador (varbit 4895 = 3) | cache npc 8854 + spawn `n46_52.jm2:22`; wiki Sir Vant |
| Stone clippings / root chippings must be dropped | wiki Stone clippings, Root chippings; struct 1721/1722 hints |

---

## 8. Camera, cutscene and engine support on 647

### 8.1 What exists

| Server prot (647 opcode) | Encoder | Script command / engine path | Wire test | Client handler |
|---|---|---|---|---|
| CAM_LOOKAT (26) | `cam_lookat.rs` | `cam_lookat(coord,height,rate,rate2)`: queued, sent build-area-local each tick (`active_player.rs:671-688, 724-743`) | `wire_647.rs:482` | `MapAreaConfig.java:488` |
| CAM_MOVETO (94) | `cam_moveto.rs` | `cam_moveto(...)` (same path, `active_player.rs:652-669`) | `wire_647.rs:503` | `:1363` |
| CAM_SHAKE (96) | `cam_shake.rs` | `cam_shake(axis,random,amplitude,rate)`, cycle always 0 (`active_player.rs:690-708`) | `wire_647.rs:525` | `:1519` |
| CAM_RESET (67) | `cam_reset.rs` | `cam_reset` (`active_player.rs:837`) | none (empty payload) | `:339` |
| CAM_SMOOTHRESET (12) | `cam_smoothreset.rs` | **none** | opcode only (`wire_647.rs:551`) | `:1373` |
| CAM_FORCEANGLE (29) | `cam_forceangle.rs` (`p2_alt1 yaw; p2_alt1 pitch`) | **none** | **none**; field names unverified against `TextureOp_Sub26.method3767`'s parameters | `:1016` |
| CAM_REMOVEROOF (36) | `cam_removeroof.rs` (`p4_alt2 coord`, -1 clears) | **none** | **none** | `:236` |
| HINT_ARROW (45) | `hint_arrow.rs` | `hint_coord`, `hint_npc` (`active_player.rs:913-925`) | not checked here | `:1219` |
| VORBIS_SOUND (51) | `vorbis_sound.rs` | **none** | **none** | not checked |
| REBUILD_REGION (77) | `rebuild_region.rs` | **none**: no engine path, no dynamic-map/instance support anywhere in `rs-engine` | **none** | |
| TUT_OPEN / TUT_FLASH | not in 647's prot list | `tut_open`/`tut_flash` write nothing on 647 (`active_player.rs:2916-2969` TODOs) | | |

### 8.2 What UF needs that is missing

1. **Nothing new for the cutscene camera itself**, if the cache's client-side
   paths are used: the server opens interface 899 and sets varc 1007
   (`sendvarc` exists, used in `cc_setup.rs2`). CAM_SMOOTHRESET/RESET equivalents
   are inside the cs2 (ops 5503/5512). `cam_reset` exists for the end.
   Gaps: which slot 899 opens into, and when each varc value is sent (**no
   source found**).
2. **Dynamic map (instance)**: cs2 2764's +16 offset means the camera points are
   only right when the cellar is built into a dynamic map, the cellar's (0,0) at
   tile (16,16) of the player's mapsquare. That needs REBUILD_REGION wired, an
   instance allocator, collision and loc/npc spawning in the copied zones. This
   shares the build-area/map-rebuild path with `rebuild_normal`, which is on
   Jordan's hands-off list: **stop and report; not implemented here.**
3. **Per-player npcs**: Sir Vant, the squire, the goblin, the dragon and Roddeck
   act per player. Without instances, every player in 57_77 would see every
   other player's dragon. The engine has no per-player npc visibility (grep: no
   owner/visible-to field in `rs-engine/src`, `rs-entity`); adding it means npc
   info (`rs-engine/src/info.rs`, `rs-engine/rs-info/**`): **hands-off; stop
   and report.** Instancing (item 2) would make it unnecessary.
4. **Script commands** for CAM_FORCEANGLE, CAM_SMOOTHRESET, CAM_REMOVEROOF:
   missing, and outside the hands-off list (rs-vm `ops/player.rs`, `engine.rs`,
   `active_player.rs`, `content/647/scripts/runescript/engine.rs2`). No UF source asks for
   them (the cs2 paths do the work), so only add them if a step needs them; each
   needs a wire test against `MapAreaConfig` first.
5. **Voice-over**: VORBIS_SOUND has no command and no mapping (section 5): out
   of scope until a source maps lines to ids.

---

## 9. Proposed build plan (one commit each, each tested)

Every stage stops where its row says **gap**.

| # | Stage | Test | Gaps |
|---|---|---|---|
| 0 | Engine decision on instancing (8.2 items 2-3): **done**, Jordan's regions and controllers (337f6e28e0: `region_create`, `region_copyarea`, `region_getcoord`, `region_del`, `controller_*`) | `_test/scripts/engine/region_tests.rs2` | |
| 1 | varp 281 as UF's: character creation ends at UF's first stage, not 1001; UF's end sets 1000; varbit 4895 = 3 at creation (inference) | testscript: after `character_created`, varp281 = 5/6 (value 1-6 meaning: **gap**); at UF end = 1000; quest list cs2 2193 reads done | values 1-6, the exact first value |
| 2 | UF configs: npc/loc/obj names for the ids in section 2; multiloc state helpers (varbits 6487, 6495, 6496, 6497) | runec lint; CRC gate if packs change | |
| 3 | Start: place the player in the cellar (instance or static), open 1079/897 controls, Task tab; arrow-key ops -> varp281 7 | harness: login of a new account lands in the cellar, IF_BUTTON on 1079:0 sets 7 | start tile, overlay slot, whether ops are transmitted: **gap** |
| 4 | Squire's gear and goblin: pick up gear (15), equip (20-30), goblin fight rules (35-40, wiki Goblin page) | harness: equip -> 35; goblin only attackable in full gear | Sir Vant's lines: **gap**; gear obj ids: **gap** |
| 5 | Quest journal, world map, minimap Tasks (50-62) | testscript per threshold | how the server detects world map open / compass click: **gap** |
| 6 | Plank, ramp (loc 45604 >= 75), low wall climb (65-75) | harness: use plank on wall -> 70, climb -> 75 | plank loc: **gap** |
| 7 | Food bowl (varbit 6487), dragon asleep (80-90) + cutscene varcs 105/201-203 | harness: use meat on bowl -> 80+ | varc timing: **gap** |
| 8 | Sabotage: rip/oil/fuse/pillars (varbits 6496/6497), tinderbox to Sir Vant, light, collapse (93-115) | harness per item-on-loc | pillar state order (0-4 meaning): inference only |
| 9 | Return gear (120-127) | harness | |
| 10 | Roddeck and the exit choice (varbit 6495), practice rocks/roots 42 xp, blockage, hole (128-170) | harness: chop/mine to level 2, clear, climb through | Roddeck's lines: **gap** |
| 11 | Ladder 45784 -> Lumbridge, varp281 = 1000, Explorer Jack Task (varbit 6494 = 3) | harness: climb -> Lumbridge, 1000 | arrival tile, Jack's lines: **gap** (void-634's only) |
| 12 | Music (Destiny, The Adventurer, The Mentor, Harmony) | testscript: midi per stage | exact switch points: wiki wording only |

**Dialogue (all stages):** section 3.4 gives every line, with its first sighting and the
branches, from 32 dated 2010-11 recordings. Recordings outrank the wiki for dialogue text (3.4's
notes). A line no recording shows stays out, or keeps a `// no source found` stub.
