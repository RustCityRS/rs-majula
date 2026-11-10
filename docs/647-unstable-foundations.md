# Unstable Foundations (2011's tutorial) on 647: design note

Research for building Unstable Foundations (UF) as 647 content, end to end. Every
claim names its source. Kinds of claim are kept apart:

- **cache**: cache 786 (configs in `content/647/scripts/configs/_unpack/all.*`, maps in
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

- `content/647/scripts/login_logout/scripts/login.rs2:40`: `if
  (%tutorial < 1000) { ~cc_open_interface(); return; }`.
- `cc_setup.rs2` `[proc,cc_open_interface]`: opens `playerdesign4` (interface
  1028) top-level, `allowdesign(^true)`.
- `cc_name.rs2:114-121` `[queue,character_created]`: `%learning_the_ropes =
  1001`, applies the design, opens the game frame. **No teleport**: the player
  stays where the account was created.
- Varp 281, called `%learning_the_ropes` in these earlier notes, is now named
  `%tutorial` (`content/647/pack/varp.pack:283`). Its section in
  `content/647/scripts/configs/_unpack/all.varp:1319` sets `scope=perm` and
  `transmit=yes`.

### 1.2 What cache 786 says varp 281 is

Varp 281 is UF's progress varp (cache):

| Source | What it shows |
|---|---|
| quest config 131 (`all.quest:669-672`) | `op1_str2=Unstable Foundations`, `op3_counted_g2_g4_g4=1,281,5,1000`: varp 281, started at 5, complete at 1000 (same layout as Cook's Assistant `1,29,1,2`, Dragon Slayer `1,176,1,10`). `op9_g1=1` (inference: quest points; it reads 1/2/3 for Cook's Assistant/Dragon Slayer/Desert Treasure, their QP). |
| cs2 2193 (quest list) | `case 147: return(~ql4_getprogress_basic(%varp281, 1000))` (cs2 2158 ql4_getprogress_detail: <1 not started, <1000 started, else done) |
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

Other UF vars (cache, `all.varpbit`):

| var | bits | used by |
|---|---|---|
| varbit 4895 | varp 1255 bits 0-1 | npc 8854 multinpc: value 3 -> `sir_vant_8848`, else hidden (Falador Sir Vant, spawn below); cs2 1886, 1661, 3971 test `= 3`; cs2 1522/1523 test `= 2` for interface 761 "Sir Vant's Requests" (Learning the Ropes, `761.if` text "This quest was originally known as 'Learning the Ropes'."). Inference: 3 marks an account that took UF, 2 a Learning the Ropes account. |
| varbit 6487 | varp 1254 bits 22-23 | loc 45795 (dragon's food bowl): 0 -> loc 1758 (no op), else -> loc 1761 (op1 Search) |
| varbit 6494 | varp 1261 bits 0-2 | cs2 3227: Task 3520 "Introducing...Explorer Jack" done when `> 2`. void-634 names it `introducing_explorer_jack_task`, 3 = completed (`data/achievement/achievement_task.varpbits.toml:87`) |
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
`unstable_foundations`, `achievement_task.varpbits.toml:49`). cs2 4098 picks the
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

### 3.5 Gaps closed from the recordings (2026-11-09)

Video ids are YouTube ids; times are seconds into the video. Local tiles are of square
57_77 (the cellar copy's (16,16) is its (0,0)).

#### 1. Start tile: local (30, 9) of square 57_77 (confirmed)

Method: each 2011 minimap is fitted against our client's minimap at a known tile by rotation,
scale and shift (the recordings-matrix skill's `minimap_match.py`). The whole room lines up: door line, ladder icon and walls.

| Recording | t | Fit score | Tile |
|---|---|---|---|
| 4KO2h2xPtdM (2011-04-30) | 8.5 | 0.82 | (29.8, 9.0) |
| VwpdbjfigD4 (2011-01-17) | 80 | 0.84 | (30.5, 9.2) |
| uHpgIVCC66Y (2011-02-04) | 5 | 0.82 | (29.8, 9.2) |
| i9o_EiHokhQ (2011-01-31) | 18 | 0.82 | (30.0, 12.5): already walking to Sir Vant |

The player stands south of the ladder icon (ladder 45785 at (32,12)), in the south-west part of
the start room. Sir Vant is at the room's north door.

#### 2. The plank: ground obj 15276 (plank_15276) on local (35, 15)

"Take Plank" (a ground item's option) in 7 recordings. The player stands on the plank's tile as it
enters the inventory:
- Kw7h2jY9ANQ @265-266: (35.2, 15.2);
- Qzq-h8kLBHk @135.5-136: (35.2, 15.0).

That is the start room's north-east corner, against the low wall's east section (35, 16), which
the ramp crosses (Sir Vant: "Try going to the eastern end where it's lower.").

#### 3. The squire's gear: bronze platebody 1117, bronze sq shield 1173, bronze sword 1277 on (32, 13)

- **Ids.** Cache 786 has no tutorial copies: one obj each by those names. The right-click menu
  (Qzq-h8kLBHk @98) lists "Take Bronze platebody / sq shield / sword", all three on one tile.
- **Tile.** The player stands on it as the gear enters the inventory:
  - Kw7h2jY9ANQ @187: (32.0, 13.5);
  - Qzq-h8kLBHk @100: (32.2, 13.2).
  That is just north of the burned ladder (32, 12), by the stone pillar where the squire stood.
- **Smoke.** In the cutscene the pile smokes (4KO2h2xPtdM @36-37).

#### 4. Cutscene timelines (4KO2h2xPtdM, 2011-04-30; one frame a second)

**The dragon (s002 shown 14.0, dismissed ~17.0; s003 at 51.9):**

| t | Shot |
|---|---|
| 17-20 | behind the player, at Sir Vant in the doorway |
| 21-24 | low across the dark main cellar; a distant "Help!" overhead at 22-23 (the squire, voice enum 3038) |
| 25 | the dragon's face, close |
| 26-33 | Sir Vant at the wall, the dragon breathing over it at his shield |
| 34-35 | the start room: the squire is hit, "Aaargh!" overhead |
| 36-37 | his gear smoking by the pillar |
| 38-41 | Sir Vant and the dragon at the wall |
| 42-44 | the player recoiling, arms up, by the fireplace |
| 45-46 | the dragon, close |
| 47-51 | the goblin at Sir Vant's bag, "Ooh, shiny!" |
| 52 | back to play, s003 |

Fastest players across the four recordings: 14.8-19.3 s from s002 shown to s003 shown.

**Sleep (s019 "Hide!" at 199.1; s020 at 237.1):**

| t | Shot |
|---|---|
| 197-201 | the player puts the meat in the bowl |
| 202 | the player runs to hide |
| 203-206 | Sir Vant at the wall: "Grub's up! Shoo!" overhead (204-205) |
| 207-211 | the dragon goes over the wall (camera behind it) |
| 212-216 | the dragon crosses the cellar to its nest |
| 217-230 | the dragon at the bowl, eating |
| 231-232 | the dragon lies down asleep on the nest |
| 233-234 | black, "Please wait..." |
| 235 | Sir Vant by the pillars, play resumes; s020 at 237.1 |

Hide! to s020: 18.9-19.8 s in four Task-build recordings, a fixed length.

**Collapse (s037 "It's burning! Run!" at 392.1; s038 at 419.3):**

| t | Shot |
|---|---|
| 390-394 | the player lights the right pillar's fuse; Sir Vant lights his |
| 395-396 | the player runs south |
| 397-405 | fixed shot on the nest: both pillars burning, the dragon asleep |
| 406 | the roof falls |
| 407-410 | black ("Please wait..." at 407); the advisor button appears at ~408 |
| 411-415 | rubble over the nest; the player stands, Sir Vant on the right: "Wow!" overhead (413-414) |
| 416-418 | Roddeck walks out of his cupboard door |
| 419 | s038 |

#### 5. Quest journal (seen at "Check your Quest Journal" only)

Every recording that opens the journal (12) does so at varp 281 ~ 50-57. It opens scrolled to
the bottom, and nobody scrolls up. 4KO2h2xPtdM @124 reads:
- (struck through, the earlier entries above it unseen) "Once his bag of supplies was safe, Sir
  Vant asked me to help him kill the dragon."
- (navy) "Quest journals record what needs to be done next in each quest. If you ever forget what
  you have been asked to do, take a look in the journal to remind yourself. You can also click
  the <red>Get QuestHelp</red> link to receive more detailed hints for how to proceed with the
  quest."
- (navy, red keywords) "Sir Vant asked me to take some <red>drugged meat</red> from his <red>bag
  of supplies</red> and place it in a food bowl near <red>two wooden pillars</red>."
- "Get QuestHelp" link, bottom right.

The old build (xqWnfqno_1M @12) says the same entry; its explanation is in the chatbox guide
instead. Entries before the struck line, and all later entries: no recording shows them.

#### 6. Music: Destiny (creation), The Mentor (cellar), Harmony (Lumbridge)

Method: our client played each track (`::ufsong 1-4`), recorded audio-only from the virtual cable.
Each 10 s window of a recording was matched to the tracks by chroma (the recordings-matrix skill's `music_match.py`); each
reference picks itself at 0.89-0.94 against 0.28-0.43 for the others.

Only Qh_-AMJpVd0 (2011-05-10) has clean game audio. 4KO2h2xPtdM and Kw7h2jY9ANQ carry the
uploaders' own music (Kw7h's scores repeat with a ~318 s period); 8jKH_7To46o is mostly
inconclusive.

| Qh_-AMJpVd0 t | Track | Score |
|---|---|---|
| 40-159 (character creation, Continue at 157.8) | Destiny | 0.53-0.74 |
| 169-397 (the cellar) | The Mentor / The Adventurer (one tune) | 0.30-0.67 |
| 447-468 (Lumbridge, after the exit) | Harmony | 0.40-0.52 |

- **The Mentor or The Adventurer?** Their chroma is the same tune. A 64-band spectrum (timbre)
  favours The Mentor in 21 of 27 cellar windows. The 2011 wiki puts The Mentor in the "Tutorial
  Cellar" and Destiny in character creation (The Mentor @ 2011-05-18, Destiny @ 2011-05-07). So
  the cellar plays The Mentor.
- **The Adventurer** is never heard distinctly. The wiki says every player has it after
  completing the tutorial (The Adventurer @ 2011-05-22), so it is unlocked, not played: when, no
  source shows.
- **Harmony.** 8jKH_7To46o also hears Harmony at its end (616-636). It plays on arriving in
  Lumbridge, square 50_50, where musicregion_50_50 already gives Harmony. "You have unlocked a
  new music track: Harmony." shows at the same moment (4KO2h2xPtdM ~607).
- **Switch points.** Destiny to The Mentor between 159 and 169 (entering the cellar); The Mentor
  to Harmony on arrival (431-447; the 437 window is mixed).

#### 7. Arrival in Lumbridge: (3207, 3240), with the quest scroll

4KO2h2xPtdM's minimap after the ladder (607.5, 610, 612) fits ours at (3207,3240), one tile north
of Explorer Jack's trapdoor (3207,3239), with shift (0,-1) px. The fit scores only 0.54 (npc dots
everywhere), but the house outlines and icons line up by eye. The 2011 camera faces south
(theta 178).

On arrival the quest-complete scroll is open (4KO2h2xPtdM @607.5): "Congratulations! You have
completed Unstable Foundations!", "You gain:", "1 Quest Point", "Level 4 Mining" (the chosen
skill), a pickaxe on the left, "Quest Points: 1". Then the game messages:
- "You have completed the Task 'How To Train and Level'!"
- "Congratulations! Quest complete!"
- "You have unlocked a new music track: Harmony."
- "You've just advanced 2 Mining levels! You have reached level 4."

#### 8. Line breaks: 14 forced <br>, everything else the client's own wrap

Each line's natural wrap is computed from q8_full's advances: 380 px for chat and item boxes, 470
for message boxes. The item box's width is bounded to 380-394 by the bag's two lines, so 380.
Every line was compared with its recorded break points: high-resolution sightings for 67 lines
(agreeing in 2+ sightings), and by eye on the clearest crop for the other 53.

Forced breaks (the recording breaks where the next word would still fit; all at a sentence end):
- s001a "Oh dear, oh dear, it's coming.<br>You'd better talk to Sir Vant. He's in charge."
- s005a "Wait! You're not properly armed.<br>Pick up the kit dropped by my squire, then attack the
  goblin when you've equipped it."
- s014 "You'll never get over the wall there.<br>Try going to the eastern end where it's lower."
- s031a "You're supposed to soak the rag in oil first.<br>Try using the rag with the oil can."
- s034 "Thanks, that's perfect.<br>Now, hand me a tinderbox."
- s061 "You'll need a Mining level of 2 to clear the rocks.<br>Your Mining level is 1."
- s062 "You find that you'll need a Woodcutting level of 2 to clear the roots.<br>Your Woodcutting
  level is 1." (the natural wrap breaks there too)
- s061a / s062a "So, your aim is to get a Mining / Woodcutting level of 2?<br>Okay, I'll find you
  something to train on."
- s065 / s065m "You break off some bits of wood / rock, gaining 42 Woodcutting / Mining
  experience.<br>Once more should suffice." (the natural wrap breaks there too)
- s070 / s071 "Congratulations, you've reached level 2.<br>You should now be able to mine through
  the rocks / chop through the tree roots in the damaged wall."
- s075b "I think we've all been in this cellar long enough.<br>Can we get on with the quest now,
  please?"

Old build only (kjgw48iBdc0, 2010-12-15), a message box not in the Task build's matrix: "Your
Woodcutting level is now high enough for you to chop through the roots in the damaged wall."

#### 9. "Ahem! I'm over here." and "Hey, I'm over here!" are both real

- Kw7h2jY9ANQ (2011-05-13) @170 shows Sir Vant's chatnpc "Ahem! I'm over here." The player is in
  "Assist Sir Vant" and has just tried "Climb-over Wall" (@168).
- The cache's voice set (3.6) has both lines as separate entries. So the wiki's Speech caption
  "Hey, I'm over here" is a second line, not a mishearing (corrected 2026-11-09). Which situation
  plays "Hey": no recording shows it.

#### 10. The advisor button is in the 2011 build

The "?" button by the minimap (toplevel `advisorbutton`, 548:183 / 746:172) appears silently
during the roof-collapse cutscene, while the screen is black before Roddeck walks out:
- 4KO2h2xPtdM: absent at 407.5, present at 408.
- Qh_-AMJpVd0 (2011-05-10): absent at 339, present at 341.

No line announces it in the Task build; the old build had Roddeck's "I'll add a button to your
display." It stays to the end (seen in Lumbridge, 4KO2h2xPtdM @607-630).

#### Overhead lines (new: not in the dialogue boxes)

From whole-frame OCR, checked on the frame:

| Speaker | Line | Where |
|---|---|---|
| Goblin | "Me get rich!" | rummaging (Kw7h2jY9ANQ @166) |
| Goblin | "Heh heh heh." | rummaging (Kw7h2jY9ANQ @170) |
| Goblin | "Ooh, shiny!" | rummaging (7_neJ6mCLXc @90; 4KO2h2xPtdM @50) |
| Goblin | "Mine, all mine!" | rummaging (Kw7h2jY9ANQ @185) |
| Goblin | "Ooh!" | rummaging (Qzq-h8kLBHk @101) |
| Dragon | "Roargh!" | the opening cutscene (7_neJ6mCLXc @74) |
| Squire | "Aaargh!" | hit by the dragon's breath (4KO2h2xPtdM @35, Qzq-h8kLBHk @80) |
| Squire | "Help!" | far off, in the dragon cutscene (4KO2h2xPtdM @22-23, dSHLzIzwXq4 @24); the squire's voice enum 3038 (3.6) |
| Sir Vant | "Grub's up! Shoo!" | sending the dragon to the bowl (4KO2h2xPtdM @204-205) |
| Sir Vant | "Wow!" | after the collapse (4KO2h2xPtdM @413-414) |

### 3.6 Voice-over: every voiced line in cache 786 (2026-11-09)

The quest is voiced (since 21 April 2010, Update:Slayer Challenges). Cache 786 holds every voiced
line as a vorbis clip, and ten enums with vorbis output (3036-3045) say which clip each speaker
uses for each line:
- one enum per speaker;
- keys 1..N follow the lines in alphabetical order;
- one clip per line.

The vorbis archive also holds alternate takes no enum uses: "Wait a moment - there's a dragon
coming!" has six clips. The enums settle which take the game played. No cache script reads these
enums (the server content did), so when each line plays comes from its text, the step order
(3.1) and the recordings.

- **Transcripts:** Whisper (openai/whisper-small.en) on every clip; near word-perfect, with
  names sometimes misspelt ("Roddick", "Zavant").
- **canon:** the column marks lines also seen in the recordings' dialogue boxes (3.4).
- **Box text versus speech:** box text is the recordings' (3.4); the speech can differ in small
  ways ("Ahem! I'm over here." in the box, "Ahem, I'm over here." spoken).
- **No recording has audible voices.** The client plays speech only when the player's
  voice-over volume isn't 0.
- **Playing a line:** the client's VORBIS_SPEECH_SOUND (server prot 7, 6 bytes: g2 clip
  (65535 = none), g1 loops, g2 delay, g1 volume) and VORBIS_SPEECH_STOP (66, empty). Content
  sends them with `sound_speech_volume(vorbis, loops, delay, volume)` and `sound_speech_stop`.
  The dev cheats `~speech <clip>` and `~speechstop` audition a clip.
- **Command names.**
  - `sound_speech_volume` is Jagex's name for this operation on the client. The NXT beta
    client's clientscript command names list it, as `sound_speech_volume(vorbis, int, int,
    int)`: Pazaz/RT4-Client `ref/cs2-names.txt` (headed "NXT Beta"), zwyz/rs3-cache
    `data/commands/audio.txt`, and skillbert/rsmv `opnames.ts` (RS3 opcode 10559).
  - In the 647 client it is clientscript opcode 3207. That opcode pops clip, loops, delay and
    volume, then calls the same `AClass55.method3670(volume, 256, delay, true, loops, clip)`
    as VORBIS_SPEECH_SOUND. Its neighbours match the same list: 3206 `sound_vorbis_volume`
    (`true` -> `false`, the effects channel) and 3209 `sound_vorbis_rate`.
  - No source names Jagex's server-side command. We give it the client command's name,
    since `sound_synth` has one name on both sides (the server's in the RuneStar leaks, the
    client's at clientscript 3200).
  - No source names a speech stop either. `sound_speech_stop` is ours, after the list's
    `sound_synth_stop`, `sound_vorbis_stop` and `sound_song_stop`.

| Enum | Lines | Clips | Speaker |
|---|---|---|---|
| 3036 | 131 | 1325-1784 | the guide narrator: the step-guide texts read aloud, mostly in pairs ("Right-click it and select..." / "Click it and select...": two- and one-button mouse). The older build's chatbox guide; the Task build replaced it with the task panel, which isn't voiced. |
| 3037 | 78 | 1332-1783 | the narrator for game messages and message boxes ("You swing your pick at the rock.") |
| 3038 | 4 | 1284-1319 | the squire: a scream, "Help!", "Oh dear, oh dear, it's coming! You'd better talk to Sir Vant! He's in charge!", "Uh oh." |
| 3039 | 110 | 687-1016 | Sir Vant |
| 3040 | 81 | 1020-3180 | Roddeck, and Explorer Jack at the end. Entries 1-70 are the older Objectives build's lines ("So, your objective is to get a mining level of 2"). Entries 71-81 (clips 3170-3180, the newest ids) are the Task build's rewrites and additions: "So, your aim is..." (as the 2011 boxes read, s061a/s062a), the chippings lines, "You should be able to mine / chop your way through ... doorway now.", "Thank you once again for your assistance.", "I'll explain later, Jack.", and Explorer Jack's "Good grief, where did you three come from?", "Begging your pardon, I need a quick word with you before you go." and the Task System line. Sir Vant's farewell at the exit is in his own enum, 3039 (clips 687, 688). |
| 3041 | 18 | 1801-1886 | the player, one voice |
| 3042 | 18 | 373-683 | the player, the other voice: the same 18 lines (inference: male and female) |
| 3043 | 8 | 1178-1248 | the goblin ("Ooh, shiny.", "LOOT", "Mine. All mine.") |
| 3044 | 10 | 691-700 | Roddeck's diary, read aloud |
| 3045 | 4 | 1249-1252 | the "On the Care of Dragons" manual, read aloud |

##### enum 3036 (131 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 1327 | 4.5 |  | You'll need to climb over the wall. Right click it and select climb over. |
| 2 | 1618 | 4.0 |  | You'll need to climb over the wall. Click it and select climb over. |
| 3 | 1384 | 7.9 |  | You need to raise your wood cutting level to 2. We'll set that as your objective. Click on the flashing icon to see your objective screen. |
| 4 | 1735 | 7.9 |  | You need to raise your mining level to 2. We'll set that as your objective. Click on the flashing icon to see your objective screen. |
| 5 | 1355 | 8.1 |  | Rotate the camera using the arrow keys until you can see the knight, Sir Vant. Right-click on him and select Talk to to get started. |
| 6 | 1335 | 6.8 |  | Return to the food bowl and search it to make sure the meat is alright. Click on the food bowl and choose search. |
| 7 | 1415 | 5.8 |  | Now that you're equipped for battle, click on the goblin and select Attack to begin fighting it. |
| 8 | 1454 | 6.4 |  | For a start, you'll need to get some meat from Savant's bag. Right-click on the bag and select Search. |
| 9 | 1633 | 6.2 |  | For a start, you'll need to get some meat from Savant's bag. Click on the bag and select Search. |
| 10 | 1408 | 16.5 |  | Explore the cellar until you find the wooden pillars. When you find them, you can put the meat in the nearby food bowl by clicking on the meat and selecting Use, then clicking on the bowl. Check the Quest Journal if you wish to remind yourself of Savant's instructions. |
| 11 | 1663 | 7.3 |  | Click on the plank in your inventory and select Use. Then click on the lowest section of the wall to use the plank on the wall. |
| 12 | 1369 | 4.1 |  | Click on the items on the floor, and select take, to pick up each one. |
| 13 | 1333 | 4.9 |  | You've got to prepare the two pillars for destruction using the supplies from Zavant's bag. |
| 14 | 1540 | 6.1 |  | You've embarked on the unstable foundations quest. Click on the flashing icon to view the quest journal. |
| 15 | 1774 | 3.2 |  | You're out. So Vann would like to say farewell now. |
| 16 | 1438 | 3.1 |  | Your worn items screen shows you the items you've got equipped. |
| 17 | 1521 | 4.7 |  | Your wood cutting level is now high enough for you to chop through the roots in the damaged wall. |
| 18 | 1602 | 19.1 |  | Your stats screen shows your level in each skill, and you can hover your mouse over the icons to see how much experience, or XP, you have in the skill. When you train a skill, you'll gain more XP, letting you advance to higher levels in that skill. At higher levels, you can perform more advanced actions. |
| 19 | 1681 | 2.9 |  | Your objective is to raise your wood cutting level to 2. |
| 20 | 1713 | 3.0 |  | Your objective is to raise your mining level to 2. |
| 21 | 1376 | 3.8 |  | Your mining level is now high enough for you to mine through the damaged wall. |
| 22 | 1395 | 4.7 |  | Your inventory shows the items you're carrying. Pick up the rest of the Squires kit. |
| 23 | 1480 | 4.3 |  | Your inventory shows the items you're carrying. Pick up the remaining item. |
| 24 | 1627 | 17.1 |  | Your current objective is to raise your wood cutting level to 2. The Objectives screen will track your progress towards this objective, and the Stats screen will show how much experience XP you've got. Use the Advisor button when you want RODex advice. |
| 25 | 1418 | 15.3 |  | Your current objective is to raise your mining level to 2. The objective screen will track your progress towards this objective and the stats screen will show how much experience XP you've got. Use the Advisor button when you want RODEX advice. |
| 26 | 1759 | 5.0 |  | Your character will keep fighting, until one of you dies, or decides to walk away. |
| 27 | 1748 | 8.8 |  | You'll see that Unstable Foundations is listed in yellow to indicate that you've started the quest. Click on it now to open the quest's journal. |
| 28 | 1599 | 8.6 |  | You'll need some meat from Savant's bag. Click on the bag to search it. Check the Quest Journal if you wish to remind yourself of Savant's instructions. |
| 29 | 1433 | 2.0 |  | You may now leave via the door. |
| 30 | 1631 | 5.7 |  | You didn't have the required skill level to do that. Click on the flashing icon to see your stats screen. |
| 31 | 1634 | 4.8 |  | You can check the Quest Journal at any time to remind yourself of Savant's instructions. |
| 32 | 1767 | 11.6 |  | While you've got a pickaxe, you can select the Damaged Walls Mine option to mine your way through the rocks. Pickaxes can be carried in your inventory and you can also wield them as weapons. |
| 33 | 1645 | 11.2 |  | While you've got a hatchet, you select the damage to walls chop option to chop through the roots that are blocking it. Hatchets can be carried in your inventory and you can also wield them as weapons. |
| 34 | 1340 | 13.2 |  | When you're using the world map to help you navigate, it helps if you rotate the camera so that north is at the top of your screen. The compass helps you find north. Click on the compass now to point the camera northwards. |
| 35 | 1651 | 4.2 |  | Use your tinderbox on the pillar to light the fuse, then run away. |
| 36 | 1374 | 11.2 |  | Use the objective screen to set yourself an objective. The random objective button will help you pick one. Once you have an objective, use the advisor button to consult RODEC. |
| 37 | 1657 | 6.0 |  | To wrap oily rags around the pillars, click on the oily rags in your inventory, then click on a pillar. |
| 38 | 1448 | 6.3 |  | to soak a rag in oil, click on the rag in your inventory to select Use, then click on the oil can. |
| 39 | 1517 | 5.1 |  | To rip the cloth into rags, click the rip option on the cloth in your inventory. |
| 40 | 1444 | 7.2 |  | To give Savant an item, right-click on it in your inventory and select Use. Then click on Savant. |
| 41 | 1736 | 6.9 |  | To give Savant an item, click on it in your inventory and select Use. Then click on Savant. |
| 42 | 1378 | 7.4 |  | To give Savant a tinderbox, click on a tinderbox in your inventory to select Use. Then click on Savant. |
| 43 | 1766 | 5.3 |  | To attach a fuse to a pillar, click on the fuse in your inventory. Then click on the pillar. |
| 44 | 1356 | 1.5 |  | Time to get on with the quest. |
| 45 | 1504 | 7.4 |  | This is the advisor screen. You can use it to get advice from RODEC at any time. Close the screen when you're ready to proceed. |
| 46 | 1490 | 5.6 |  | The wall is too high for you to climb. There must be something in this cellar that you could use to get over it. |
| 47 | 1416 | 7.7 |  | The wall is too high for you to climb. So Vance suggests that you try picking up the plank that he saw by the eastern wall of the cellar. |
| 48 | 1626 | 2.8 |  | The ramp should make it possible for you to climb over the wall. |
| 49 | 1705 | 14.2 |  | The objective screen recommends activities to you. It's a handy resource if you're ever looking for something to do. Now you've got an objective, Rodick can advise you on how to achieve it. Click the Advisor button to seek his advice. |
| 50 | 1731 | 2.4 |  | Talk to Savant to see how you're getting on. |
| 51 | 1513 | 2.5 |  | Talk to Zavant to find out what he wants. |
| 52 | 1782 | 3.8 |  | Speak to Roddick, your advisor, about breaking through the damaged wall. |
| 53 | 1350 | 12.7 |  | Savant wants his Squires kit back. To remove items that you're wearing, go to your worn items screen and click the items remove option. The item will be moved to your inventory. |
| 54 | 1372 | 4.6 |  | Savant is trying to explain his plan. Talk to him to hear the rest of it. |
| 55 | 1485 | 5.1 |  | Zavant is explaining how to kill the dragon. Talk to him to continue the conversation. |
| 56 | 1742 | 7.0 |  | Savantas pointed out that you're all stuck down the cellar. See if he or Rodic has a suggestion for how to escape. |
| 57 | 1656 | 1.1 |  | Run run run! |
| 58 | 1441 | 8.0 |  | Rotate the camera using the arrow keys until you can see the night servant. Click on him and select Talk to to get started. |
| 59 | 1473 | 2.6 |  | Rodic would like to offer you some parting advice. |
| 60 | 1571 | 8.6 |  | Right click on the plank in your inventory and select Use. Then click on the lowest section of the wall to use the plank on the wall. |
| 61 | 1506 | 4.3 |  | Right click on the items on the floor and select Take to pick up each one. |
| 62 | 1543 | 5.4 |  | Right click on an item in your inventory and select wheeled or where to equip it. |
| 63 | 1696 | 7.3 |  | Return to the food bowl and search it to make sure the meat is all right. Right click on the food bowl and choose search. |
| 64 | 1690 | 4.2 |  | Now, pick up the rest of the Squires Kit, by clicking its Take option. |
| 65 | 1398 | 4.9 |  | Now, pick up the final item and equip it. Then you'll be ready to fight the goblin. |
| 66 | 1710 | 7.3 |  | Now, equip the other two items by clicking their wheeled or wear options. You'll need to be viewing your inventory to do this. |
| 67 | 1461 | 7.7 |  | Now equip the items by right clicking on them and selecting wheeled or wear. You'll need to be viewing your inventory to do this. |
| 68 | 1412 | 7.4 |  | Now equip the items by clicking on them and selecting wheeled or wear. You'll need to be viewing your inventory to do this. |
| 69 | 1607 | 3.9 |  | Now equip the final item and you'll be ready to fight the goblin. |
| 70 | 1424 | 5.6 |  | Now that you're equipped for battle, right-click on the goblin and select Attack to begin fighting it. |
| 71 | 1699 | 3.6 |  | Now click on the World Map button to see a map of this seller. |
| 72 | 1443 | 6.1 |  | Great, you've got the dragon in the right place. Talk to Savant to find out the next part of his plan. |
| 73 | 1664 | 17.3 |  | Explore the cellar until you find the wooden pillars. When you find them, you can put the meat in the nearby food bowl by right-clicking on the meat and selecting Use, then clicking on the bowl. Check the Quest Journal if you wish to remind yourself of Savant's instructions. |
| 74 | 1562 | 4.4 |  | Excellent. You can click on the stumps of the roots to climb through the hole now. |
| 75 | 1650 | 3.9 |  | Excellent, you can click on the remaining rubble to climb through the hole now. |
| 76 | 1498 | 1.8 |  | Escape up the ladder to freedom. |
| 77 | 1325 | 6.2 |  | Congratulations, the dragon's dead. Speak to Rodic, the man who's just emerged from the cupboard. |
| 78 | 1362 | 7.4 |  | Click on the question mark button at the top right of your screen to see the advisor screen. You can use it to get advice from RODEC at any time. |
| 79 | 1700 | 3.4 |  | Click on the flashing icon to view your worn items screen. |
| 80 | 1409 | 2.8 |  | Click on the flashing icon to view your inventory. |
| 81 | 1569 | 4.7 |  | Click on an item in your inventory and select wheeled or where to equip it. |
| 82 | 1732 | 4.1 |  | Check the Quest Journal, if you wish to remind yourself of Savant's instructions. |
| 83 | 1547 | 8.7 |  | As you get more familiar with RuneScape, you'll learn other forms of combat and meet tougher creatures. Talk to Savant to find out what he wants now. |
| 84 | 1361 | 2.5 |  | You've successfully levelled up in wood cutting. |
| 85 | 1525 | 2.8 |  | you've successfully leveled up in mining. |
| 86 | 1544 | 2.9 |  | You can use it to seek RODEX advice at any time. |
| 87 | 1337 | 0.8 |  | Where am I? |
| 88 | 1619 | 1.7 |  | What is wood cutting for? |
| 89 | 1538 | 1.2 |  | What is mining for? |
| 90 | 1502 | 1.5 | s006 | We'll come back to that in a moment. |
| 91 | 1354 | 3.4 |  | This is the advisor screen. |
| 92 | 1414 | 2.7 |  | Rodek will then be able to offer you advice about it. |
| 93 | 1491 | 3.0 |  | Rodek will offer more advice once you finish this quest. |
| 94 | 1329 | 3.0 |  | Rodek looks at your inventory and your surroundings. |
| 95 | 1553 | 1.8 |  | Please set yourself an objective. |
| 96 | 1589 | 2.0 |  | How can I raise my wood cutting level? |
| 98 | 1588 | 1.1 |  | General advice. |
| 99 | 1784 | 3.3 |  | From this, he tries to work out what advice you might want. |
| 100 | 1637 | 2.2 |  | Close the screen when you're ready to continue. |
| 101 | 1483 | 3.8 |  | Use the Advisor button for help on completing your objective. |
| 102 | 1653 | 2.9 | s087 | As you leave, Rodek wanders off home. |
| 103 | 1474 | 5.5 |  | You don't need to keep clicking. Once you've started, your character will keep fighting automatically. |
| 104 | 1716 | 16.8 | s017 | Your character can run if they have sufficient energy. You can make your character run automatically by turning on run mode. Click the boot icon by the minimap. If you hold down the control key while clicking on the floor or minimap, your character will run to their destination. |
| 105 | 1661 | 7.0 |  | If you ever forget what you were asked to do, take a look in the journal to remind yourself. Close the journal when you've finished reading it. |
| 106 | 1756 | 7.2 |  | You can also click the Get Quest Help link to get hints for how to proceed with the quest. Close the journal when you've finished reading it. |
| 107 | 1704 | 3.5 |  | Quest journals record what you're supposed to be doing in each quest. |
| 108 | 1669 | 7.7 | s077 | When you drop items on the ground, they will eventually vanish. It's a good way to get rid of junk, but don't drop anything you want to keep. |
| 109 | 1572 | 11.5 |  | Your worn items screen shows you the items you've got equipped. Now, equip the items by right-clicking on them and selecting wheeled or wear. You'll need to be viewing your inventory to do this. |
| 110 | 1600 | 10.9 |  | Your worn items screen shows you the items you've got equipped. Now, equip the items by clicking on them and selecting wheeled or wear. You'll need to be viewing your inventory to do this. |
| 111 | 1702 | 7.6 |  | Your worn items screen shows you the items you've got equipped. Now, pick up the rest of the Squires kit, by clicking its take option. |
| 112 | 1654 | 10.3 |  | Your worn items screen shows you the items you've got equipped. Now, equip the other two items by clicking their wheeled or wear options. You'll need to be viewing your inventory to do this. |
| 113 | 1604 | 7.5 |  | Your worn items screen shows you the items you've got equipped. Now, equip the final item, and you'll be ready to fight the goblin. |
| 114 | 1545 | 8.0 |  | Your worn items screen shows you the items you've got equipped. Now, pick up the final item and equip it. Then you'll be ready to fight the goblin. |
| 115 | 1673 | 9.3 |  | Your worn items screen shows you the items you've got equipped. Now that you're equipped for battle, right-click on the goblin and select Attack to begin fighting it. |
| 116 | 1472 | 8.8 |  | Your worn items screen shows you the items you've got equipped. Now that you're equipped for battle, click on the goblin and select Attack to begin fighting it. |
| 117 | 1747 | 11.3 |  | To soak a rag in oil, click on the rag in your inventory to select Use, then click on the oil can. You can check the Quest Journal at any time to remind yourself of Savant's instructions. |
| 118 | 1649 | 10.1 |  | to rip the cloth into rags, click the rip option on the cloth in your inventory. You can check the Quest Journal at any time to remind yourself of Savannah's instructions. |
| 119 | 1360 | 10.3 |  | You've got to prepare the two pillars for destruction using the supplies from Savant's bag. You can check the Quest Journal at any time to remind yourself of Savant's instructions. |
| 120 | 1373 | 10.9 |  | To wrap oily rags around the pillars, click on the oily rags in your inventory, then click on a pillar. You can check the Quest Journal at any time to remind yourself of Savant's instructions. |
| 121 | 1730 | 10.8 |  | To attach a fuse to a pillar, click on the fuse in your inventory, then click on the pillar. You can check the Quest Journal at any time to remind yourself of Savant's instructions. |
| 122 | 1622 | 6.0 |  | time to get on with the quest. You'll need to climb over the wall, right click it and select climb over. |
| 123 | 1682 | 6.2 |  | Time to get on with the quest. You'll need to climb over the wall. Click it, and select, climb over. |
| 124 | 1476 | 8.3 |  | time to get on with the quest. For a start, you'll need to get some meat from Savant's bag. Right-click on the bag and select Search. |
| 125 | 1331 | 8.5 |  | time to get on with the quest. For a start, you'll need to get some meat from Savant's bag. Click on the bag and select Search. |
| 126 | 1697 | 27.5 |  | Your stats screen shows your level in each skill and you can hover your mouse over the icons to see how much experience or XP you have in the skill. When you train a skill, you'll gain more XP, letting you advance to higher levels in that skill. At higher levels, you can perform more advanced actions. You need to raise your mining level to 2. We'll set that as your objective. Click on the flashing icon to see your objective screen. |
| 127 | 1436 | 26.5 |  | Your stats screen shows your level in each skill and you can hover your mouse over the icons to see how much experience or XP you have in the skill. When you train a skill, you'll gain more XP, letting you advance to higher levels in that skill. At higher levels, you can perform more advanced actions. You need to raise your wood cutting level to 2. We'll set that as your objective. Click on the flashing icon to see your objective screen. |
| 128 | 1587 | 16.4 |  | Your objective is to raise your mining level to 2. The objective screen recommends activities to you. It's a handy resource if you're ever looking for something to do. Now you've got an objective, Rodick can advise you on how to achieve it. Click the advisor button to seek his advice. |
| 129 | 1533 | 17.1 |  | Your objective is to raise your wood cutting level to 2. The objective screen recommends activities to you. It's a handy resource if you're ever looking for something to do. Now you've got an objective, Rodick can advise you on how to achieve it. Click the Advisor button to seek his advice. |
| 130 | 1400 | 11.1 |  | Quest journals record what you're supposed to be doing in each quest. If you ever forget what you are asked to do, take a look in the journal to remind yourself. Close the journal when you've finished reading it. |
| 131 | 1603 | 16.3 |  | Quest journals record what you're supposed to be doing in each quest. If you ever forget what you are asked to do, take a look in the journal to remind yourself. You can also click the Get Quest Help link to get hints for how to proceed with the quest. Close the journal when you've finished reading it. |
| 132 | 1395 | 4.7 |  | Your inventory shows the items you're carrying. Pick up the rest of the Squires kit. |

##### enum 3037 (78 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 1555 | 5.9 |  | Your inventory is too full to hold any more wood. Free up some space by dropping things you don't need. |
| 2 | 1431 | 1.7 | s063 | You swing your pick at the rock. |
| 3 | 1515 | 3.6 |  | You break off some bits of rock and gain a little mining experience. |
| 4 | 1482 | 1.7 | s065a | You break off some more bits of rock. |
| 5 | 1684 | 2.0 | s064 | You swing your hatchet at the root. |
| 6 | 1526 | 3.9 |  | You break off some little bits of wood, gaining a little wood cutting experience. |
| 7 | 1777 | 1.8 | s065a | You break off some more bits of wood. |
| 8 | 1352 | 2.5 |  | It would be rude to take that without permission. |
| 9 | 1558 | 1.4 |  | You won't be needing that again. |
| 10 | 1382 | 2.8 | s027 | You rip the cloth and end up with two rags. |
| 11 | 1452 | 3.5 | s030 | You rip the oily cloth and end up with two oily rags. |
| 12 | 1541 | 1.5 | s029 | You soak the cloth in oil. |
| 13 | 1379 | 1.4 | s028 | You soak the rag in oil. |
| 14 | 1344 | 1.9 | s031 | The cloth has already been soaked in oil. |
| 15 | 1598 | 1.9 | s031 | The rag has already been soaked in oil. |
| 16 | 1770 | 2.1 |  | There's no need to put oil on the fuse. |
| 18 | 1683 | 3.7 |  | The pillar has been reduced to a stump. There's nothing more to be done with it. |
| 19 | 1641 | 2.3 |  | This doesn't seem like a good time to mess with it. |
| 20 | 1332 | 2.3 |  | Perhaps you should wait until Savant is ready. |
| 21 | 1773 | 2.7 |  | That pill has already got an oil soaked cloth around it. |
| 22 | 1698 | 1.9 | s032 | You wrap the oily rag around the pillar. |
| 23 | 1621 | 1.6 | s033 | You attach the fuse to the pillar. |
| 25 | 1550 | 2.2 | s023a | The bag contains various supplies. |
| 26 | 1608 | 4.8 | s013 | The bag contains several parcels of meat, all wrapped up in paper. You take one. |
| 27 | 1739 | 6.5 |  | There's nothing useful left in the bag except for savants spare underpants, which you leave well alone. |
| 28 | 1488 | 3.4 | s023 | You search Savant's bag, and take a selection of supplies. |
| 29 | 1451 | 3.7 |  | You search Savant's bag. It's full of miscellaneous supplies. |
| 30 | 1769 | 3.8 | s015 | Even this low section of the wall is too high for you to climb over. |
| 31 | 1439 | 4.7 |  | Well done. Now you just need to put the meat in the dragon's food bowl. |
| 32 | 1764 | 1.8 |  | You won't need another one of those. |
| 33 | 1364 | 1.1 |  | That'll come later. |
| 34 | 1401 | 2.4 | s016 | You use the plank to construct a crude ramp. |
| 35 | 1596 | 3.3 |  | There are definitely easier ways of getting past this wall. |
| 36 | 1413 | 4.6 |  | The door won't open. It feels like someone's holding it closed from inside. |
| 37 | 1381 | 2.4 |  | It's full of cabbages. You've already got one. |
| 38 | 1701 | 4.1 |  | It's full of cabbages. Your inventory is too full for you to take one. |
| 39 | 1561 | 2.4 | s050 | It's full of cabbages. You take one. |
| 40 | 1615 | 3.2 |  | You eat the cabbage. It restores some of your life points. |
| 41 | 1595 | 1.0 | s051 | You eat the cabbage. |
| 42 | 1345 | 1.9 | s049 | You find a couple of books and take them. |
| 43 | 1592 | 4.9 |  | There are a couple of interesting looking books on the shelf, but you only have room to take one of them. |
| 44 | 1385 | 4.8 |  | There are a couple of interesting looking books on the shelf, but you don't have room to hold them. |
| 45 | 1419 | 1.4 |  | You take a book from the shelf. |
| 46 | 1530 | 4.0 |  | There's an interesting looking book on the shelf, but you don't have room to hold it. |
| 47 | 1668 | 2.2 |  | There's nothing particularly interesting here. |
| 48 | 1737 | 3.1 |  | There's not much point in trying to feed that to the dragon. |
| 49 | 1406 | 2.6 |  | You don't have enough inventory space to pick up the meat. |
| 50 | 1459 | 1.7 |  | You won't need any more meat now. |
| 51 | 1511 | 3.0 |  | You don't have enough space in your inventory to pick up the meat. |
| 52 | 1594 | 1.1 |  | What would you like to say? |
| 53 | 1556 | 1.8 |  | You offers a vent at tinderbox. |
| 54 | 1460 | 5.5 |  | You don't need to keep clicking. Once you've started, your character will keep fighting automatically. |
| 55 | 1692 | 3.3 | g004 | Congratulations, Quest Complete. |
| 56 | 1514 | 6.3 |  | You have completed unstable foundations. You gain one quest point and level four mining. |
| 57 | 1579 | 6.8 |  | You have completed unstable foundations. You gain one quest point and level four wood cutting. |
| 58 | 1653 | 2.9 | s087 | As you leave, Rodek wanders off home. |
| 59 | 1680 | 1.9 | s059 | You attempt to mine through the rocks. |
| 60 | 1531 | 4.0 | s061 | You'll need a mining level of 2 to clear the rocks. Your mining level is 1. |
| 61 | 1586 | 2.5 | s072 | you succeed in clearing the rocks from the hole. |
| 62 | 1420 | 1.9 | s060 | You attempt to chop through the roots. |
| 63 | 1353 | 5.5 | s062 | You find that you'll need a wood cutting level of 2 to clear the roots. Your wood cutting level is 1. |
| 64 | 1629 | 2.4 | s073 | You succeed in clearing the roots from the hole. |
| 65 | 1754 | 2.2 |  | There doesn't seem much point in going back in there. |
| 66 | 1776 | 3.7 |  | Roddick retrieves his pickaxe, his diary and his book from you. |
| 67 | 1725 | 2.9 |  | Rodek retrieves his pickaxe and his diary from you. |
| 68 | 1647 | 2.8 | s074 | Rodic retrieves his pickaxe and his book from you. |
| 69 | 1632 | 2.0 | s074 | Roddick retrieves his pickaxe from you. |
| 70 | 1447 | 3.4 |  | Rodek retrieves his hatchet, his diary and his book from you. |
| 71 | 1783 | 3.0 |  | Rodek retrieves his hatchet and his diary from you. |
| 72 | 1758 | 2.7 |  | Rodek retrieves his hatchet and his book from you. |
| 73 | 1484 | 2.1 | s075 | Rodic retrieves his hatchet from you. |
| 74 | 1509 | 1.6 | s057 | Rodic gives you a pickaxe. |
| 75 | 1341 | 1.6 | s058 | Rodic gives you a hatchet. |
| 76 | 1628 | 1.8 | s057 | Rodic gives you another pickaxe. |
| 77 | 1590 | 1.7 | s058 | Rodic gives you another hatchet. |
| 78 | 1457 | 3.4 |  | Congratulations! You have completed your objective. |
| 79 | 1560 | 5.0 | g006 | You've just advanced a mining level. You have reached level 4. |
| 80 | 1638 | 5.1 |  | You've just advanced a wood cutting level. You have reached level four. |

##### enum 3038 (4 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 1292 | 1.4 |  | AHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH ––Tick ––Tick–Tick–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T–T |
| 2 | 1284 | 1.4 |  | Help! |
| 3 | 1297 | 6.3 | s001a | Oh dear, oh dear, it's coming! You'd better talk to Sir Van! He's in charge! |
| 4 | 1319 | 1.1 |  | Uh oh. |

##### enum 3039 (110 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 763 | 5.7 | s038 | Ah, here you are, Roddick. The White Knights got your message and sent me to slay your dragon. |
| 2 | 1004 | 2.1 |  | Ahem, I'm over here. |
| 3 | 930 | 4.6 | s018 | Are you mad? Don't eat that, it's raw and it's covered in sleeping potion. |
| 4 | 3169 | 9.1 | s041c | I'd like my Squire's kit back so I can return it to the Porchaps family. Take it off and hand it to me, please. I'd like my tinderbox and oil can, too. |
| 5 | 705 | 9.7 | s041 | Before we go any further, could I have my squires kit back, please? I ought to return it to the poor chap's family. I'd like my tinderbox and oil can, too. |
| 6 | 998 | 1.9 |  | Big boom coming up! |
| 7 | 1008 | 2.1 |  | Come behind the wall to talk to me please. |
| 8 | 993 | 2.5 | s022 | Could you get the rest of the supplies for my bag, please? |
| 9 | 890 | 7.9 |  | Could you possibly leave my squire's kid behind? I ought to return it to his family. Just drop it on the floor, or hand it to me. |
| 10 | 977 | 3.6 |  | Don't give stuff to the dragon. It won't thank you, you know. |
| 11 | 834 | 4.7 |  | Don't stick the meat on the pillar. You're meant to put it in a bowl for the dragon to eat. |
| 12 | 978 | 4.4 |  | Excellent! You look ready to fight. You can deal with the goblin now. |
| 13 | 820 | 5.0 |  | Finally attach a fuse to each pillar. That'll give us time to run away when we light them. |
| 14 | 876 | 7.6 | s025 | First we need to wrap the pillars in oily rags. There was a big cloth in my bag. Rip it in half, then soak the pieces in oil. |
| 15 | 754 | 3.0 |  | First you need to get both pillars wrapped in oily rags. |
| 16 | 938 | 5.7 |  | First you need to wrap both pillars in oily rags. I see you've worked out how to make oily rags. |
| 17 | 891 | 6.1 |  | First you need to wrap both pillars in oily rags. You can make oily rags by ripping up that big oily cloth you've got. |
| 18 | 800 | 8.1 |  | First you need to wrap both pillars in oily rags. You can make oily rags by ripping up the big cloth from my bag and using my can of oil on the rags. |
| 19 | 728 | 6.8 |  | First you need to wrap both pillars in oily rags. You can make oily rags by using my can of oil with rags of cloth. |
| 20 | 966 | 4.2 |  | Get some more meat from my bag and put it in the food bowl by the wooden pillars, please. |
| 21 | 729 | 4.3 |  | Go on, finish off the goblin. It's too weak to be a real threat. |
| 22 | 784 | 3.7 |  | Great! We can climb through the hole in the wall now. Thank you. |
| 23 | 850 | 2.6 |  | Grubs up, shoo! |
| 24 | 870 | 1.8 |  | Have you got the stuff from my bag? |
| 25 | 880 | 2.1 |  | Hey, don't walk in front of the dragon! |
| 26 | 1016 | 6.6 |  | Hey, you'll land on the dragon if you climb over the wall there. Try going to the eastern end where the walls lower. |
| 27 | 819 | 4.3 | s013a | Hey, don't just drop it on the floor. You're supposed to be placing it in a food bowl. |
| 28 | 701 | 2.2 |  | Hey, I'm over here! |
| 29 | 757 | 1.2 | s026a | How are you getting on? |
| 30 | 836 | 4.9 |  | I appreciate your enthusiasm, but could you please wait until I finish telling you what to do? |
| 31 | 807 | 5.3 | s005 | I can't turn my back on this dragon. I need you to take my squire's equipment and deal with the goblin. |
| 32 | 798 | 2.1 | s033b | I don't think I need that just at the moment, thank you. |
| 33 | 882 | 1.7 | s003 | I need some help over here. |
| 34 | 1011 | 1.8 | s019a | I really don't think you should do that. |
| 35 | 859 | 4.3 |  | I see you're holding a plank. You could use it against the wall to create a ramp. |
| 36 | 919 | 5.1 |  | I see you finished preparing the pillars. Could you hand me one of the tinder boxes for my bag please? |
| 37 | 776 | 3.2 |  | I see you've got both pillars nicely wrapped in oily rags now. |
| 38 | 796 | 4.4 |  | I think I saw a plank over by the Eastern Wall. Maybe you'll be able to use that. |
| 39 | 799 | 4.7 |  | I think it would be better if you lit the other pillar. I'll light this one at the same time. |
| 40 | 720 | 4.7 | s075b | I think we've all been in this cellar long enough. Can we get on with the quest now please? |
| 41 | 991 | 5.8 |  | I think you should wrap the oily rag around the pillar before adding the fuse. I'm sure it'll work better that way. |
| 42 | 687 | 7.0 | s083 | I'd better head back to Falador. My squire's family will need to be informed of his death. It's been a pleasure working with you. |
| 43 | 840 | 3.1 |  | I'd like my oil can back too, now that you've finished with it. |
| 44 | 778 | 3.7 |  | I'd like my tinderbox and oil can back too, now that you've finished with them. |
| 45 | 780 | 3.5 |  | I'd like my tinderbox back too, now that you've finished with it. |
| 46 | 940 | 2.8 |  | If you don't know what you're doing, perhaps you should ask me. |
| 47 | 726 | 5.1 | s012 | I'll keep the dragon's attention. Once you've got the drugged meat, you'll need to climb over the wall next to me. |
| 48 | 837 | 3.9 | s045 | I'm sure Rodda can teach you how to clear the rocks or the tree roots out of the way. |
| 49 | 806 | 8.3 | s011 | In my bag is some meat I prepared with a sleeping potion. I need you to put the meat in a food bowl by the wooden pillars. The dragon will eat it and fall asleep. |
| 50 | 709 | 5.8 |  | It worked! I see the owner of this cellar has emerged. I think he owes you a word of thanks. |
| 51 | 881 | 1.6 | s037 | It's burning. Run! |
| 52 | 715 | 1.9 | s019 | It's not the meat. Hide! |
| 53 | 860 | 2.5 | s035a | Just hand me one of those tinder boxes from my bag. |
| 54 | 907 | 9.7 | s026c | Me? Touch oily rags? You've clearly never tried to clean oil off white armor. No, I'll let you have all the fun. |
| 55 | 811 | 6.0 |  | My squire dropped his sword and armor when he died. You can pick them up and use them to defend my bag from the goblin. |
| 56 | 785 | 1.8 | s040 | Next time, get a puppy. |
| 57 | 908 | 2.8 |  | No thanks. Drop it on the floor if you don't need it. |
| 58 | 795 | 5.6 | s024 | Now, we're going to burn these wooden pillars and bring the ceiling down on this fat old worm. |
| 59 | 947 | 4.9 | s075a | Ah, well done. Climb through the hole, and we can all get out of this cellar. |
| 60 | 932 | 5.1 | s022b | Oh? Well, search my bag and bring the supplies to me before the dragon wakes up. |
| 61 | 943 | 6.3 | s010 | One corner of the ceiling is held up by wooden pillars. We're going to lure the dragon there and then collapse the roof on top of it. |
| 62 | 723 | 2.1 |  | Perhaps you shouldn't throw that away. |
| 63 | 920 | 1.8 | s042 | Please hand over my Squires kit. |
| 64 | 773 | 3.1 |  | Put that drug meat in the food bowl by the wooden pillars, please. |
| 65 | 821 | 6.2 |  | Really? The dragon should have smelted by now. Go back and search the bowl to make sure the meat's alright. |
| 66 | 861 | 4.9 | s043 | Right, I think that's everything. Now we just need a new way out of the cellar. |
| 67 | 745 | 6.4 |  | See if you can dig us out of this cellar. Rodek will advise you on what to do. I'll just stand here and watch. |
| 68 | 854 | 6.7 |  | So, Rodic's offering to be an advisor for you. That might be useful. It's a big world out there and he can teach you a lot. |
| 69 | 887 | 5.3 | s004 | Thank goodness you're here. My squire is dead, and there's a goblin going through my bag. |
| 70 | 704 | 6.7 | s078 | Thank you again for your help with the dragon. You'll receive a little reward for completing the quest when you get outside. |
| 71 | 730 | 5.5 | s007 | Thank you. Now, to deal with the dragon. Come here and I'll tell you my plan. |
| 72 | 893 | 5.1 | s036 | Thank you. When you're ready, set fire to the other pillar. I'll light this one at the same moment. |
| 73 | 931 | 4.0 | s034 | Thanks, that's perfect. Now hand me a tinderbox. |
| 74 | 838 | 3.1 |  | Thanks. I'd like my oil can back too, please. |
| 75 | 765 | 3.2 |  | Thanks. I'd like my tinderbox back too, please. |
| 76 | 744 | 2.4 | s041a | Thanks. Keep them coming, please. |
| 77 | 786 | 2.8 |  | That pillar's already got an oil-soaked cloth around it. |
| 78 | 737 | 3.9 |  | The dragon destroyed the Selalandar, so we'll need a new way out. |
| 79 | 905 | 2.6 |  | The pillars already fully prepared for burning. |
| 80 | 810 | 5.4 |  | There must be some object in this cellar that could help you get over the wall. Talk to me again if you get stuck. |
| 81 | 732 | 4.6 | s009 | This dragon's a tougher customer than I'd expected, but I know a way we can defeat it. |
| 82 | 792 | 5.3 |  | Use one of the tinder boxes from my bag to set light to the other pillar. I light this one at the same time. |
| 83 | 833 | 4.5 | s033a | Wait a minute! You're supposed to get both pillars prepared before we light them! |
| 84 | 770 | 2.1 | s002 | Wait a moment, there's a dragon coming! |
| 85 | 758 | 4.5 |  | Wait, you're not properly armed. You should equip all the kit you've picked up. |
| 86 | 988 | 6.0 | s005a | Wait! You're not properly armed! Pick up the kit dropped by my squire, then attack the goblin when you've equipped it! |
| 87 | 911 | 3.5 | s021 | We need to destroy these pillars to collapse the ceiling on the dragon. |
| 88 | 958 | 3.4 | s020 | Well done. Come out of hiding and I'll tell you what we're doing next. |
| 89 | 922 | 3.6 | s048a | Well, I would, but my armor's terribly hard to clean when it gets dirty. |
| 90 | 688 | 4.3 | s082 | Well, my mission is accomplished now. Rodex Dragon is no longer a threat. |
| 91 | 724 | 4.2 |  | We're going to burn down these wooden pillars to collapse the ceiling on the dragon. |
| 92 | 804 | 7.2 | s078b | We've nearly finished the quest now. Once you climb that ladder, you'll be free to start exploring the world outside this cellar. |
| 93 | 935 | 8.8 | s044 | When the ceiling collapsed, it damaged that wall. It looks like there are two places where you could break through. One's blocked by rocks, the other by tree roots. |
| 94 | 788 | 4.1 |  | When you've got the rest of my squire's kit equipped, you'll be ready to deal with the goblin. |
| 95 | 846 | 4.1 |  | When you've got the rest of my squire's kit equipped, you'll be ready to deal with the goblin. |
| 96 | 845 | 1.3 |  | Wonderful! |
| 97 | 782 | 1.2 |  | Wow! |
| 98 | 844 | 3.8 | s026 | Wrap the oily rags around the pillars then add a fuse to each one. |
| 99 | 772 | 7.3 |  | Yes, go to the eastern side of the cellar on my right and climb over the wall there. I mustn't move or I'll lose the dragon's attention. |
| 100 | 706 | 1.6 |  | You can leave that alone now. |
| 101 | 913 | 5.8 | s034a | You'll need to give me a tinderbox first. Otherwise, I can't light my pillar at the same time as you like yours. |
| 102 | 755 | 4.0 | s014 | You'll never get over the wall there. Try going to the eastern end where it's lower. |
| 103 | 791 | 2.5 |  | You're supposed to rip the cloth in half first. |
| 104 | 852 | 5.5 |  | You're supposed to rip the cloth into two rags, then soak the rags in oil before sticking them to the pillar. |
| 105 | 750 | 4.7 | s031a | You're supposed to soak the rag in oil first. Try using the rag with the oil can. |
| 106 | 992 | 2.6 |  | You've already lured the dragon to the right place. |
| 107 | 703 | 5.1 |  | You've already lured the dragon to the right place. We don't need to mess around with its food anymore. |
| 108 | 939 | 5.7 |  | You've got the fuses sorted out too. Great. Now just hand me one of those tinder boxes from my bag. |
| 109 | 739 | 2.8 | s008 | I don't believe I gave you permission to go through my bag. |
| 110 | 950 | 2.8 |  | I haven't finished explaining what you've got to do. |

##### enum 3040 (81 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 1101 | 6.3 | s062b | Ah, I have something. You can practice chopping those roots growing through the wall. |
| 2 | 1067 | 8.6 | s061b | Ah, I have something. You can practice mining on the rocks that fell into my cellar when the roof collapsed. |
| 3 | 1095 | 7.8 |  | Ah, I see you've run into a little difficulty. In a moment, I'll have some advice for you. |
| 4 | 1098 | 8.7 |  | As your mining level increases, you'll be able to mine for more valuable ores. You'll also get ores much faster. |
| 5 | 1282 | 9.8 | s081 | At last we're out. Now Savant has accomplished his mission to rescue me for my pet dragon, so I suppose he'll be off. |
| 6 | 1142 | 9.9 | s071 | Congratulations! You've reached level 2. You should now be able to chop through the tree roots in the damaged wall. |
| 7 | 1074 | 8.8 | s070 | Congratulations! You've reached level 2. You should now be able to mine through the rocks in the damaged wall. |
| 8 | 1131 | 3.7 |  | Could you perhaps come and chat with me before doing that? |
| 9 | 1141 | 3.9 |  | Could you perhaps come and have a chat with me before doing that? |
| 10 | 1100 | 7.1 |  | Could you possibly leave my hatchet behind? Just drop it on the floor or hand it to me. |
| 11 | 1127 | 4.1 |  | Don't forget to use my advisor button to get my advice. |
| 12 | 1260 | 5.8 |  | Excuse me, I hadn't finished talking to you. |
| 13 | 1129 | 4.2 |  | Go on, use the button to call me for advice. |
| 14 | 1253 | 4.4 | s084 | Goodbye, Savant, and thank you again. |
| 15 | 1065 | 5.6 |  | Great! We can climb through the hole in the wall now. Thank you. |
| 16 | 1047 | 3.5 | s033b | I don't think I need that at the moment. Thank you. |
| 17 | 1035 | 4.7 |  | I love advising people. It makes me feel important. |
| 18 | 1258 | 6.9 |  | I suppose this must be your first visit to Lumbridge, so I'll give you some advice before you go. |
| 19 | 1030 | 9.0 |  | I suppose you'd better return Zavant's stuff to him. Don't worry. I can tell you where to buy your own armour later. |
| 20 | 1094 | 10.8 | s078a | Hmm, I think this ladder leads to the house of my neighbor. Explorer Jack. He's an understanding chap. He won't mind us walking through his house. |
| 21 | 1274 | 3.4 |  | I think we've all had quite enough action down there. |
| 22 | 1027 | 3.9 | s041b | I think you should be returning that item to Savant, not to me. |
| 23 | 1023 | 4.0 |  | I thought you were meant to be learning mining, not wood cutting. |
| 24 | 1022 | 5.4 |  | I thought you were meant to be learning wood cutting, not mining. |
| 25 | 1087 | 6.3 |  | I'd like my hatchet back too. Just drop it on the floor or hand it to me. |
| 26 | 1116 | 6.1 |  | I'd like my pickaxe back too. Just drop it on the floor or hand it to me. |
| 27 | 1084 | 5.7 |  | I'll add a button to your display. Use it when you want my advice. |
| 28 | 1280 | 2.5 |  | I'll take that if you don't mind. |
| 29 | 1134 | 9.4 |  | I am incredibly grateful to you for disposing of the Dragon. In thanks, allow me to offer my services as your advisor. |
| 30 | 1148 | 3.8 |  | Keep chopping those roots until your wood cutting level reaches 2. |
| 31 | 1028 | 4.0 |  | Keep mining those rocks until your mining level reaches 2. |
| 32 | 1054 | 3.5 |  | Let's come back to that once we've got out of this cellar. |
| 33 | 1072 | 6.5 |  | Mining is used for breaking up rocks and extracting valuable ores from inside them. |
| 34 | 1031 | 5.9 | s058a | Now you've got a hatchet, chop your way through the roots inside the damaged wall. |
| 35 | 1037 | 6.4 | s058a | Now you've got a pickaxe. Mine your way through the rocks inside the damaged wall. |
| 36 | 1071 | 4.5 |  | Oh dear, you seem to have lost the hatchet I gave you. |
| 37 | 1091 | 5.4 |  | Oh dear, you seem to have lost the pickaxe I gave you. |
| 38 | 1079 | 7.1 |  | Once we're out of this cellar, I'll be able to advise you about where to get useful items like those. |
| 39 | 1029 | 13.1 |  | Perhaps you should concentrate on the objective you've already got. Remember, we can't get out of this cellar until you've raised your mining level to two and mined through the damaged wall. |
| 40 | 1136 | 13.2 |  | Perhaps you should concentrate on the objective you've already got. Remember, we can't get out of this cellar until you've raised your wood cutting level to two and chopped through the damaged wall. |
| 41 | 1145 | 7.9 | s061a | So, your objective is to get a mining level of 2. Okay, I'll find you something to train on. |
| 42 | 1133 | 8.1 | s061a | So, your objective is to get a wood cutting level of 2. Okay, I'll find you something to train on. |
| 43 | 1102 | 8.5 | s039 | Thank you both very much. It's had me trapped in the cupboard for ages. I'd completely lost control of it. |
| 44 | 1272 | 8.0 |  | Thank you. If you want one of these, try Bob's Axe Shop in Lumbridge, near the exit of this cellar. |
| 45 | 1279 | 1.1 |  | Thanks. |
| 46 | 1097 | 14.5 |  | That bit of damaged wall is blocked by rocks, not tree roots. I gave you a hatchet, so you should try chopping the other section of damaged wall. The bit that's blocked by tree roots. |
| 47 | 1046 | 14.6 |  | That bit of damaged wall is blocked by tree roots, not rocks. I gave you a pickaxe, so you should try mining the other section of damaged wall, the bit that's blocked by rocks. |
| 48 | 1281 | 4.7 |  | That's junk. Just drop it on the floor. I don't want it. |
| 49 | 1068 | 6.4 | s053 | The choice is yours. Would you like to mine through the rocks or chop through the roots? |
| 50 | 1049 | 8.5 |  | The dragon destroyed the ladder? That's a catastrophe. Why should never have tried rearing a dragon in here? |
| 51 | 1254 | 10.9 |  | Use the objective system to pick yourself an objective. When you've got one that you like, use my advisor button to ask me for advice. |
| 52 | 1085 | 7.3 | s052 | We need you to break through one of the damaged wall sections by clearing some rocks or tree roots. |
| 53 | 1261 | 6.6 |  | When you've got an objective that you like, use my advisor button to ask me for advice. |
| 54 | 1059 | 10.2 |  | Wherever you are, you can call on me for advice. I can teach you how to train your skills, how to make money, and much more besides. |
| 55 | 1105 | 12.9 |  | Wood cutting is used for chopping down trees. As your wood cutting level increases, you'll be able to cut down more variety of trees, getting increasingly valuable types of wood. |
| 56 | 1256 | 7.1 |  | You can leave the house now. Don't forget to use my advisor button to help you complete your objective. |
| 57 | 1043 | 6.9 |  | You won't be able to do that until you've trained your mining skill on the fallen rocks I showed you. |
| 58 | 1093 | 3.1 |  | You'll also be able to chop down trees much faster. |
| 59 | 1110 | 5.2 |  | You'll need a hatchet to chop through the roots. Come here and I'll give you one. |
| 60 | 1122 | 3.8 |  | You'll need a hatchet. Come and get one from me. |
| 61 | 1063 | 6.1 |  | You'll need a pickaxe to mine the rocks out of the way. Come here and I'll give you one. |
| 62 | 1112 | 3.9 |  | You'll need a pickaxe. Come and get one from me. |
| 63 | 1055 | 3.2 |  | You're in my cellar, below the town of Lumbridge. |
| 64 | 1255 | 8.8 |  | You're in the town of Lumbridge. South of us is Lumbridge Castle. The city of Varrock lies to the north. |
| 65 | 1061 | 9.1 | s076a | You're still carrying a few stone chippings. It's best to drop them on the floor. They'll just take up space in your inventory. |
| 66 | 1149 | 10.0 | s076 | You're still carrying a few wooden chippings. It's best to drop them on the floor. They'll just take up space in your inventory. |
| 67 | 1138 | 3.8 |  | You've already broken through the other section of damaged wall. |
| 68 | 1021 | 11.3 |  | You've mined a few stone chippings from that rock. The chippings are now in your inventory. Since they're not particularly useful, you should drop them on the floor. |
| 69 | 1081 | 11.2 |  | You've chopped off a few wood chippings from that root. The chippings are now in your inventory. Since they're not particularly useful, you should drop them on the floor. |
| 70 | 1020 | 7.2 |  | Could you possibly leave my pickaxe behind? Just drop it on the floor or hand it to me. |
| 71 | 3174 | 7.2 | s061a | So, your aim is to get a mining level of two. Okay, I'll find you something to train on. |
| 72 | 3172 | 7.2 | s062a | So, your aim is to get a wood-cutting level of two. Okay, I'll find you something to trade on. |
| 73 | 3176 | 12.0 | s066 | You'll mind a few stone chippings from that rock. The chippings are now in your inventory. They're not particularly useful, but you'll get more useful goods from mining once you're out of this cellar. |
| 74 | 3171 | 12.0 | s067 | You've chopped off a few wood chippings from that root. The chippings are now in your inventory. They're not particularly useful, but you'll get more useful goods from cutting wood once you're out of this cellar. |
| 75 | 3179 | 4.7 |  | You should be able to mine your way through the rubble blocking that left-hand doorway now. |
| 76 | 3175 | 4.7 |  | You should be able to chop your way through the roots blocking that right hand doorway now. |
| 77 | 3170 | 2.8 | s087a | Thank you once again for your assistance. |
| 78 | 3173 | 1.6 | s080 | I'll explain later, Jack. |
| 79 | 3177 | 16.3 |  | Once you're out and about, you can use the task system to suggest aims and endeavors to gopher. Everyone you do will get your reward. You'll always earn some money, sometimes additional goodies, and often progress towards a piece of armour with special effects. Look at the rewards tab for a task to learn more. |
| 80 | 3178 | 2.7 |  | Make your pardon, I need a quick word in you before you go. |
| 81 | 3180 | 1.7 | s079 | Good grief, where did you three come from? |

##### enum 3041 (18 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 1850 | 1.0 | s001 | Hello. |
| 2 | 1801 | 2.6 |  | Hello? Is anyone there? |
| 3 | 1874 | 3.1 |  | I could just walk through the gap now that the events moved away. |
| 4 | 1819 | 1.4 | s022a | I haven't searched it yet. |
| 5 | 1844 | 1.4 | s056 | I'll chop through the roots. |
| 6 | 1865 | 1.2 | s047 | I'll get on with it. |
| 7 | 1886 | 1.4 |  | our minds through the rocks. |
| 8 | 1839 | 1.3 | s026f | I'm getting there. |
| 9 | 1829 | 2.2 |  | I've placed the meat in the bowl like you said. |
| 10 | 1878 | 1.2 |  | Okay, I'll do that. |
| 11 | 1843 | 1.6 | s047 | OK, I'll get on with it. |
| 12 | 1816 | 6.5 |  | So I've got to climb over this wall, put this meat in the food bowl near the two wooden pillars and then tell you when I've finished. |
| 13 | 1851 | 6.5 |  | So I've got to get some meat from your bag. Climb over this wall and put this meat in the food bowl near the two wooden pillars. |
| 14 | 1817 | 3.9 | s014a | The wall is too high for me to climb over, so I can't get past you. |
| 15 | 1828 | 3.8 | s016a | It doesn't seem much point in messing around with this wall now that I've built a ramp. |
| 16 | 1803 | 1.6 |  | This seller's weird. |
| 17 | 1880 | 1.5 |  | What am I meant to be doing? |
| 18 | 1859 | 1.6 | s048 | Why don't you do it yourself? |

##### enum 3042 (18 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 373 | 1.0 | s001 | Hello! |
| 2 | 646 | 2.2 |  | Hello? Is anyone in there? |
| 3 | 656 | 3.5 |  | I could just walk through the gap now that Cervance moved away. |
| 4 | 640 | 1.7 | s022a | I haven't searched it yet. |
| 5 | 655 | 1.6 | s056 | I'll chop through the roots. |
| 6 | 430 | 1.5 |  | I'll get home with it. |
| 7 | 683 | 1.7 | s055 | I'll mine through the rocks. |
| 8 | 414 | 1.3 | s026f | I'm getting there. |
| 9 | 543 | 2.4 |  | I've placed the meat in the bowl like you said. |
| 10 | 675 | 1.7 |  | Okay, I'll do that. |
| 11 | 677 | 1.9 | s047 | Okay, I'll get on with it. |
| 12 | 576 | 7.6 |  | So, I've got to climb over this wall, put this meat in the food bowl near the two wooden pillars, and then tell you when I finished. |
| 13 | 670 | 6.9 |  | So, I've got to get some meat from your bag, climb over this wall and put this meat in the food bowl near the two wooden pillars. |
| 14 | 673 | 4.1 | s014a | The wall is too high for me to climb over, so I can't get past you. |
| 15 | 649 | 4.2 | s016a | There doesn't seem to be much point in messing around with this wall now that I've built a ramp. |
| 16 | 639 | 1.7 |  | The cell is weird. |
| 17 | 661 | 1.6 |  | What am I meant to be doing? |
| 18 | 578 | 1.7 | s048 | Why don't you do it yourself? |

##### enum 3043 (8 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 1215 | 2.4 |  | Ooh, shiny. |
| 2 | 1207 | 2.0 |  | Hahahaha! |
| 3 | 1197 | 1.6 |  | LOOT |
| 4 | 1178 | 2.3 |  | me get rich. |
| 5 | 1248 | 3.6 |  | Mine. All mine. |
| 6 | 1231 | 2.1 |  | Oooooooh. |
| 7 | 1234 | 0.6 |  | ROW! |
| 8 | 1891 | 3.1 |  | NOOOOOOO! |

##### enum 3044 (10 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 691 | 30.9 |  | Roddick's Diary, 3rd Ractuba. Yet again it's raining. Just when I wanted to go for a walk to Drainer Village with my gorgeous new walking-stick and maybe pop him for tea with a wise old man. How typical. Oh well, I'm sure I can find some way to pass the time. Maybe I'll leaf through some of those old books that belong to my late uncle. He had such strange interests. Saradomin rest his soul. |
| 2 | 693 | 24.4 |  | 4th Ractuba I believe I found a marvelous new hobby. One of those old books was a manual for rearing pet dragons. I don't have a garden, but I'm sure I can do just as well keeping it in my cellar. There's plenty of space down there, and it'll be out of the rain. I've written to the Taverly Pet Shop to order an egg and some supplies. |
| 3 | 697 | 16.5 |  | Fifth Raktuba. No sign of my egg yet, and it's still raining. Never mind. I shall brave the rain and take a walk after all. Sixth Raktuba. Oh dear. I have such a terrible cold. |
| 4 | 694 | 37.9 |  | 9th Raktuba At last I'm feeling well enough to resume my diary. Furthermore, my delivery has arrived far sooner than I had hoped. That postman is a curious little chap, but he certainly knows his trade. Now, to unpack the egg and get started. 11th Raktuba Apparently it takes ages for these eggs to hatch. How disappointing. I think I shall name it Sottlepot if it's a boy dragon and Pitypins if it's a girl dragon. |
| 5 | 700 | 22.9 |  | Fourteenth Raktuba, little subtle pot has hatched. These dragons really are marvellous creatures. Thirty-second Raktuba, I've had no time for this diary recently. I seem to find myself constantly busy preparing minced meat and bringing him water. He's growing so fast. |
| 6 | 696 | 28.5 |  | 15th PENTEMBA I must admit that I am starting to have second thoughts about this little project of mine. Sottlepot has grown so large that I am not entirely sure he will fit through the trap-door of the cellar any more. Furthermore, no matter how much I feed him, he keeps, well, salivating when he looks at me. He was a lot sweeter when he was smaller. I wonder if parenthood is like this. |
| 7 | 692 | 22.3 |  | 20th Pentember. I'm becoming seriously concerned about Sottlepot. He's completely smashed the pillars of my cellar. Fortunately, I was able to shore up the ceiling with some bits of wood before the whole cellar caved in. Hmm, perhaps I should have taken up carpentry instead of raising a dragon. |
| 8 | 698 | 28.3 |  | 21st Pentember. That blasted dragon almost managed to break the chains that were holding it to the wall. If he gets loose, there's no way I'm going to be able to restrain him again. At the risk of looking like a complete nincompoop, I think I'm going to have to call someone to sort him out before I lose control of him completely. The White Knights might be able to help. |
| 9 | 699 | 29.5 |  | Post-scriptum. Saved at last. The White Knight sent an agent in response to my message. He couldn't have arrived at a better time. I'd completely lost control of the dragon and it had me cornered in a cupboard in my own cellar. With the aid of a passing adventurer, he managed to kill the dragon, and then we tumbled out of the cellar into Jack's house because the blasted dragon had burnt my ladder. |
| 10 | 695 | 13.4 |  | I've agreed to act as an advisor for the adventurer, since they are unfamiliar with this country. Still, now all that excitement is over, I should try to find myself a new hobby. |

##### enum 3045 (4 lines)

| # | clip | s | canon | transcript (Whisper) |
|---|---|---|---|---|
| 1 | 1250 | 24.9 |  | On the care of dragons. Dragons are the noblest of creatures. While it requires considerable skill to keep them as pets, this simple guide will help you grasp the essentials. Firstly, you must ensure you have adequate space for the dragon. A large garden or menagerie is ideal. There should be some shelter from the elements and a plentiful supply of food and water. |
| 2 | 1251 | 33.9 |  | Once you are prepared, obtain a dragon's egg and place it in a warm, dry place. A dedicated incubator is preferred as the temperature of a cooking range is unlikely to be suitable. After a few days of this treatment, your egg will hatch. Do not leave your dragon outside except in warm weather until it has attained a size of approximately 12 inches. When it is ready to be introduced to its outdoor habitat, keep a close eye on its health to ensure it does not develop scale rot or lung damp. |
| 3 | 1249 | 32.6 |  | It is essential, especially in the first days, to keep the dragon well fed. In the wild, a mother dragon will typically spend most of her time feeding her brood. Dragons require copious quantities of raw meat. For the first weeks, you should mince this into a fine paste, having removed all bones. But once your dragon has attained a size of approximately 18 inches, it should be able to consume meat on the bone. You will rapidly learn what form of sustenance your dragon prefers. |
| 4 | 1252 | 10.8 |  | Dragon owners are strongly advised not to bring their dragons indoors once their flame ducts have developed, as this will typically lead to inconvenient house fires. |

### 3.7 When each voiced line plays (2026-11-09)

Every line of the voice enums (3.6), placed at its step of the quest and its trigger.

**Line codes.** Each line is `<code><key>`, with its clip in brackets:
- V = Sir Vant (3039), R = Roddeck / Explorer Jack (3040), Q = squire (3038);
- P = player (3041 / 3042, same keys, two voices), G = goblin (3043);
- N = message narrator (3037), U = guide narrator (3036).

**Steps** are the cache's own (task ids 3500-3523, varp 281 bands, 3.1).

**Evidence:**
- **R**: a recording shows the line there, with its 3.4 id. The best kind.
- **T**: the line's own text says when it plays.
- **G**: my guess from its place in the story. Treat it as unproven.

**Build:**
- **old**: only the older Objectives / advisor build uses it: the guide narrator U, Roddeck's
  advisor and objective lines, "So, Roddeck's offering to be an advisor". The 2011 Task build
  doesn't play these. Their topics are gone: no recording from late December 2010 on shows them,
  and their rewrites exist (R71-81).
- **task**: the Task build only (R71-81).
- Lines with no mark: both builds (seen in both, or no sign of either).

The 2011 port uses the Task build. "Old" lines are listed so the table is complete.

#### Start: Rotate Your Camera (3522), Talk to Sir Vant (3500); varp 281 5-9

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| P2 "Hello? Is anyone there?" [1801] | arriving in the cellar, before meeting anyone (inference) | G | |
| P16 "This cellar's weird." [1803] | an early look around (examining the cellar?) | G | |
| Q3 "Oh dear, oh dear, it's coming! You'd better talk to Sir Vant! He's in charge!" [1297] | talking to the squire before Sir Vant | R s001a | |
| U5 / U58 "Rotate the camera using the arrow keys until you can see the knight, Sir Vant..." [1355, 1441] | the opening guide (two-button / one-button mouse) | T | old |
| U51 "Talk to Sir Vant to find out what he wants." [1513] | the guide, after rotating | T | old |
| P1 "Hello." [1850] | first talk to Sir Vant | R s001 | |
| V84 "Wait a moment, there's a dragon coming!" [770] | Sir Vant's reply; the dragon cutscene follows | R s002 | |

#### Assist Sir Vant (3501): the dragon cutscene; varp 10

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| Q2 "Help!" [1284] | cutscene, far off (4KO2h2xPtdM @22) | R overhead | |
| Q4 "Uh oh." [1319] | cutscene, as the dragon turns on him | G | |
| Q1 (scream) [1292] | cutscene, the squire burns (4KO2h2xPtdM @35, "Aaargh!") | R overhead | |
| G1 "Ooh, shiny." [1215] | cutscene, the goblin at the bag (4KO2h2xPtdM @50) | R overhead | |
| V33 "I need some help over here." [882] | after the cutscene | R s003 | |
| V69 "Thank goodness you're here. My squire is dead, and there's a goblin going through my bag." [887] | talk | R s004 | |
| V31 "I can't turn my back on this dragon. I need you to take my squire's equipment and deal with the goblin." [807] | talk | R s005 | |
| V2 "Ahem, I'm over here." [1004] | trying the wall during this step (Kw7h2jY9ANQ @168-170) | R | |
| V28 "Hey, I'm over here!" [701] | the same kind of nudge, another action (which, unknown) | G | |
| U90 "We'll come back to that in a moment." [1502] | an action the step doesn't allow yet (examine menu, world map) | R s006 | |

#### Gather the Squire's Gear (3502), Equip Yourself (3503), Kill the Goblin (3504); varp 15-40

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V55 "My squire dropped his sword and armour when he died. You can pick them up and use them to defend my bag from the goblin." [811] | talking to Sir Vant before taking the gear | T | |
| V85 "Wait, you're not properly armed. You should equip all the kit you've picked up." [758] | attacking the goblin holding but not wearing the kit | T | |
| V86 "Wait! You're not properly armed! Pick up the kit dropped by my squire, then attack the goblin when you've equipped it!" [988] | attacking the goblin without the kit | R s005a | |
| V94 / V95 "When you've got the rest of my squire's kit equipped, you'll be ready to deal with the goblin." [788, 846] | talking with part of the kit on | T | |
| V12 "Excellent! You look ready to fight. You can deal with the goblin now." [978] | talking with all three worn | T | |
| V21 "Go on, finish off the goblin. It's too weak to be a real threat." [729] | talking during the fight | T | |
| G2-G8 "Hahahaha!", "LOOT", "me get rich.", "Mine. All mine.", "Oooooooh.", "ROW!", "NOOOOOOO!" [1207, 1197, 1178, 1248, 1231, 1234, 1891] | the goblin's overhead lines while it rummages (several seen: Kw7h2jY9ANQ @166-185, Qzq-h8kLBHk @101) and fights; "NOOOOOOO!" as it dies (inference) | R / G | |
| V109 "I don't believe I gave you permission to go through my bag." [739] | searching the bag before allowed | R s008 | |
| N8 "It would be rude to take that without permission." [1352] | taking something not yet allowed | T | |
| U12, U61, U62, U64-U70, U79-U81, U16, U22, U23, U26, U83, U103, U109-U116 | the guide's pick-up / equip / attack steps | T | old |
| N54 "You don't need to keep clicking..." [1460] | clicking the goblin again mid-fight (U103 is its guide twin) | T | |

#### Find Out Sir Vant's Plan (3505); varp 45-49

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V71 "Thank you. Now, to deal with the dragon. Come here and I'll tell you my plan." [730] | the goblin dies | R s007 | |
| V81, V61, V49, V47 [732, 943, 806, 726] | the plan, in that order | R s009-s012 | |
| V7 "Come behind the wall to talk to me please." [1008] | talking to him from the wrong side (inference: over the wall) | T | |
| V30 "I appreciate your enthusiasm, but could you please wait until I finish telling you what to do?" [836] | an action mid-plan | T | |
| V110 "I haven't finished explaining what you've got to do." [950] | as V30 | T | |
| V46 "If you don't know what you're doing, perhaps you should ask me." [940] | a wrong action with no plan heard | T | |
| P17 "What am I meant to be doing?" [1880] | a menu option when talking to him mid-quest | R s026b | |
| P12 / P13 "So I've got to [get some meat from your bag,] climb over this wall, put this meat in the food bowl..." [1816, 1851] | the player's recap reply (before / after taking meat) | T | |
| P10 "Okay, I'll do that." [1878] | the player's reply | G | |
| U54, U55 "Sir Vant is trying to explain his plan..." [1372, 1485] | the guide, if the player walks away | T | old |

#### Quest Journal (3506), World Map (3508), Realign Minimap (3509); varp 50-62

Only the guide speaks here (U14, U27, U105-U107, U130, U131, U34, U71, U82, U31): old build. The
Task build has the task panel and no voice. N52 "What would you like to say?" [1594] is the menu
title read aloud (G: old build).

#### Climb the Wall (3507), Make a Ramp (3523); varp 65-75

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| N26 "The bag contains several parcels of meat, all wrapped up in paper. You take one." [1608] | searching the bag | R s013 | |
| N49 / N51 "You don't have enough (inventory) space ... to pick up the meat." [1406, 1511] | bag, full inventory | T | |
| N50 "You won't need any more meat now." [1459] | bag, already holding meat | T | |
| V102 "You'll never get over the wall there. Try going to the eastern end where it's lower." [755] | climbing the high wall section | R s014 | |
| V26 "Hey, you'll land on the dragon if you climb over the wall there. Try going to the eastern end where the wall's lower." [1016] | climbing the section by the dragon | T | |
| V99 "Yes, go to the eastern side of the cellar on my right and climb over the wall there. I mustn't move..." [772] | asking him how to get over | T | |
| V80 "There must be some object in this cellar that could help you get over the wall. Talk to me again if you get stuck." [810] | asking him, no plank yet | T | |
| V38 "I think I saw a plank over by the eastern wall. Maybe you'll be able to use that." [796] | asking him again (stuck) | T | |
| V35 "I see you're holding a plank. You could use it against the wall to create a ramp." [859] | asking him, holding the plank | T | |
| P14 "The wall is too high for me to climb over, so I can't get past you." [1817] | the player, trying to pass Sir Vant | R s014a | |
| N30 "Even this low section of the wall is too high for you to climb over." [1769] | climbing the low wall without a ramp | R s015 | |
| N35 "There are definitely easier ways of getting past this wall." [1596] | another wall section (G) | T | |
| N34 "You use the plank to construct a crude ramp." [1401] | plank on the low wall | R s016 | |
| N32 "You won't need another one of those." [1764] | taking a second plank | T | |
| P15 "There doesn't seem much point in messing around with this wall now that I've built a ramp." [1828] | using something on the wall after the ramp | R s016a | |
| P3 "I could just walk through the gap now that Sir Vant's moved away." [1874; Whisper heard "the events" / "Cervance"] | trying the wall later, when he has left the doorway (G: after the sleep cutscene) | T | |
| V25 "Hey, don't walk in front of the dragon!" [880] | walking into the dragon's sight (the wiki: "you would automatically run away") | T | |
| V3 "Are you mad? Don't eat that, it's raw and it's covered in sleeping potion." [930] | eating the meat | R s018 | |
| V27 "Hey, don't just drop it on the floor. You're supposed to be placing it in a food bowl." [819] | dropping the meat | R s013a | |
| V11 "Don't stick the meat on the pillar. You're meant to put it in a bowl for the dragon to eat." [834] | meat on a pillar | T | |
| N48 "There's not much point in trying to feed that to the dragon." [1737] | an item on the dragon | T | |
| V10 "Don't give stuff to the dragon. It won't thank you, you know." [977] | an item on the dragon (Sir Vant's version) | T | |
| N31 "Well done. Now you just need to put the meat in the dragon's food bowl." [1439] | climbing the ramp with meat | T | |
| U1-U2, U8-U11, U28, U44, U46-U48, U60, U122-U125 | the guide's meat / wall / ramp steps | T | old |
| P9 "I've placed the meat in the bowl like you said." [1829] | the old build's "Yes, I've done it" reply (wiki transcript) | T | old |
| V64 "Put that drugged meat in the food bowl by the wooden pillars, please." [773] | talking to him before it's placed | T | |
| V20 "Get some more meat from my bag and put it in the food bowl by the wooden pillars, please." [966] | talking to him with no meat held or placed | T | |

#### Put the Dragon to Sleep (3510): bowl and cutscene; varp 80-90

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V52 "It's smelled the meat! Hide!" [715] | meat in the bowl | R s019 | |
| V23 "Grub's up! Shoo!" [850] | the sleep cutscene, at the wall (4KO2h2xPtdM @204) | R overhead | |
| V65 "Really? The dragon should have smelt it by now. Go back and search the bowl to make sure the meat's alright." [821] | talking to him before the cutscene runs (G: the old build's bowl check, U6 / U63) | T | old? |
| V106 / V107 "You've already lured the dragon to the right place. [We don't need to mess around with its food anymore.]" [992, 703] | meat or bowl again after the cutscene | T | |
| V34 "I really don't think you should do that." [1011] | attacking the dragon | R s019a | |
| V88 "Well done. Come out of hiding and I'll tell you what we're doing next." [958] | the cutscene ends | R s020 | |

#### Talk to Sir Vant Again (3511), Sabotage! (3512): pillars; varp 93-115

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V87 "We need to destroy these pillars to collapse the ceiling on the dragon." [911] | talk | R s021 | |
| V91 "We're going to burn down these wooden pillars to collapse the ceiling on the dragon." [724] | the same, other wording (G: old build) | T | old? |
| V8 "Could you get the rest of the supplies from my bag, please?" [993] | talk | R s022 | |
| P4 "I haven't searched it yet." [1819] | reply | R s022a | |
| V60 "Oh? Well, search my bag and bring the supplies to me before the dragon wakes up." [932] | reply | R s022b | |
| V24 "Have you got the stuff from my bag?" [870] | talking before searching | T | |
| N28 "You search Sir Vant's bag, and take a selection of supplies." [1488] | the bag | R s023 | |
| N25 "The bag contains various supplies." [1550] | the bag, examined / first look (old build: 7_neJ6mCLXc) | R s023a | |
| N29 "You search Sir Vant's bag. It's full of miscellaneous supplies." [1451] | the bag, other wording | T | |
| N27 "There's nothing useful left in the bag except for Sir Vant's spare underpants, which you leave well alone." [1739] | the bag, supplies taken | T | |
| V58, V14, V98 [795, 876, 844] | the plan | R s024-s026 | |
| V29 "How are you getting on?" [757] | talking mid-task | R s026a | |
| P8 "I'm getting there." / P17 "What am I meant to be doing?" | menu replies | R s026b, s026f | |
| V54 "Me? Touch oily rags? ... No, I'll let you have all the fun." [907] | "Why don't you do it yourself?" here | R s026c/d | |
| V15-V19 "First you need to wrap both pillars in oily rags..." (five, by progress: none / ripped / oily / ...) [754, 938, 891, 800, 728] | "What am I meant to be doing?", by what the player has made | T | |
| V13 "Finally attach a fuse to each pillar..." [820] | the same, rags on, fuses not | T | |
| V37 "I see you've got both pillars nicely wrapped in oily rags now." [776] | the same, rags on | T | |
| V108 "You've got the fuses sorted out too. Great. Now just hand me one of those tinderboxes from my bag." [939] | the same, fuses on | T | |
| V36 "I see you finished preparing the pillars. Could you hand me one of the tinderboxes from my bag please?" [919] | the same, pillars done | T | |
| N10-N16, N21-N23 | rip, soak, wrap, fuse messages (N10 s027, N11 s030, N12 s029, N13 s028, N14/N15 s031, N22 s032, N23 s033) | R / T | |
| V103 "You're supposed to rip the cloth in half first." [791] | using the whole cloth on a pillar | T | |
| V105 "You're supposed to soak the rag in oil first. Try using the rag with the oil can." [750] | a dry rag on a pillar | R s031a | |
| V104 "You're supposed to rip the cloth into two rags, then soak the rags in oil before sticking them to the pillar." [852] | the cloth on a pillar (other wording) | T | |
| V41 "I think you should wrap the oily rag around the pillar before adding the fuse..." [991] | a fuse before the rag | T | |
| V77 / N21 "That pillar's already got an oil-soaked cloth around it." [786, 1773] | a second rag on one pillar | T | |
| V79 "The pillar's already fully prepared for burning." [905] | more on a finished pillar | T | |
| N16 "There's no need to put oil on the fuse." [1770] | the oil can on a fuse | T | |
| V83 "Wait a minute! You're supposed to get both pillars prepared before we light them!" [833] | the tinderbox on a pillar too soon | R s033a | |
| V101 "You'll need to give me a tinderbox first..." [913] | the tinderbox on a pillar before giving him one | R s034a | |
| N20 "Perhaps you should wait until Sir Vant is ready." [1332] | lighting before he's in place (G) | T | |
| V73 "Thanks, that's perfect. Now hand me a tinderbox." [931] | talking, pillars done | R s034 | |
| V53 "Just hand me one of those tinderboxes from my bag." [860] | talking again, no tinderbox given | R s035a | |
| N53 "You offer Sir Vant a tinderbox." [1556] | tinderbox on Sir Vant | R s035 | |
| V72 "Thank you. When you're ready, set fire to the other pillar..." [893] | after the tinderbox | R s036 | |
| V39 / V82 "I think it would be better if you lit the other pillar..." / "Use one of the tinderboxes from my bag to set light to the other pillar..." [799, 792] | lighting his pillar / talking without a tinderbox | T | |
| V32 "I don't think I need that just at the moment, thank you." [798] | other items on Sir Vant | R s033b | |
| V6 "Big boom coming up!" [998] | talking after both are lit (G) | T | |
| V51 "It's burning. Run!" [881] | lighting the other pillar | R s037 | |
| U57 "Run run run!" [1656] | the guide, the same moment | T | old |
| V97 "Wow!" [782] | after the collapse (4KO2h2xPtdM @413) | R overhead | |
| N18 "The pillar has been reduced to a stump..." [1683] | a pillar after the collapse | T | |

#### Return the Squire's Gear (3513); Roddeck arrives; varp 120-127

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V50 "It worked! I see the owner of this cellar has emerged. I think he owes you a word of thanks." [709] | talking to Sir Vant before Roddeck | T | |
| V1, R43, V56 [763, 1102, 785] | Roddeck's arrival talk | R s038-s040 | |
| R29, R54, R27, V68, R3, U45, U78, U86, U91-U94, U98-U100 | advisor introduction and screen | T | old |
| V5 "Before we go any further, could I have my squire's kit back, please?..." [705] | talk | R s041 | |
| V4 "I'd like my squire's kit back... Take it off and hand it to me, please..." [3169] | the same, still wearing it (newer clip id) | R s041c | |
| V63 "Please hand over my squire's kit." [920] | talking again, kit not given | R s042 | |
| V76 "Thanks. Keep 'em coming, please." [744] | each piece handed over | R s041a | |
| V74 / V75 / V44 / V43 / V45 "(Thanks.) I'd like my oil can / tinderbox (and oil can) back too..." [838, 765, 778, 840, 780] | kit returned, his tools not | T | |
| V9 "Could you possibly leave my squire's kit behind? ... Just drop it on the floor, or hand it to me." [890] | trying to leave with it (the ladder, later) | T | |
| V57 "No thanks. Drop it on the floor if you don't need it." [908] | handing him something else | T | |
| R22 "I think you should be returning that item to Sir Vant, not to me." [1027] | kit items on Roddeck | R s041b | |
| R19 "I suppose you'd better return Sir Vant's stuff to him..." [1030] | asking Roddeck about armour (old: advisor topic) | T | old? |
| V62 "Perhaps you shouldn't throw that away." [723] | dropping a tutorial item (the 2011 wiki item pages' destroy text) | T | |
| N9 "You won't be needing that again." [1558] | picking a dropped quest item back up (wiki: Roddeck's diary) | T | |

#### Discover an Exit (3514), Cutting a Path / Breaking Through (3515 / 3516); varp 128-165

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V66, V93, V48 [861, 935, 837] | Sir Vant on the way out | R s043-s045 | |
| V78 "The dragon destroyed the cellar ladder, so we'll need a new way out." [737] | the burned ladder | T | |
| R50 "The dragon destroyed the ladder? That's a catastrophe..." [1049] | Roddeck, the same moment | T | |
| P6 / P11 "(OK,) I'll get on with it." [1865, 1843] | menu reply | R s047 | |
| P18 "Why don't you do it yourself?" [1859] | menu reply | R s048 | |
| V89 "Well, I would, but my armour's terribly hard to clean when it gets dirty." [922] | Sir Vant's answer | R s048a | |
| V67 "See if you can dig us out of this cellar. Roddeck will advise you on what to do. I'll just stand here and watch." [745] | talking to Sir Vant later in this step | T | |
| V40 "I think we've all been in this cellar long enough..." [720] | Sir Vant, later still | R s075b | |
| R52, R49 [1085, 1068] | Roddeck's choice | R s052-s053 | |
| P5 / P7 "I'll chop through the roots." / "I'll mine through the rocks." [1844, 1886] | the choice | R s055-s056 | |
| N74-N77 "Roddeck gives you (another) pickaxe / hatchet." | the tool | R s057-s058 | |
| R34 / R35 "Now you've got a hatchet / pickaxe, ..." [1031, 1037] | talking, tool held (old build: kjgw48iBdc0) | R s058a | old? |
| R59-R62 "You'll need a hatchet / pickaxe..." [1110, 1122, 1063, 1112] | the blockage without the tool | T | |
| R36 / R37 "Oh dear, you seem to have lost the hatchet / pickaxe I gave you." [1071, 1091] | talking, tool lost | T | |
| N59 / N62 "You attempt to mine / chop through the ..." | the blockage | R s059-s060 | |
| N60 / N63 "You'll need a Mining level of 2 ... / You find that you'll need a Woodcutting level of 2 ..." | the blockage at level 1 | R s061-s062 | |
| R46 / R47 "That bit of damaged wall is blocked by rocks / tree roots, not ..." [1097, 1046] | the wrong blockage | T | |
| R23 / R24 "I thought you were meant to be learning mining / woodcutting, not ..." [1023, 1022] | the other skill's practice spot | T | |
| R67 "You've already broken through the other section of damaged wall." [1138] | the other blockage after one is cleared | T | |
| R71 / R72 "So, your aim is to get a Mining / Woodcutting level of 2?..." [3174, 3172] | talking after the level message | R s061a, s062a | task |
| R41 / R42 "So, your objective is ..." [1145, 1133] | the same, old build | T | old |
| R2 / R1 "Ah, I have something! You can practice mining on the rocks / chopping those roots..." [1067, 1101] | next | R s061b, s062b | |
| R31 / R30 "Keep mining those rocks / chopping those roots until your level reaches 2." [1028, 1148] | talking while training | T | |
| R57 "You won't be able to do that until you've trained your mining skill on the fallen rocks I showed you." [1043] | the blockage again, still level 1 | T | |
| N2-N7, N1 | swing / break-off messages; inventory full of wood (N1) | R s063-s065a / T | |
| R73 / R74 "You've mined / chopped a few chippings..." [3176, 3171] | the first chippings | R s066, s067 | task |
| R68 / R69 the same, older wording [1021, 1081] | the same | T | old |
| R65 / R66 "You're still carrying a few stone / wooden chippings..." [1061, 1149] | chippings held at the exit (G) | R s076a, s076 | |
| R7 / R6 "Congratulations! You've reached level 2..." [1074, 1142] | level 2 | R s070, s071 | |
| R75 / R76 "You should be able to mine / chop your way through the rubble / roots blocking that left / right-hand doorway now." [3179, 3175] | talking at level 2 | T | task |
| U17, U21 "Your Woodcutting / Mining level is now high enough..." [1521, 1376] | the same, guide | R (kjgw48iBdc0) | old |
| N61 / N64 "You succeed in clearing the rocks / roots from the hole." | the blockage at level 2 | R s072, s073 | |
| N69 / N73 "Roddeck retrieves his pickaxe / hatchet from you." (N66-N72: with his diary / book) | the hole cleared | R s074, s075 | |
| R70 / R10 / R26 / R25 "Could you possibly leave my pickaxe / hatchet behind? ..." / "I'd like my pickaxe / hatchet back too..." [1020, 1100, 1116, 1087] | leaving with his tool / book | T | |
| V59 "Ah, well done. Climb through the hole, and we can all get out of this cellar." [947] | Sir Vant, the hole cleared | R s075a | |
| R15 "Great! We can climb through the hole in the wall now. Thank you." [1065] | Roddeck, the same | T | |
| R4, R33, R55, R58 | "What is mining / woodcutting for?" answers | T | old |
| R39 / R40 "Perhaps you should concentrate on the objective you've already got..." | old advisor | T | old |
| R8 / R9 "Could you perhaps come and (have a) chat with me before doing that?" [1131, 1141] | acting before talking to Roddeck | T | |
| R12 "Excuse me, I hadn't finished talking to you." [1260] | walking off mid-talk | T | |
| R28 "I'll take that if you don't mind." [1280] / R45 "Thanks." [1279] / R48 "That's junk..." [1281] | items handed to Roddeck | T | |
| R44 "Thank you. If you want one of these, try Bob's Axe Shop in Lumbridge..." [1272] | his tool handed back (G) | T | |
| R38 "Once we're out of this cellar, I'll be able to advise you about where to get useful items like those." [1079] | asking him about items (old: kjgw48iBdc0) | R | old |
| R16 "I don't think I need that at the moment. Thank you." [1047] | other items on Roddeck | T | |
| R63 "You're in my cellar, below the town of Lumbridge." [1055] | P "Where am I?" | T | old |
| R17 "I love advising people. It makes me feel important." [1035] | old advisor | T | old |
| R32 "Let's come back to that once we've got out of this cellar." [1054] | old advisor, an off-topic question | T | old |
| Books: N42-N47 shelf messages; U-less readings: the diary (3044 1-10) and the manual (3045 1-4) | reading Roddeck's diary / the manual | T | |
| N37-N41 the cabbage barrel and eating | the barrel in Roddeck's room | R s050-s051 | |
| N36 "The door won't open. It feels like someone's holding it closed from inside." [1413] | the cupboard door before Roddeck comes out | T | |
| N19 "This doesn't seem like a good time to mess with it." [1641] | a cellar loc out of turn (G) | T | |
| N65 "There doesn't seem much point in going back in there." [1754] | the hole back (G) | T | |

#### To Freedom! (3519), Lumbridge (3520, 3521); varp 170-1000

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V70 "Thank you again for your help with the dragon. You'll receive a little reward..." [704] | talking to Sir Vant before the ladder | R s078 | |
| V92 "We've nearly finished the quest now. Once you climb that ladder, you'll be free to start exploring the world outside this cellar." [804] | the same, later | R s078b | |
| R20 "Hmm, I think this ladder leads to the house of my neighbour, Explorer Jack..." [1094] | Roddeck, at the ladder | R s078a | |
| N55 / N56 / N57 "Congratulations, quest complete." / "You have completed Unstable Foundations. You gain one quest point and level four mining / woodcutting." [1692, 1514, 1579] | the quest scroll on arrival (4KO2h2xPtdM @607) | R | |
| N79 / N80 "You've just advanced a Mining / Woodcutting level..." [1560, 1638] | the reward's level-up | R | |
| R81 "Good grief, where did you three come from?" [3180] | Explorer Jack, on arrival | R s079 | task |
| R78 "I'll explain later, Jack." [3173] | Roddeck | R s080 | task |
| R5 "At last we're out. Now Sir Vant has accomplished his mission..." [1282] | Roddeck (the other recorded opening) | R s081 | |
| V90 "Well, my mission is accomplished now. Roddeck's dragon is no longer a threat." [688] | Sir Vant | R s082 | |
| V42 "I'd better head back to Falador..." [687] | Sir Vant | R s083 | |
| R14 "Goodbye, Sir Vant, and thank you again." [1253] | Roddeck | R s084 | |
| R80 "Begging your pardon, I need a quick word with you before you go." [3178] | Explorer Jack, if the player heads for the door (G) | T | task |
| R79 "Once you're out and about, you can use the Task System... You'll always earn some money... Look at the Rewards tab for a Task to learn more." [3177] | Explorer Jack: one clip for both boxes | R s085, s086 | task |
| R77 "Thank you once again for your assistance." [3170] | Roddeck, talked to again after | R s087a | task |
| N58 "As you leave, Roddeck wanders off home." [1653] | the scene ends | R s087 | |
| R18, R64, R51, R53, R56, R13, R11, U36, U49, U59, U101, U29 "You may now leave via the door." | the old build's exit: Roddeck's Lumbridge advice and the objective screen | T | old |

#### Any step

| Line | Trigger | Ev. | Build |
|---|---|---|---|
| V100 "You can leave that alone now." [706] | an item or loc already used (G) | T | |
| V96 "Wonderful!" [845] | a short reply (G) | G | |

Recorded box lines (3.4) with no clip of their own:
- s026d and s086 share a clip with the line before them (V54; R79 [3177]).
- s013b ("You should take some meat from Sir Vant's bag before you venture out.") and s065 / s065m
  ("...gaining 42 ... experience. Once more should suffice.") are Task-build wordings with no
  clip. The cache voices only the older "gain a little ... experience" lines (N3, N6), so these
  were silent in 2011 (inference).
- s033c "Nothing interesting happens." and s068 / s069 "Congratulations: You have reached
  level 2 ..." are the engine's generic boxes, unvoiced.
- Menus (s046, s054) have none. The hint boxes s017 and s077 have clips in the guide's enum
  (U104, U108); whether the Task build played them: unknown.

---

### 3.8 The arrival, read off the recordings and matched in our client (2026-11-09)

**Where Sir Vant and his squire stand.**
- Method: our client's minimap, north-up at local (30, 9), is the reference. The recordings-matrix
  skill's `minimap_match.py` fits 4KO2h2xPtdM's minimap onto it: @8.5, score 0.815, scale 0.68,
  turned 328 degrees, shift (0, 0). So 2011's player stands on (30, 9) exactly.
- `minimap_dots.py` then reads the yellow npc dots in tiles from the player's dot.
- As "Hello!" opens (@12.38) the player walks up and stops on (32, 15). The npcs are one tile
  north and one tile east of that.
- The 3D view @13.5 shows the knight in the gap of the start room's north wall line, and the
  squire south of it.
- Result: **Sir Vant on (32, 16)**, the doorway where loc 3550 inviswall_serverside sits, and
  **the squire on (33, 15)**.
- VwpdbjfigD4 @80 (north-up) agrees. Both stand still: the same tiles at 8.5 s and 80 s.

**Which Sir Vant.**
- Each of cache 786's six (8848-8853) was placed in our client on his tile and set beside
  4KO2h2xPtdM @9-14.
- 2011's knight holds his orange shield up in front, feet set wide: that is **8850**
  (bas_1614).
- 8848 and 8849 carry only a sword, 8851 holds the sword across, 8852 holds it raised, and 8853
  holds the shield flat.

**The side panel.** 4KO2h2xPtdM's tab rows were read every 10 s, and every second around the
gear:
- arrival to "Gather the Squire's Gear": only the task system and options tabs;
- the gear picked up ("Equip Yourself", @79): inventory, with a yellow arrow at it;
- the gear worn (@82-83): combat and worn equipment;
- the quest journal step: quests;
- the first practice xp: stats;
- arrival in Lumbridge: prayer and magic, and friends, ignore, clan, emotes, music and notes.

**The first seconds.** 8jKH_7To46o, 2011-05-11, @56-66, one frame a second:
- **@56:**
  - The game view shows the **Controls overlay**: cache 897 tutorial3_cameracontrols; 1079 is a
    copy of it.
  - The side panel shows **options**, with a **yellow arrow** pointing at the task system tab.
  - The arrow is varbit 3756, the side tab to flash plus 1, which clientscripts 117 and 2756
    draw. The client drops it while that tab is the one showing.
- **@58:** the player opens the task system tab: "Rotate Your Camera" (Task 3522).
- **@63:** the camera has turned. The panel goes on to "Talk to Sir Vant" and the overlay closes.
- **@64-66:** the task system's popup (interface 1055, woken by varc 1425) shows "TASK
  COMPLETE!" and "Rotate Your Camera".
- The chat box stays empty: no "You have completed the Task" lines for the tutorial's Tasks.

What the overlay does:
- Its listener binds the four arrow keys to ops 1-4 and transmits them.
- Its key lights wait for varp 281 to reach 6 (cs2 2773), so the overlay's stage is 6.
- Quest config 131 gives 5 at creation.

**Matched in our client** (local build, waiting on the rest of the tutorial):
- character creation (Select Character, Select Style) into the cellar on (30, 9);
- Sir Vant 8850 and the squire on their tiles;
- the task system and options tabs only, options open, the arrow at the task tab;
- the Controls overlay;
- an arrow key closes the overlay and slides in TASK COMPLETE! for Rotate Your Camera, frame for
  frame as 8jKH_7To46o @56-66.

**Still open:**
- 2011's arrival camera sits lower and closer.
- 2011's minimap shows only the start room and a strip north of it, where ours also shows the
  room to its west. Cache 786 places a second start room around (56..60, 3..9). Whether 2011
  started players there: to be fitted.

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
| 0 | Engine decision on instancing (8.2 items 2-3): **done**, Jordan's regions and controllers (337f6e28e0: `region_create`, `region_copyarea`, `region_getcoord`, `region_del`, `controller_*`) | `_test/scripts/engine/region_tests.rs2` | |
| 1 | varp 281 as UF's: character creation ends at UF's first stage, not 1001; UF's end sets 1000; varbit 4895 = 3 at creation (inference) | testscript: after `character_created`, varp281 = 5/6 (value 1-6 meaning: **gap**); at UF end = 1000; quest list cs2 2193 reads done | values 1-6, the exact first value |
| 2 | UF configs: npc/loc/obj names for the ids in section 2; multiloc state helpers (varbits 6487, 6495, 6496, 6497) | runec lint; CRC gate if packs change | |
| 3 | Start: place the player in the cellar (instance or static), open 1079/897 controls, Task tab; arrow-key ops -> varp281 7 | harness: login of a new account lands in the cellar, IF_BUTTON on 1079:0 sets 7 | start tile: local (30, 9) (3.5 #1); overlay slot and whether the arrow-key ops are transmitted: **gap** (client test) |
| 4 | Squire's gear and goblin: pick up gear (15), equip (20-30), goblin fight rules (35-40, wiki Goblin page) | harness: equip -> 35; goblin only attackable in full gear | lines: 3.4; gear: bronze platebody 1117, sq shield 1173, sword 1277 on (32, 13) (3.5 #3) |
| 5 | Quest journal, world map, minimap Tasks (50-62) | testscript per threshold | how the server detects world map open / compass click: **gap** |
| 6 | Plank, ramp (loc 45604 >= 75), low wall climb (65-75) | harness: use plank on wall -> 70, climb -> 75 | the plank is ground obj 15276 on (35, 15), not a loc (3.5 #2) |
| 7 | Food bowl (varbit 6487), dragon asleep (80-90) + cutscene varcs 105/201-203 | harness: use meat on bowl -> 80+ | shot-by-shot timelines in 3.5 #4; which varc 1007 value draws each shot: client test |
| 8 | Sabotage: rip/oil/fuse/pillars (varbits 6496/6497), tinderbox to Sir Vant, light, collapse (93-115) | harness per item-on-loc | pillar state order (0-4 meaning): inference only |
| 9 | Return gear (120-127) | harness | |
| 10 | Roddeck and the exit choice (varbit 6495), practice rocks/roots 42 xp, blockage, hole (128-170) | harness: chop/mine to level 2, clear, climb through | Roddeck's lines: **gap** |
| 11 | Ladder 45784 -> Lumbridge, varp281 = 1000, Explorer Jack Task (varbit 6494 = 3) | harness: climb -> Lumbridge, 1000 | arrival (3207, 3240) with the quest scroll (3.5 #7); Jack's lines in 3.4 |
| 12 | Music (Destiny, The Adventurer, The Mentor, Harmony) | testscript: midi per stage | Destiny in creation, The Mentor in the cellar, Harmony on arrival (3.5 #6) |

**Dialogue (all stages):** section 3.4 gives every line, with its first sighting and the
branches, from 32 dated 2010-11 recordings. Recordings outrank the wiki for dialogue text (3.4's
notes). A line no recording shows stays out, or keeps a `// no source found` stub.
