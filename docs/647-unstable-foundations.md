# Unstable Foundations (2011's tutorial) on 647: design note

Research for building Unstable Foundations (UF) as 647 content, end to end. Every
claim names its source. Kinds of claim are kept apart:

- **cache**: cache 786 (configs in `content/647/scripts/_unpack/all.*`, maps in
  `content/647/maps/*.jm2`, interfaces in
  `content/647/interfaces/*.if` (named components), decompiled
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

- `content/647/scripts/login_logout/scripts/login.rs2:40`: `if
  (%learning_the_ropes < 1000) { ~cc_open_interface(); return; }`.
- `cc_setup.rs2` `[proc,cc_open_interface]`: opens `playerdesign4` (interface
  1028) top-level, `allowdesign(^true)`.
- `cc_name.rs2:114-121` `[queue,character_created]`: `%learning_the_ropes =
  1001`, applies the design, opens the game frame. **No teleport**: the player
  stays where the account was created.
- `%learning_the_ropes` is varp 281 (`content/647/pack/varp.pack:282`, perm in
  `content/647/scripts/player/configs/perm.varp:3`, `transmit=yes` in
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
`1080.if:552`).

Hint buttons (`param_1282..1289`, type coord): every UF value decodes to
x=6336, z=6337..6383 (e.g. 103815399 -> (6336,6375)); 6337 and 6338 are given
to "click on the plank in your inventory" and "open the worn equipment tab", so
they are keys, not tiles (inference). What the server does for each key: **no
source found**.

The first-step "Controls" overlay: interfaces 897 and 1079 (identical; texts
"Controls", "Arrow keys rotate the camera.", "Click on the ground or minimap to
walk.", "CTRL + click to run.", "Please rotate the camera to begin.";
`1079.if:588`). Component 0 binds the four arrow keys (`opkey=0..3,1,96..99`)
to ops 1-4 with `active_properties=30`; onop cs2 2773 lights the pressed key
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

**Every other line of UF dialogue (Sir Vant, Squire, Goblin, Dragon, Roddeck
in the cellar, the player, the narrator): no source found, 2011 or void-634.** The dialogue was voiced
(Update:Slayer Challenges, 21 April 2010: "Voices have been implemented for all
NPCs (and the player) within the tutorial"), so cache 786's vorbis archive may
hold the spoken lines; no cache table maps lines to vorbis ids (no enum or
param of a vorbis type; void-634 has none either): **no source found** for
which vorbis ids are UF's. The build must stop at the dialogue until a source
is chosen.

The quest journal's text (server-sent in 647): **no source found**.

---

## 4. Cutscenes and camera (cache 786: client-side camera paths)

647 runs UF's cutscene camera in the client. Interface 899 (one layer,
`899.if:7-9`: `onload=i2765`, `hook18=i2765`, `varctransmit_list=1007`) runs
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
  `content/647/scripts/music/configs/music.dbrow`):

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
   `active_player.rs`, `content/647/scripts/engine.rs2`). No UF source asks for
   them (the cs2 paths do the work), so only add them if a step needs them; each
   needs a wire test against `MapAreaConfig` first.
5. **Voice-over**: VORBIS_SOUND has no command and no mapping (section 5): out
   of scope until a source maps lines to ids.

---

## 9. Proposed build plan (one commit each, each tested)

Every stage stops where its row says **gap**.

| # | Stage | Test | Gaps |
|---|---|---|---|
| 0 | Engine decision on instancing (8.2 items 2-3): integrator/Jordan | | blocks stages 3+ in their 2011 form |
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

**Dialogue (all stages):** the seven 2011-quoted lines in 3.3, and for stage 11
void-634's Explorer Jack lines (3.3), commented as void-634's. Nothing else:
no 2011 source and no void-634 line exists for the cellar (Sir Vant, Squire,
Goblin, Dragon, Roddeck), and 289 has no UF. Per the brief, no stage writes a
line no source shows. Unless the integrator names another source (for
example, transcribing UF's voice-over from the cache's vorbis archive once its
ids are found), stages 4-10 can be built only as mechanics, Task text and
camera, with every Sir Vant/Roddeck/Squire/Goblin conversation left as a
`// no source found` stub.
