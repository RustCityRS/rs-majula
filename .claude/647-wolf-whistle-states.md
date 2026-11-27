# Wolf Whistle (2008 version, as of May 2011): states decoded from dated recordings

Work folder: `target/refs/wolf_whistle/`. 17 usable recordings (N=17), upload dates 2009-09-23 to 2011-10-30;
1 dropped (`excluded.tsv`: zi8B6nf3zRQ is a private server). Ids used below (short form = first 4 characters):

| id | upload | size | notes |
|---|---|---|---|
| 0NxpQ6TqrTo | 2009-09-23 | 1152x720 (game is small inside a browser capture) | full run |
| HemCMqn3Y3E | 2009-11-20 | 1280x720 | full run, sharpest text; has journal, locked trapdoor |
| lGcCWqJlpSo | 2010-01-01 | 640x480 | full run; only one with the "Wait. Why..." branch; body search |
| EwNMHpeFCRk | 2010-03-27 | 1280x720 (small game) | full run, journal |
| s4JFOcanjlM | 2010-03-31 | 640x480 | blurry, almost nothing readable |
| f5AzI4WjuVc | 2010-04-24 | 1280x720 | body search, cutscenes |
| EADAootOI24 | 2010-07-09 | 640x480 | full run, blurry |
| ldSYrbNZbI8 | 2010-07-31 | 728x480 | full run |
| oqiSERsUHZU | 2010-08-25 | 852x480 | full run |
| zGpeA1CrlVg | 2010-11-28 | 600x480 | 2-minute cut; body search only |
| 9uujQ6UkUIQ | 2010-12-19 | 640x480 | later half only |
| QlQ_p-MBxJ8 | 2011-02-09 | 960x720 | full run |
| ROB_P-ogto4 | 2011-03-01 | 508x360 | full run, slow clicks; body search WITH bones already held |
| 2EfU5Mrh8Kc | 2011-05-24 | 626x480 | full run, new account (Task List button on screen) |
| JQZnlZk0KqE | 2011-05-24 | 650x480 | full run, clearest 2011 cutscenes |
| RKNUR6gsrwc | 2011-08-20 | 640x480 | 2-minute cut, no dialogue |
| WKPggURQByY | 2011-10-30 | 544x360 | most of the run; body search |

**How the text was read.** Dialogue boxes were cut from every frame, OCR'd (Windows OCR) and matched to the canon
(`canon.tsv`, `out2/matrix.md`). OCR of the RuneScape fonts is poor, so **every line in the table below was read by eye**
from tiled crops: the main story from HemCMqn3Y3E (720p, 2009) and ROB_P-ogto4 (2011), then compared against the OCR of
the other 2010-11 recordings to find differences. OCR alone was used only to count which recordings show a line and at
which second. Lines confirmed by eye in a 2011 recording as well as 2009 are the default; the exceptions are listed
under "2009 versus 2011".

Line breaks: `⏎` marks the line breaks seen on screen. Almost all are ordinary wraps. **Forced breaks I can vouch
for**: the three "Mustn't look round..." lines (c2_01), and the journal's last line. I did not run `br_check.py`
(no font metrics were handed to me), so other breaks are "as displayed", not proven forced.

Chathead expressions: the faces are visible but I cannot name animation ids from footage. Where it is obvious it is
noted in section 2 (frightened/wide-eyed during the attic lines, eyebrows-down angry Pikkupstix on a35/b02, dizzy
head-roll on g10). Treat expressions as **not sourced**.

## 2009 versus 2011 text differences (eye-confirmed in both)

- **b07**: 2009 (0Nxp @193, HemC @72): "...but are a little ⏎ less violent on the spiritual plane."
  2011 (QlQ_ @137, ROB_ @228, 2EfU @112, JQZn @162): "...less violent on the spirit plane."
- **f08**: 2009 (0Nxp @271, HemC @183): "scared that it will retreat to its spiritual plane, rather ⏎ than face the spirit wolf."
  2011 (QlQ_ @252, ROB_ @442, 2EfU @197, JQZn @368): "scared that it will retreat to its spirit plane, rather than ⏎ face the spirit wolf."
- **Level-up chat line**: 2009 (HemC @268) "You've just advanced three Summoning levels! You have reached level 4."
  2011 (2EfU @353, JQZn @528) "You've just advanced 3 Summoning levels! You have reached level 4."
  (generic level-up text, not quest text; depends on the player's level).
- Level-up window: 2009-10 is the chatbox flash; 2011 is the big parchment "Congratulations!" window (section 7).
- No other differences found in quest dialogue between 2009 and 2011.

## 1. Dialogue, in order

Columns: id, kind, speaker, text, recordings showing it (of 17), sources `videoid @second` (2011 first).

| id | kind | speaker | text | seen | sources |
|---|---|---|---|---|---|
| m_topic | menu |  | Select an Option: Talk about Wolf Whistle quest. ⏎ Talk about Pikkenmix. | 11/17 | QlQ_p-MBxJ8 @64, ROB_P-ogto4 @29, 2EfU5Mrh8Kc @17 |
| a01 | chatnpc | Pikkupstix | You there! What are you doing here, as if I didn't have ⏎ enough troubles? | 13/17 | QlQ_p-MBxJ8 @67, ROB_P-ogto4 @32, 2EfU5Mrh8Kc @20 |
| a02 | chatplayer | PLAYER | I'm just a passing adventurer. Who are you? | 12/17 | QlQ_p-MBxJ8 @68, ROB_P-ogto4 @40, 2EfU5Mrh8Kc @21 |
| a03 | chatnpc | Pikkupstix | Who am I? My dear boy, I just so happen to be one ⏎ of the most important druids in this village! | 11/17 | QlQ_p-MBxJ8 @68, ROB_P-ogto4 @42, 2EfU5Mrh8Kc @23 |
| a04 | chatplayer | PLAYER | Wow, I don't meet that many celebrities. | 11/17 | QlQ_p-MBxJ8 @69, ROB_P-ogto4 @46, 2EfU5Mrh8Kc @24 |
| a05 | chatplayer | PLAYER | What do you do here? | 11/17 | QlQ_p-MBxJ8 @70, ROB_P-ogto4 @49, 2EfU5Mrh8Kc @25 |
| a06 | chatnpc | Pikkupstix | Well, firstly, I- | 9/17 | QlQ_p-MBxJ8 @71, ROB_P-ogto4 @51, 2EfU5Mrh8Kc @27 |
| a07 | mesbox |  | There is a loud crash from upstairs, followed by the tinkling of ⏎ broken glass. | 11/17 | QlQ_p-MBxJ8 @71, ROB_P-ogto4 @53, 2EfU5Mrh8Kc @28 |
| a08 | chatnpc | Pikkupstix | Confound you, lapine menace! | 11/17 | QlQ_p-MBxJ8 @72, ROB_P-ogto4 @58, 2EfU5Mrh8Kc @28 |
| a09 | chatplayer | PLAYER | What was that? | 11/17 | QlQ_p-MBxJ8 @73, ROB_P-ogto4 @61, 2EfU5Mrh8Kc @29 |
| a10 | chatnpc | Pikkupstix | That was the sound of my carefully arranged room ⏎ being destroyed by a fluffy typhoon of wickedness! | 12/17 | QlQ_p-MBxJ8 @74, ROB_P-ogto4 @62, 2EfU5Mrh8Kc @30 |
| a11 | chatplayer | PLAYER | What? | 9/17 | QlQ_p-MBxJ8 @74, ROB_P-ogto4 @64, 2EfU5Mrh8Kc @31 |
| a12 | chatplayer | PLAYER | Wait, I'm good with monsters. Do you want me to go ⏎ kill it for you? | 11/17 | QlQ_p-MBxJ8 @75, ROB_P-ogto4 @65, 2EfU5Mrh8Kc @33 |
| a13 | chatnpc | Pikkupstix | Well, yes, but it's a little more complicated than that. | 11/17 | QlQ_p-MBxJ8 @75, ROB_P-ogto4 @70, 2EfU5Mrh8Kc @34 |
| a14 | chatplayer | PLAYER | How complicated can it be? | 10/17 | QlQ_p-MBxJ8 @76, ROB_P-ogto4 @72, 2EfU5Mrh8Kc @36 |
| a15 | mesbox |  | A resounding crash comes from upstairs. | 10/17 | QlQ_p-MBxJ8 @77, ROB_P-ogto4 @73, 2EfU5Mrh8Kc @37 |
| a16 | chatnpc | Pikkupstix | Complicated enough for me to waste time explaining it ⏎ to you. | 8/17 | QlQ_p-MBxJ8 @77, ROB_P-ogto4 @77, 2EfU5Mrh8Kc @39 |
| a17 | chatnpc | Pikkupstix | Tell me, have you ever heard of Summoning? | 9/17 | QlQ_p-MBxJ8 @78, ROB_P-ogto4 @82, 2EfU5Mrh8Kc @40 |
| a18 | chatplayer | PLAYER | Not really. | 10/17 | QlQ_p-MBxJ8 @79, ROB_P-ogto4 @85, 2EfU5Mrh8Kc @41 |
| a19 | chatnpc | Pikkupstix | Well, some of the concepts I am about to discuss might ⏎ go over your head, so I'll stick to the basics. | 10/17 | QlQ_p-MBxJ8 @80, ROB_P-ogto4 @86, 2EfU5Mrh8Kc @42 |
| a20 | mesbox |  | There is a series of loud crunches from upstairs. | 10/17 | QlQ_p-MBxJ8 @80, ROB_P-ogto4 @91, 2EfU5Mrh8Kc @43 |
| a21 | chatnpc | Pikkupstix | Gah! That better not be my wardrobe! | 10/17 | QlQ_p-MBxJ8 @81, ROB_P-ogto4 @94, 2EfU5Mrh8Kc @44 |
| a22 | chatnpc | Pikkupstix | Summoners are able to call upon animal familiars for a ⏎ number of uses. These familiars can help the summoner ⏎ practice their skills, fight on the battlefield, or offer any ⏎ number of other benefits. | 10/17 | QlQ_p-MBxJ8 @82, ROB_P-ogto4 @97, 2EfU5Mrh8Kc @46 |
| a23 | chatnpc | Pikkupstix | Beneath this house is a monument - an obelisk - that ⏎ allows us druids to tap into this Summoning power. | 10/17 | QlQ_p-MBxJ8 @82, ROB_P-ogto4 @106, 2EfU5Mrh8Kc @48 |
| a24 | mesbox |  | A loud gnawing begins upstairs. | 9/17 | QlQ_p-MBxJ8 @83, ROB_P-ogto4 @112, 2EfU5Mrh8Kc @49 |
| a25 | chatnpc | Pikkupstix | The downside to these obelisks is that rogue familiars, ⏎ like the one upstairs, get into this world and wreak ⏎ havoc. | 9/17 | QlQ_p-MBxJ8 @83, ROB_P-ogto4 @115, 2EfU5Mrh8Kc @50 |
| a26 | chatnpc | Pikkupstix | I chased it upstairs, but I didn't have the necessary ⏎ Summoning pouch to banish it. | 9/17 | QlQ_p-MBxJ8 @84, ROB_P-ogto4 @121, 2EfU5Mrh8Kc @51 |
| a27 | chatnpc | Pikkupstix | If I leave this house, it will come down here, potentially ⏎ even getting to the obelisk. If that happens, it could call ⏎ another familiar through and multiply! | 10/17 | QlQ_p-MBxJ8 @85, ROB_P-ogto4 @125, 2EfU5Mrh8Kc @52 |
| a28 | chatnpc | Pikkupstix | I sent my assistant, Stikklebrix, out to bring me what I ⏎ needed, but he has not returned. | 10/17 | QlQ_p-MBxJ8 @85, ROB_P-ogto4 @137, 2EfU5Mrh8Kc @54 |
| a29 | mesbox |  | The gnawing stops. There is a crash. | 9/17 | QlQ_p-MBxJ8 @86, ROB_P-ogto4 @141, 2EfU5Mrh8Kc @55 |
| a30 | chatplayer | PLAYER | How fast can it multiply? | 9/17 | QlQ_p-MBxJ8 @86, ROB_P-ogto4 @145, 2EfU5Mrh8Kc @56 |
| a31 | chatnpc | Pikkupstix | Like a rabbit, because it is- | 9/17 | QlQ_p-MBxJ8 @87, ROB_P-ogto4 @147, 2EfU5Mrh8Kc @57 |
| a32 | chatplayer | PLAYER | A rabbit! | 11/17 | QlQ_p-MBxJ8 @88, ROB_P-ogto4 @149, 2EfU5Mrh8Kc @58 |
| a33 | chatnpc | Pikkupstix | Well yes, rabbit-like, but- | 10/17 | QlQ_p-MBxJ8 @90, ROB_P-ogto4 @151, 2EfU5Mrh8Kc @59 |
| a34 | chatplayer | PLAYER | Oh, don't worry, I'll deal with this myself. One rabbit ⏎ stew coming right up. | 9/17 | ROB_P-ogto4 @153, 2EfU5Mrh8Kc @60, JQZnlZk0KqE @113 |
| a35 | chatnpc | Pikkupstix | Very well, young fool. You go see how well you do ⏎ against it. | 12/17 | QlQ_p-MBxJ8 @91, ROB_P-ogto4 @155, 2EfU5Mrh8Kc @61 |
| c1_01 | chatplayer | PLAYER | Come on then little, fluffy... | 11/17 | QlQ_p-MBxJ8 @100, 2EfU5Mrh8Kc @70, JQZnlZk0KqE @122 |
| c1_02 | chatplayer | PLAYER | Gigantic... | 5/17 | JQZnlZk0KqE @125, EADAootOI24 @112, ldSYrbNZbI8 @104 |
| c1_03 | chatplayer | PLAYER | scary-looking... | 9/17 | JQZnlZk0KqE @127, EADAootOI24 @114, ldSYrbNZbI8 @106 |
| c1_04 | chatplayer | PLAYER | razor-fanged... | 4/17 | JQZnlZk0KqE @128, ldSYrbNZbI8 @108, 0NxpQ6TqrTo @160 |
| c1_04b | chatplayer | PLAYER | bunny... | 8/17 | 2EfU5Mrh8Kc @81, JQZnlZk0KqE @130, ldSYrbNZbI8 @109 |
| c1_05 | chatplayer | PLAYER | What is that thing? | 9/17 | QlQ_p-MBxJ8 @111, 2EfU5Mrh8Kc @85, JQZnlZk0KqE @133 |
| c1_06 | chatplayer | PLAYER | Maybe, if I leave quietly, it won't notice me. | 11/17 | QlQ_p-MBxJ8 @113, 2EfU5Mrh8Kc @86, JQZnlZk0KqE @135 |
| c1_07 | mesbox |  | A chilling, unnatural fear envelops you! | 11/17 | QlQ_p-MBxJ8 @118, 2EfU5Mrh8Kc @91, JQZnlZk0KqE @140 |
| c1_08 | chatplayer | PLAYER | It's coming to get me! | 11/17 | QlQ_p-MBxJ8 @120, 2EfU5Mrh8Kc @93, JQZnlZk0KqE @142 |
| b01 | chatplayer | PLAYER | The teeth! | 13/17 | QlQ_p-MBxJ8 @130, ROB_P-ogto4 @214, 2EfU5Mrh8Kc @104 |
| b02 | chatnpc | Pikkupstix | So, you finally saw what you're up against, eh? Not as ⏎ harmless as you assumed, I take it? | 13/17 | QlQ_p-MBxJ8 @132, ROB_P-ogto4 @215, 2EfU5Mrh8Kc @106 |
| b03 | chatplayer | PLAYER | Horns! Teeth! | 12/17 | QlQ_p-MBxJ8 @133, ROB_P-ogto4 @219, 2EfU5Mrh8Kc @107 |
| b04 | chatnpc | Pikkupstix | Are you prepared to treat this situation with the gravity ⏎ it deserves, now? | 12/17 | QlQ_p-MBxJ8 @134, ROB_P-ogto4 @220, 2EfU5Mrh8Kc @108 |
| b05 | chatplayer | PLAYER | What IS it? | 8/17 | ROB_P-ogto4 @223, 2EfU5Mrh8Kc @110, JQZnlZk0KqE @159 |
| b06 | chatnpc | Pikkupstix | A wolpertinger, and a pretty big one at that. | 12/17 | QlQ_p-MBxJ8 @136, ROB_P-ogto4 @225, 2EfU5Mrh8Kc @111 |
| b07 | chatnpc | Pikkupstix | They are spirits that tend to be on the energetic and ⏎ destructive side when they manifest here, but are a little ⏎ less violent on the spirit plane. | 10/17 | QlQ_p-MBxJ8 @137, ROB_P-ogto4 @228, 2EfU5Mrh8Kc @112 |
| b08 | chatplayer | PLAYER | So, what can be done about it? Can't you banish it? | 10/17 | QlQ_p-MBxJ8 @138, ROB_P-ogto4 @232, 2EfU5Mrh8Kc @113 |
| b09 | chatnpc | Pikkupstix | Well, I could, but in order to do so, I'd need a spirit ⏎ wolf pouch. | 11/17 | QlQ_p-MBxJ8 @138, ROB_P-ogto4 @234, 2EfU5Mrh8Kc @114 |
| b10 | chatnpc | Pikkupstix | Wolpertingers are generally afraid of wolves; that's the ⏎ rabbity-side of them. A spirit wolf will scare it and banish ⏎ it. | 10/17 | QlQ_p-MBxJ8 @139, ROB_P-ogto4 @238, 2EfU5Mrh8Kc @115 |
| b11 | chatnpc | Pikkupstix | The problem is that I don't have any of the necessary ⏎ spirit wolf pouches, and the only thing keeping the giant ⏎ wolpertinger upstairs is my presence. If I were to leave, ⏎ it would amble downstairs to bring more of its kind | 10/17 | QlQ_p-MBxJ8 @141, ROB_P-ogto4 @244, 2EfU5Mrh8Kc @117 |
| b12 | chatnpc | Pikkupstix | through, as I said before. | 11/17 | QlQ_p-MBxJ8 @141, ROB_P-ogto4 @248, 2EfU5Mrh8Kc @118 |
| b13 | chatplayer | PLAYER | Well, what if I were to bring you the elements you ⏎ needed? Would that work? | 11/17 | QlQ_p-MBxJ8 @142, ROB_P-ogto4 @250, 2EfU5Mrh8Kc @119 |
| b14 | chatnpc | Pikkupstix | Not really; I would need to go down to the obelisk in ⏎ my cellar and use the necessary ingredients to make a ⏎ spirit wolf pouch and some Howl scrolls, but it may slip ⏎ out of the front door. | 11/17 | QlQ_p-MBxJ8 @144, ROB_P-ogto4 @253, 2EfU5Mrh8Kc @120 |
| b15 | chatnpc | Pikkupstix | You could infuse the pouch yourself, though. Would ⏎ you like to learn the secrets of Summoning? I can ⏎ sense the spark within you, urging you to master the ⏎ art. | 10/17 | QlQ_p-MBxJ8 @144, ROB_P-ogto4 @254, 2EfU5Mrh8Kc @121 |
| b16 | chatplayer | PLAYER | Certainly! | 11/17 | QlQ_p-MBxJ8 @145, ROB_P-ogto4 @258, 2EfU5Mrh8Kc @123 |
| b17 | chatplayer | PLAYER | What do you need me to bring? | 12/17 | QlQ_p-MBxJ8 @145, ROB_P-ogto4 @259, 2EfU5Mrh8Kc @124 |
| b18 | chatnpc | Pikkupstix | That's wonderful! | 12/17 | QlQ_p-MBxJ8 @146, ROB_P-ogto4 @261, 2EfU5Mrh8Kc @126 |
| b19 | chatnpc | Pikkupstix | You need to bring two lots of wolf bones. I can provide ⏎ the other items you will need. | 12/17 | QlQ_p-MBxJ8 @147, ROB_P-ogto4 @263, 2EfU5Mrh8Kc @127 |
| m_why | menu |  | Select an Option: I'll get right on it. ⏎ Wait. Why do you need those things? | 11/17 | QlQ_p-MBxJ8 @147, ROB_P-ogto4 @268, JQZnlZk0KqE @194 |
| b20 | chatplayer | PLAYER | I'll get right on it. | 12/17 | QlQ_p-MBxJ8 @150, ROB_P-ogto4 @270, 2EfU5Mrh8Kc @130 |
| w01 | chatplayer | PLAYER | Wait. Why do you need those things? | 10/17 | QlQ_p-MBxJ8 @149, 2EfU5Mrh8Kc @129, JQZnlZk0KqE @195 |
| w02 | chatnpc | Pikkupstix | Well, I can tell you in depth later, but, since you ask, ⏎ those are the ingredients for a spirit wolf pouch. | 1/17 | lGcCWqJlpSo @114 |
| w03 | chatnpc | Pikkupstix | One of the pouches can be split into some scrolls, which ⏎ we will need to banish the giant wolpertinger. | 1/17 | lGcCWqJlpSo @114 |
| w04 | chatplayer | PLAYER | Just wolf bones? | 1/17 | lGcCWqJlpSo @116 |
| w05 | chatnpc | Pikkupstix | No, no. There are other items that I need to give you, ⏎ but I do not have access to the bones. | 1/17 | lGcCWqJlpSo @117 |
| w06 | chatnpc | Pikkupstix | The wolf bones give a solidity to the spirit wolf's spirit. ⏎ That's why I need them for a spirit wolf pouch. | 1/17 | lGcCWqJlpSo @117 |
| w07 | chatnpc | Pikkupstix | Other familiars' pouches require other items, like ⏎ different charm types, or a larger number of spirit ⏎ shards. | 1/17 | lGcCWqJlpSo @119 |
| w08 | chatnpc | Pikkupstix | Since the scrolls transform from Summoning pouches, ⏎ you will need to bring enough ingredients for two ⏎ Summoning pouches. This will leave you one spirit wolf ⏎ pouch to summon the spirit wolf, and a set of scrolls. | 1/17 | lGcCWqJlpSo @120 |
| w09 | chatnpc | Pikkupstix | The charms are used to differentiate between different ⏎ classes of spirit, as there might well be a spirit out there ⏎ that requires a crimson charm and wolf bones. ⏎ Obviously it would be a completely different animal to | 1/17 | lGcCWqJlpSo @121 |
| w10 | chatnpc | Pikkupstix | the spirit wolf, but the charm lets you know this in ⏎ advance. | 1/17 | lGcCWqJlpSo @122 |
| w11 | chatnpc | Pikkupstix | The spirit shards are chips of an obelisk, and they give ⏎ a focus to the spirit. | 1/17 | lGcCWqJlpSo @123 |
| w12 | chatnpc | Pikkupstix | More powerful creatures need more shards to focus on. | 1/17 | lGcCWqJlpSo @124 |
| w13 | chatnpc | Pikkupstix | The scrolls allow a spirit wolf to perform a special move, ⏎ allowing it Howl at the giant wolpertinger, sending it ⏎ home. | 1/17 | lGcCWqJlpSo @125 |
| w14 | chatplayer | PLAYER | So, what items are you going to give me for the ⏎ Summoning pouch? | 1/17 | lGcCWqJlpSo @125 |
| w15 | chatnpc | Pikkupstix | You'll find that out when you bring me the bones, won't ⏎ you? | 1/17 | lGcCWqJlpSo @126 |
| w16 | chatplayer | PLAYER | Sorry for asking. | 1/17 | lGcCWqJlpSo @127 |
| d01 | chatplayer | PLAYER | This must be Stikklebrix. | 4/17 | ROB_P-ogto4 @290, WKPggURQByY @175, lGcCWqJlpSo @164 |
| d02 | chatplayer | PLAYER | The poor fool didn't stand a chance against the wolves. | 5/17 | ROB_P-ogto4 @292, WKPggURQByY @177, zGpeA1CrlVg @43 |
| d04 | chatplayer | PLAYER | Let's see what he has in his pack: three unfinished ⏎ Strength potions, two burnt lobsters, a guam leaf with a ⏎ footprint on it... | 4/17 | WKPggURQByY @178, zGpeA1CrlVg @44, lGcCWqJlpSo @165 |
| d05 | chatplayer | PLAYER | and some wolf bones! | 4/17 | WKPggURQByY @179, zGpeA1CrlVg @45, lGcCWqJlpSo @166 |
| d06 | chatplayer | PLAYER | It looks he got enough wolf bones for Pikkupstix and ⏎ then died. How tragic! | 3/17 | WKPggURQByY @180, lGcCWqJlpSo @166, f5AzI4WjuVc @132 |
| d07 | chatplayer | PLAYER | Well, his sacrifice will not be in vain. | 4/17 | WKPggURQByY @181, zGpeA1CrlVg @46, lGcCWqJlpSo @167 |
| d03 | chatplayer | PLAYER | I don't have time to search him. I have to get these ⏎ wolf bones back to Pikkupstix so that Stikklebrix's ⏎ sacrifice is not in vain. | 1/17 | ROB_P-ogto4 @294 |
| e01 | chatplayer | PLAYER | I have the wolf bones right here. | 11/17 | QlQ_p-MBxJ8 @194, ROB_P-ogto4 @308, 2EfU5Mrh8Kc @136 |
| e02 | chatnpc | Pikkupstix | Splendid, dear boy. | 11/17 | ROB_P-ogto4 @310, 2EfU5Mrh8Kc @138, JQZnlZk0KqE @259 |
| e03 | chatnpc | Pikkupstix | Here are the pouches, spirit shards and charms you will ⏎ need to make the spirit wolf pouch and Howl scrolls. ⏎ This key is to the trapdoor over there, which leads to ⏎ the obelisk. | 12/17 | QlQ_p-MBxJ8 @196, ROB_P-ogto4 @311, 2EfU5Mrh8Kc @139 |
| e04 | chatplayer | PLAYER | What are these things? | 10/17 | QlQ_p-MBxJ8 @197, ROB_P-ogto4 @315, 2EfU5Mrh8Kc @140 |
| e05 | chatnpc | Pikkupstix | Well, the wolf bones, spirit shards and gold charms are ⏎ all ingredients to go into the pouch. | 11/17 | QlQ_p-MBxJ8 @198, ROB_P-ogto4 @317, 2EfU5Mrh8Kc @141 |
| e06 | chatnpc | Pikkupstix | The wolf bones give a solidity to the spirit, the charm ⏎ attracts the type of familiar that you desire - in this ⏎ case a gold charm - and the shards are the spirit's ⏎ focus. | 11/17 | QlQ_p-MBxJ8 @199, ROB_P-ogto4 @320, 2EfU5Mrh8Kc @142 |
| e07 | chatnpc | Pikkupstix | You will first need to make two Summoning pouches: ⏎ one that can be used to summon the spirit wolf, and ⏎ another to tear open at the obelisk, creating some Howl ⏎ scrolls. These Howl scrolls will help make the spirit wolf | 12/17 | QlQ_p-MBxJ8 @201, ROB_P-ogto4 @324, 2EfU5Mrh8Kc @144 |
| e08 | chatnpc | Pikkupstix | perform a fear-inducing Howl special move. | 11/17 | QlQ_p-MBxJ8 @202, ROB_P-ogto4 @328, 2EfU5Mrh8Kc @145 |
| e09 | chatplayer | PLAYER | Okay, I think I understood that. So, how do I get the ⏎ obelisk to work? | 11/17 | QlQ_p-MBxJ8 @204, ROB_P-ogto4 @330, 2EfU5Mrh8Kc @146 |
| e10 | chatnpc | Pikkupstix | Well, you stand before the obelisk with the charms, ⏎ pouches, shards and bones necessary to complete two ⏎ Summoning pouches. You should then 'use' your empty ⏎ pouch on the obelisk. Your ingredients will be added to | 12/17 | QlQ_p-MBxJ8 @205, ROB_P-ogto4 @335, 2EfU5Mrh8Kc @147 |
| e11 | chatnpc | Pikkupstix | the pouch, mixing with the spirit of a familiar to create a ⏎ spirit wolf pouch. | 12/17 | QlQ_p-MBxJ8 @205, ROB_P-ogto4 @346, 2EfU5Mrh8Kc @149 |
| e12 | chatnpc | Pikkupstix | Creating a scroll is a similar process. By using a ⏎ Summoning pouch on an obelisk and breaking it open, ⏎ you are allowing the spirit energy to transform into a ⏎ different form - some scrolls. | 12/17 | QlQ_p-MBxJ8 @206, ROB_P-ogto4 @348, 2EfU5Mrh8Kc @149 |
| e13 | chatnpc | Pikkupstix | Once you have a spirit wolf pouch and some Howl ⏎ scrolls, you can come upstairs to see me, to find out ⏎ what must be done with the giant wolpertinger. | 12/17 | QlQ_p-MBxJ8 @207, ROB_P-ogto4 @351, 2EfU5Mrh8Kc @150 |
| e14 | chatplayer | PLAYER | It all sounds simple enough. | 12/17 | QlQ_p-MBxJ8 @208, ROB_P-ogto4 @354, 2EfU5Mrh8Kc @150 |
| f01 | chatplayer | PLAYER | Here is the pouch and scroll. | 13/17 | QlQ_p-MBxJ8 @243, ROB_P-ogto4 @422, 2EfU5Mrh8Kc @189 |
| f02 | chatnpc | Pikkupstix | Wonderful! Now, all you have to do is go upstairs and ⏎ summon the spirit wolf, then the Howl effect. | 13/17 | QlQ_p-MBxJ8 @244, ROB_P-ogto4 @424, 2EfU5Mrh8Kc @190 |
| f03 | chatplayer | PLAYER | That's it? | 10/17 | QlQ_p-MBxJ8 @245, ROB_P-ogto4 @428, 2EfU5Mrh8Kc @191 |
| f04 | chatnpc | Pikkupstix | It's that simple. The spirit wolf will, when you use the ⏎ Howl scroll, chase away the giant wolpertinger and then ⏎ disappear. Under normal circumstances, the spirit wolf ⏎ would follow you around, defend you in combat and | 12/17 | QlQ_p-MBxJ8 @247, ROB_P-ogto4 @430, 2EfU5Mrh8Kc @192 |
| f05 | chatnpc | Pikkupstix | often lend you its powers. | 11/17 | QlQ_p-MBxJ8 @248, ROB_P-ogto4 @431, 2EfU5Mrh8Kc @193 |
| f06 | chatplayer | PLAYER | What sort of powers? | 11/17 | QlQ_p-MBxJ8 @249, ROB_P-ogto4 @433, 2EfU5Mrh8Kc @194 |
| f07 | chatnpc | Pikkupstix | Well, for a start, it has the ability to perform a ⏎ Summoning scroll that forces opponents to flee. That is ⏎ what it will use on the giant wolpertinger, with the ⏎ difference being that the giant wolpertinger will be so | 11/17 | QlQ_p-MBxJ8 @250, ROB_P-ogto4 @435, 2EfU5Mrh8Kc @195 |
| f08 | chatnpc | Pikkupstix | scared that it will retreat to its spirit plane, rather than ⏎ face the spirit wolf. | 12/17 | QlQ_p-MBxJ8 @252, ROB_P-ogto4 @441, 2EfU5Mrh8Kc @196 |
| f09 | chatplayer | PLAYER | Great! So, how will I make it perform that Summoning ⏎ scroll? | 12/17 | QlQ_p-MBxJ8 @253, ROB_P-ogto4 @443, 2EfU5Mrh8Kc @198 |
| f10 | chatnpc | Pikkupstix | All you need to do is use the special move button in ⏎ your follower details and aim it at the giant ⏎ wolpertinger, once the spirit wolf has been summoned. ⏎ The spirit wolf will then perform its special move. | 12/17 | QlQ_p-MBxJ8 @254, ROB_P-ogto4 @446, 2EfU5Mrh8Kc @200 |
| f11 | mesbox |  | To access your follower details, right-click the Summoning icon near ⏎ your minimap and select 'Follower Details'. | 12/17 | QlQ_p-MBxJ8 @255, ROB_P-ogto4 @450, 2EfU5Mrh8Kc @201 |
| f12 | mesbox |  | You can also access your familiar's spell and various other functions ⏎ from this menu. | 12/17 | QlQ_p-MBxJ8 @256, ROB_P-ogto4 @457, 2EfU5Mrh8Kc @202 |
| f13 | mesbox |  | Bear in mind that the icon will only function so long as you have a ⏎ Summoning follower present. | 13/17 | QlQ_p-MBxJ8 @257, ROB_P-ogto4 @462, 2EfU5Mrh8Kc @204 |
| f14 | chatplayer | PLAYER | What a relief! | 13/17 | QlQ_p-MBxJ8 @258, ROB_P-ogto4 @465, 2EfU5Mrh8Kc @205 |
| f15 | chatnpc | Pikkupstix | Well, I think that I have talked enough. Now you have ⏎ to put it into practice. | 12/17 | QlQ_p-MBxJ8 @259, ROB_P-ogto4 @466, 2EfU5Mrh8Kc @206 |
| f16 | chatplayer | PLAYER | Wish me luck! | 10/17 | QlQ_p-MBxJ8 @261, 2EfU5Mrh8Kc @207, JQZnlZk0KqE @381 |
| c2_01 | chatplayer | PLAYER | Mustn't look round, bunny will eat me. ⏎ Mustn't look round, bunny will eat me. ⏎ Mustn't look round, bunny will eat me. | 10/17 | QlQ_p-MBxJ8 @269, 2EfU5Mrh8Kc @217, JQZnlZk0KqE @390 |
| c2_02 | chatplayer | PLAYER | Okay, I have the spirit wolf pouch, so it can't hurt me... | 12/17 | QlQ_p-MBxJ8 @272, 2EfU5Mrh8Kc @220, JQZnlZk0KqE @392 |
| c2_03 | chatplayer | PLAYER | ...I hope. | 9/17 | 2EfU5Mrh8Kc @222, EADAootOI24 @324, ldSYrbNZbI8 @235 |
| c2_04 | chatplayer | PLAYER | Oh, good. It's too busy gnawing to notice me. | 10/17 | QlQ_p-MBxJ8 @278, 2EfU5Mrh8Kc @224, JQZnlZk0KqE @397 |
| c2_05 | chatplayer | PLAYER | I spoke too soon! | 8/17 | JQZnlZk0KqE @400, EADAootOI24 @330, ldSYrbNZbI8 @241 |
| c2_06 | chatplayer | PLAYER | I'll use the spirit wolf pouch. I hope this works. | 10/17 | QlQ_p-MBxJ8 @283, JQZnlZk0KqE @401, WKPggURQByY @309 |
| c3_01 | chatplayer | PLAYER | Get that thing! | 9/17 | QlQ_p-MBxJ8 @300, JQZnlZk0KqE @422, EADAootOI24 @350 |
| c3_02 | chatplayer | PLAYER | Well, that certainly seems to have got rid of the giant ⏎ wolpertinger. | 8/17 | QlQ_p-MBxJ8 @309, 2EfU5Mrh8Kc @261, EADAootOI24 @359 |
| c3_03 | chatplayer | PLAYER | Come on, let's go tell Pikkupstix. | 7/17 | QlQ_p-MBxJ8 @311, 2EfU5Mrh8Kc @263, ldSYrbNZbI8 @280 |
| c3_04 | chatplayer | PLAYER | They both vanished! I'm sure Pikkupstix will be able to ⏎ explain this to me. | 8/17 | 2EfU5Mrh8Kc @268, EADAootOI24 @365, ldSYrbNZbI8 @284 |
| g01 | chatnpc | Pikkupstix | So, how did it go? | 13/17 | QlQ_p-MBxJ8 @325, ROB_P-ogto4 @561, 2EfU5Mrh8Kc @278 |
| g02 | chatplayer | PLAYER | The spirit wolf scared it away! | 13/17 | QlQ_p-MBxJ8 @326, ROB_P-ogto4 @562, 2EfU5Mrh8Kc @280 |
| g03 | chatnpc | Pikkupstix | My dear boy, you've done it! | 13/17 | QlQ_p-MBxJ8 @327, ROB_P-ogto4 @565, 2EfU5Mrh8Kc @281 |
| g04 | chatnpc | Pikkupstix | I just hope the damage to my room isn't too bad. | 13/17 | QlQ_p-MBxJ8 @328, ROB_P-ogto4 @567, 2EfU5Mrh8Kc @282 |
| g05 | chatplayer | PLAYER | All in a day's... | 13/17 | QlQ_p-MBxJ8 @329, ROB_P-ogto4 @570, 2EfU5Mrh8Kc @284 |
| g06 | chatplayer | PLAYER | Woah, I feel a little dizzy. | 13/17 | QlQ_p-MBxJ8 @330, ROB_P-ogto4 @571, 2EfU5Mrh8Kc @285 |
| g07 | chatnpc | Pikkupstix | Well, that Summoning was a drain on your rather ⏎ untrained reserves. | 13/17 | QlQ_p-MBxJ8 @330, ROB_P-ogto4 @574, 2EfU5Mrh8Kc @286 |
| g08 | chatnpc | Pikkupstix | Go downstairs to the obelisk, as it will renew your ⏎ energy. | 13/17 | QlQ_p-MBxJ8 @331, ROB_P-ogto4 @582, 2EfU5Mrh8Kc @287 |
| g09 | chatnpc | Pikkupstix | You should do that immediately - you'll be cut off from ⏎ Summoning while you have no Summoning skill points ⏎ remaining. | 12/17 | QlQ_p-MBxJ8 @332, ROB_P-ogto4 @585, 2EfU5Mrh8Kc @289 |
| g10 | chatplayer | PLAYER | Wooooah... | 10/17 | QlQ_p-MBxJ8 @333, ROB_P-ogto4 @592, JQZnlZk0KqE @457 |
| g11 | chatnpc | Pikkupstix | You really need to renew your Summoning skill points ⏎ at the obelisk, <player name>. | 12/17 | QlQ_p-MBxJ8 @335, ROB_P-ogto4 @594, 2EfU5Mrh8Kc @291 |
| h01 | chatnpc | Pikkupstix | Feeling refreshed? | 12/17 | QlQ_p-MBxJ8 @361, ROB_P-ogto4 @623, 2EfU5Mrh8Kc @317 |
| h02 | chatplayer | PLAYER | Yeah, that does feel a lot better! | 12/17 | QlQ_p-MBxJ8 @362, ROB_P-ogto4 @625, 2EfU5Mrh8Kc @318 |
| h03 | chatplayer | PLAYER | Won't my energy renew on its own? | 11/17 | QlQ_p-MBxJ8 @363, ROB_P-ogto4 @627, 2EfU5Mrh8Kc @320 |
| h04 | chatnpc | Pikkupstix | Not really. Your Summoning skill points need to be ⏎ renewed at the obelisks or mini obelisks scattered ⏎ around the world, while your special move bar will ⏎ recharge slowly over time. | 11/17 | QlQ_p-MBxJ8 @364, ROB_P-ogto4 @630, 2EfU5Mrh8Kc @321 |
| h05 | chatnpc | Pikkupstix | Think of your Summoning skill points as Prayer ⏎ Points, that need to be recharged from time-to-time. | 11/17 | QlQ_p-MBxJ8 @366, ROB_P-ogto4 @639, 2EfU5Mrh8Kc @322 |
| h06 | chatnpc | Pikkupstix | The special move bar is like your special attack bar, not ⏎ only because it is useful when attacking, but because it ⏎ recharges slowly over time. | 10/17 | QlQ_p-MBxJ8 @367, ROB_P-ogto4 @645, 2EfU5Mrh8Kc @323 |
| h07 | chatnpc | Pikkupstix | The action of Summoning a familiar uses up your ⏎ Summoning skill points, while a special move will deplete ⏎ your special move bar. | 9/17 | QlQ_p-MBxJ8 @368, ROB_P-ogto4 @653, 2EfU5Mrh8Kc @324 |
| h08 | chatnpc | Pikkupstix | The best way to recover either of these is at the ⏎ obelisks. | 9/17 | QlQ_p-MBxJ8 @371, ROB_P-ogto4 @656, 2EfU5Mrh8Kc @326 |
| h09 | chatnpc | Pikkupstix | To help you on your way, I can tell you that I know ⏎ of two other obelisks you can use. | 10/17 | QlQ_p-MBxJ8 @372, ROB_P-ogto4 @659, 2EfU5Mrh8Kc @327 |
| h10 | chatnpc | Pikkupstix | One is guarded by the ogres in Gu'Tanoth, and the ⏎ other is on the coast near Brimhaven. There will be ⏎ others, however. | 12/17 | QlQ_p-MBxJ8 @373, ROB_P-ogto4 @663, 2EfU5Mrh8Kc @328 |
| h11 | chatplayer | PLAYER | Thanks for the information. | 11/17 | QlQ_p-MBxJ8 @375, ROB_P-ogto4 @666, 2EfU5Mrh8Kc @329 |
| h12 | chatnpc | Pikkupstix | My dear boy, it was nothing. | 11/17 | QlQ_p-MBxJ8 @376, ROB_P-ogto4 @668, 2EfU5Mrh8Kc @330 |
| h13 | chatnpc | Pikkupstix | Well, from your actions, you have proved yourself ⏎ worthy to learn the secrets of Summoning. | 11/17 | QlQ_p-MBxJ8 @378, ROB_P-ogto4 @670, 2EfU5Mrh8Kc @332 |
| h14 | chatnpc | Pikkupstix | Speak to me on the subject if you wish to learn more ⏎ about it. In the meantime, I will use my power to grant ⏎ you access to the skill. | 11/17 | QlQ_p-MBxJ8 @379, ROB_P-ogto4 @673, 2EfU5Mrh8Kc @333 |
| h15 | chatnpc | Pikkupstix | Here is a little something to help you start your ⏎ Summoning training. | 12/17 | QlQ_p-MBxJ8 @381, ROB_P-ogto4 @678, 2EfU5Mrh8Kc @334 |
| h16 | chatplayer | PLAYER | Thanks! | 9/17 | QlQ_p-MBxJ8 @382, ROB_P-ogto4 @682, 2EfU5Mrh8Kc @335 |

Notes on the table:

- **m_topic** opens every talk with Pikkupstix at every stage seen, before and during the quest (first at ROB_ @29,
  2EfU @17, JQZn @40; again at each return). Title "Select an Option". Only the first option is ever picked; **what
  "Talk about Pikkenmix." says is not shown by any recording.**
- **a01-a35** first talk. After a35 the dialogue closes; the player climbs the ladder (cutscene 1, c1_*).
- **b01-b20** second talk (after cutscene 1). **m_why** title "Select an Option", options "I'll get right on it." /
  "Wait. Why do you need those things?". Option 1: b20, dialogue ends. Option 2: w01-w16 (only lGcC @112-127, read by
  eye from every frame; w13 really reads "allowing it Howl at", no "to"), then the dialogue ends after w16.
- **d01-d07** dead body (section 3).
- **e01-e14** hand-over talk; items appear in the inventory with no item box (section 10).
- **f01-f16** after making pouch and scrolls. f11-f13 are plain message boxes in blue text with no chathead
  (HemC @185-187, ROB_ @450-462).
- **c2_*, c3_*** attic cutscenes 2 and 3 (section 2).
- **g01-g11** after the banishing. g11 ends with the player's display name ("...at the obelisk, Reubens Here." HemC
  @239; "...at the obelisk, B F W." JQZn @459; "...Love2phreak." ROB_ @594).
- **h01-h16** final talk, then the quest-complete scroll.
- d06 really reads "It looks he got enough wolf bones" (no "like"): f5Az @131, lGcC @166, WKPg @180.

### Branch lines: what was and was not found

| situation | found? |
|---|---|
| "Wait. Why do you need those things?" | yes, w01-w16 (1 recording, lGcC) |
| Searching the body with bones already held | yes, d01, d02, d03 (ROB_ @289-296) |
| Searching the body without bones | yes, d01, d02, d04-d07 (f5Az, lGcC, zGpe, WKPg) |
| Open trapdoor without key | yes (section 4) |
| Re-talk to Pikkupstix without the bones / before climbing the ladder / with only a pouch or only scrolls | **not shown** |
| Talking to Pikkupstix in stage g again without renewing | not shown as a talk; only g01-g11 once each |
| Talking after completion, shop, enchanting headgear, "Talk about Pikkenmix." | **not shown** |
| Using the ladder again after cutscene 1 and before having pouch + scrolls | **not shown** |

## 2. Attic cutscenes

Times are from JQZnlZk0KqE (10 fps, 2011-05-24) with HemCMqn3Y3E (24 fps, 2009) alongside. Contact sheets:
`eye/cs1_jqz.png`, `eye/cs2_jqz.png`, `eye/cs3_jqz.png`, `eye/cs1_hemc.png`, `eye/cs3_hemc.png`.
In all three the minimap is blacked out and the side tabs are hidden from the fade until the scene ends.

### Cutscene 1 (first climb; JQZn 120-147, HemC 51-66)

| JQZn s | HemC s | beat |
|---|---|---|
| 120.0 | 51 | player clicks Climb-up on the ladder; screen starts fading to black (about 1 s) |
| 121.0-122.4 | 52 | black |
| 122.5-123.0 | 53 | fade in. Shot A: low camera in the attic looking at the player from the front, standing at the top of the ladder, facing the camera. Chatbox: c1_01 "Come on then little, fluffy..." |
| 124.5 | 54 | on continue, the player's pose changes (weapon arm drops; a small recoil). c1_02 "Gigantic..." |
| 127.5 | 55 | c1_03 "scary-looking..." |
| 128.5 | 55.5 | c1_04 "razor-fanged..." |
| 129.5 | 56.2 | c1_04b "bunny..." (face now wide-eyed, HemC @56) |
| 130.7 | 56.7 | hard cut. Shot B: camera behind the player's shoulder looking across the green rug at the giant wolpertinger, which stands at the far side of the rug with its back half-turned, gnawing |
| 132.0-133.0 | 58 | overhead text over the wolpertinger: **"Mrooowr?"** (yellow; JQZn @132.6, HemC @58, 2EfU @82.5); it turns its head to the player |
| 133.5 | 57.8 | c1_05 "What is that thing?" |
| 135.0 | 58.7 | c1_06 "Maybe, if I leave quietly, it won't notice me." |
| 137.5-139.0 | 60-61 | box closes; the wolpertinger rears/turns and lunges toward the camera |
| 139.5-140.2 | 61.2 | overhead **"Raaaarw!"** with a white jagged shock-wave graphic leaving its mouth toward the player (HemC @61.2 read by eye; JQZn @139.8) |
| 140.4 | 61.5 | hard cut back to Shot A. Message box (no head): c1_07 "A chilling, unnatural fear envelops you!" |
| 142.5 | 62.4 | c1_08 "It's coming to get me!"; the player plays a cowering/terrified animation (crouched, arms up, HemC @63-64) and the same words appear as the player's **overhead text and as a public chat line** ("<name>: It's coming to get me!" in the chatbox, HemC @63, JQZn @142-145) |
| 145.5-146.5 | 64.5-66 | fade to black, about 1 s black, fade in downstairs beside the ladder with normal camera; the cowering animation is still playing as the view returns (HemC @66-67) |

The player never has control in the attic in this stage. 11 of 17 recordings show it.

### Cutscene 2 (climb with pouch + scrolls; JQZn 388-404, HemC 191-202)

| JQZn s | HemC s | beat |
|---|---|---|
| 388.0-390.2 | 191-192.5 | Climb-up; fade to black |
| 390.5 | 193 | fade in on Shot A (same front-on camera at the ladder top). c2_01, three identical lines |
| 392.5 | 195 | c2_02 "Okay, I have the spirit wolf pouch, so it can't hurt me..." |
| 394.5 | 196 | c2_03 "...I hope." |
| 395.5 | 197 | hard cut to Shot B (over-shoulder view of the wolpertinger gnawing, back turned) |
| 397.0 | 198.2 | c2_04 "Oh, good. It's too busy gnawing to notice me." |
| 398.5-399.0 | 199 | the wolpertinger turns round and rears |
| 399.5-400.0 | 199.5 | overhead **"Raaaarw!"** (JQZn @399.8); c2_05 "I spoke too soon!" |
| 401.5 | 200.6 | c2_06 "I'll use the spirit wolf pouch. I hope this works." |
| 403.8 | 201.8 | hard cut (no fade) to normal play: player stands at the ladder top in the attic with full control, tabs back; the wolpertinger stays in the room (visible in the west part of the attic, JQZn @404-414, 2EfU @237) |

### Free play between 2 and 3

The player right-clicks the pouch (Use / Summon / Destroy / Examine, JQZn @406, HemC @203) and picks Summon; then
clicks the Howl special-move button in the Follower Details panel and targets the wolpertinger (section 6). Cutscene 3
starts as soon as the Howl is cast on it.

### Cutscene 3 (after Howl; JQZn 414.5-441.5, HemC 213.5-228.5)

| JQZn s | HemC s | beat |
|---|---|---|
| 414.5-415.5 | 213.5 | chat "The spirit wolf howls at the giant wolpertinger."; fade to black |
| 416-418 | 214-215 | black (about 2.5 s) |
| 418.5-419.0 | 216 | fade in. Shot C: camera behind the player, who stands on the near edge of the rug; the spirit wolf sits/stands to the player's right facing the wolpertinger, which stands on the rug facing them |
| 420.0-420.5 | 216.3 | overhead over the wolf: **"Grrrrrrrrrrrr..."** (HemC @216.3 read by eye; 2EfU @251 OCR "grrr...") |
| 421.0 | 217.0 | overhead over the wolpertinger: **"Whiiiiine!"** (HemC @217.0 read by eye) |
| 422.0 | 217.2 | c3_01 "Get that thing!" |
| 424.0-424.5 | 218.5 | the wolf raises its head and howls; the wolpertinger flips onto its back |
| 425.0-425.5 | 219-219.5 | a dark grey funnel/whirlwind graphic rises over the wolpertinger |
| 426.0-429.5 | 220-221 | the wolpertinger spins and shrinks inside it |
| 430.0 | 221.5 | the wolpertinger is gone (a small puff where it stood) |
| 430.5 | 222 | camera cuts closer (Shot D: player and wolf side by side). c3_02 "Well, that certainly seems to have got rid of the giant ⏎ wolpertinger." |
| 432.0 | 222.5 | c3_03 "Come on, let's go tell Pikkupstix." (player turns to the wolf) |
| 433.0-434.0 | 223-224 | the spirit wolf fades out (becomes transparent, then gone) with no graphic |
| 435.0 | 224.5 | c3_04 "They both vanished! I'm sure Pikkupstix will be able to ⏎ explain this to me." |
| 437.0-441.0 | 226.5-228 | fade to black (about 3 s black) |
| 441.5 | 229 | fade in downstairs next to Pikkupstix; in JQZn the m_topic menu opens at once (442.5) because the player clicks him; HemC goes straight to g01 at 229.5 (same reason, cannot tell whether the talk is automatic) |

After this the Follower Details panel is gone and the summoning orb shows 0 points (g06-g11 "dizzy").

## 3. Dead body of Stikklebrix (White Wolf Mountain)

- Object "Dead body", options **Search / Examine** (ROB_ @287 right-click menu; WKPg @173-175 top-left "Search Dead body").
- **Without wolf bones** (f5Az @126-133, lGcC @163-168, zGpe @42-46, WKPg @175-181; 4 of 17), all chatplayer, no
  message box, no item box, no game message:
  d01 "This must be Stikklebrix." → d02 "The poor fool didn't stand a chance against the wolves." →
  d04 "Let's see what he has in his pack: three unfinished ⏎ Strength potions, two burnt lobsters, a guam leaf with a ⏎ footprint on it..." →
  d05 "and some wolf bones!" → d06 "It looks he got enough wolf bones for Pikkupstix and ⏎ then died. How tragic!" →
  d07 "Well, his sacrifice will not be in vain."
  The wolf bones are then in the inventory (WKPg @183 "Drop Wolf bones" on the new item). **How many are given, and
  the exact moment they appear, is not readable** in any of the four.
- **With bones already held** (ROB_ @289-296, 1 of 17): d01 → d02 →
  d03 "I don't have time to search him. I have to get these ⏎ wolf bones back to Pikkupstix so that Stikklebrix's ⏎ sacrifice is not in vain."
- Searching again after taking the bones with none left, or after the quest: **not shown**.

## 4. Trapdoor (ground floor of Pikkupstix's house)

- Closed: options **Open / Examine** (HemC @41.5, 2EfU @154, RKNU @73). Open without the key: game message
  **"This trapdoor appears to be locked."** (HemC @43, read by eye; 1 of 17, 2009 only).
- With the key, plain **Open** (no "use key on" needed: oqiS @156 "Open Trapdoor / 3 more options", 2EfU @154-157;
  WKPg @215 uses the key on it instead, same result): game message **"You unlock the trapdoor."** (14 of 17; HemC @162,
  2EfU @158, JQZn @310). The trapdoor becomes open with options **Climb-down / Close / Examine** (HemC @177, oqiS @157,
  JQZn @385). The key is not consumed message-wise; whether it leaves the inventory is not readable.
- Climb-down: no message; arrive in the obelisk cave; first visit prints the red line
  "You have unlocked a new music track: Animal Apogee." (HemC @164, JQZn @324, 2EfU @186).
- Cave has a ladder back up (visible HemC @168). Messages on Close, or on Open a second time: **not shown**
  (QlQ_ @271 OCR once read "You lock the trapdoor." — single OCR frame, not confirmed by eye; treat as unconfirmed).

## 5. Obelisk

- Object "Obelisk", options **Infuse-pouch / Renew-points / Walk here / Examine / Cancel** (HemC @165, 2EfU @304.5, 9uuj @160).
- **Infuse-pouch opens a main-screen window titled "Summoning Pouch Creation"** (15 of 17; HemC @166.5, JQZn @316,
  2EfU @163). Layout: a grid of pouch icons 8 per row with a scrollbar, each with a small number under it
  (shard counts: 7, 8, 8, 9, 7, 1, 45, 57 / 64, 75, 51, 47, 84, 81, 84, 72 / ...), close X top-right. Top-left has
  **two small icon buttons** (a pouch/wolf-head icon and a scroll icon); the second has tooltip
  **"Swap between creating animal pouches and scrolls"** (2EfU @172) and top-left action text **"Transform Scrolls"**
  (0Nxp @251, lGcC @236, 2EfU @171, JQZn @338, WKPg @248). Clicking it replaces the window content with the
  **"Scroll Creation"** view (same frame, same grid, scroll icons with "1" under them; HemC @171.5, JQZn @343,
  2EfU @174.5). So: **one window, two modes switched by the top-left buttons** — not two tabs side by side and not
  two separately opened windows. An older client view (0Nxp @243, Sept 2009) shows the same window as a 4-per-row
  list with names under icons ("Spirit wolf pouch", "Dreadfowl pouch", "Spirit spider pouch", "Thorny snail pouch"...).
  Only what the player can make is lit; the rest are greyed.
- Hover tooltip on the wolf pouch slot (HemC @167, WKPg @237): "Level 1: Spirit wolf pouch" / "This item requires"
  / icons with counts **2/1** (pouch), **14/7** (spirit shards), **2/1** (gold charm), **2/1** (wolf bones) — held/needed.
  Scroll slot (HemC @172, 2EfU @174.5): "Level 1: Howl scroll" / "This item requires" / pouch icon **2/1**.
- **Right-click on the pouch slot: "Infuse Spirit wolf pouch / Examine Spirit wolf pouch / Cancel"** only
  (lGcC @229.7, ROB_ @388.6, JQZn @318.5, 0Nxp @243; top-left shows "Infuse Spirit wolf pouch / 1 more options").
  **Right-click on the scroll slot: "Transform Howl scroll / Examine Howl scroll / Cancel"** (lGcC @238.7; top-left
  "Transform Howl scroll / 1 more options" HemC @172, QlQ_ @229, JQZn @335). **No Infuse-5 / Infuse-10 / Infuse-X /
  Infuse-All and no Transform-5 etc. in any recording from 2009-09 to 2011-10** (the quantity options belong to
  the later version).
- One click on Infuse makes **both** pouches (inventory goes from 2 empty pouches + charms + shards + bones to
  2 spirit wolf pouches: HemC @169, JQZn @343) and closes the window. Animation: the player kneels/bends toward the
  obelisk while the obelisk is wrapped in bright cyan lightning-like lines for about 2 s (JQZn @320.5-321.5).
  Messages (12 of 17; lGcC @233, 2EfU @168, JQZn @322-330, read by eye):
  **"You infuse a spirit wolf pouch."** then **"You now need to transform a spirit wolf pouch into some Howl scrolls."**
- Transform (one click) turns one pouch into a stack of **10** Howl scrolls (stack number 10: HemC @174, JQZn @345);
  same kneel animation, obelisk glow weaker (JQZn @344-345). Messages (12 of 17; HemC @174, 2EfU @178, JQZn @352):
  **"You transform a spirit wolf pouch into some Howl scrolls."** then
  **"You should speak to Pikkupstix. He will tell you what to do with the scrolls and the ⏎ spirit wolf pouch."**
- **Renew-points** at the end stage (12 of 17; HemC @245, 2EfU @310, JQZn @468): player kneels at the obelisk (JQZn @467),
  messages **"You have recharged your Summoning skill points."** then **"You should speak to Pikkupstix to get your reward."**
- Other obelisk messages seen:
  - "You already have full Summoning points." (HemC @168, ldSY @327: Renew-points with full points).
  - "You do not have the required Summoning knowledge to use this obelisk yet." (2EfU @306, ldSY @323; appears at the
    renew stage just before the recharge line; the click that caused it is off-screen — most likely Infuse-pouch
    during the quest's last stage. Cause **not confirmed**).
  - "You are standing in a Summoning-free area. You must move out of this area to ⏎ summon a familiar." (HemC @168,
    three times: trying Summon in the cave) and "Nothing interesting happens." (HemC @168, cause off-screen).
  - A tooltip fragment "You do not have access to this" appears over greyed slots (8 recordings, OCR only, rest unreadable).

## 6. Summoning the wolf and casting Howl

- Pouch item options: **Use / Summon / Destroy / Examine / Cancel** (HemC @203, JQZn @406). Scroll item options:
  **Use / Destroy / Examine / Cancel** (HemC @174).
- On Summon (in the attic): the wolf appears next to the player inside a ring of pale blue sparkles/shards rising from
  the floor (HemC @204, JQZn @408-410). Chat (12 of 17; HemC @204, 2EfU @246, JQZn @415):
  **"You have summoned the spirit wolf! Now you need to use the spirit wolf's special ⏎ move - Howl - on the giant wolpertinger to banish it."**
- **The side panel switches by itself to the Follower Details tab** (all years: HemC @204 Nov 2009, 2EfU @237 and
  JQZn @408 May 2011). Contents top to bottom: blue bar "S P E C I A L   M O V E"; left a Howl-scroll button with the
  scroll count "10" under it (tooltip "Scrolls remaining" HemC @209; hover text "Level 1: Howl (3 Special Move points)" /
  "Makes NPCs flee for a short time" with scroll icon "10/1", HemC @206); centre the spirit wolf's head; right a
  claw/paw icon; name plate "Spirit wolf"; below, two rows of buttons (attack/call/dismiss style icons, one with a
  red "no" circle; tooltips seen: "Renew familiar" HemC @205, "Dismiss ..." HemC @210, QlQ_ @289) and at the right an
  hourglass with **"6.00"** (time left) and a small wolf icon with "1/1" (HemC @206, JQZn @410). The special-move bar
  tooltip reads **"60/60 special move points remaining"** (HemC @208, JQZn @410). This is the Follower Details panel,
  not the older plain Summoning tab; no recording in the set shows any other tab opening.
- The summoning orb by the minimap (bottom-right of the four orbs): before, a green number (HemC "1" level) with the
  wolf-head icon dim; while the familiar is out the icon is lit; the tab row gains the brown familiar-head tab icon
  (HemC @204-213). Exact orb states beyond that are too small to read.
- Spirit wolf npc options: **Interact / Walk here / Examine** (JQZn @408.5). Interacting during this stage:
  **"The wolf is growling at the giant wolpertinger, and will not reply to you."** (HemC @212, 1 of 17).
- Attack on the wolpertinger: **"You can't attack this npc."** (HemC @211). Wolpertinger options: **Examine** only
  (2EfU @82.5 "Examine Giant wolpertinger").
- Clicking the scroll item itself: **"These scrolls can only be used by clicking the Special Move button in the Follower ⏎ Details tab."** (HemC @212-213, 1 of 17).
- Casting Howl on the wolpertinger: **"The spirit wolf howls at the giant wolpertinger."** (12 of 17; HemC @214,
  2EfU @251, JQZn @415), then cutscene 3 at once. Scroll count afterwards: 9 (HemC @263 inventory shows 9 scrolls).

## 7. Quest complete

- Scroll (14 of 17; HemC @262, 2EfU @336-341, JQZn @500-504; read by eye in all three, identical):
  ```
  Congratulations!
  You have completed Wolf Whistle!
  You are awarded:
  1 Quest Point.
  276 Summoning XP.
  Access to the Summoning
  skill.
  275 gold charms.
  Quest Points:    <total>
  ```
  ("Access to the Summoning skill." wraps onto two lines.) **Item model shown: a Spirit wolf pouch** (white pouch with
  the blue-eared wolf head), left of the text.
- Chat lines, in order (HemC @266, 2EfU @346-353, JQZn @508-528):
  "Congratulations! Quest complete!" then the level-up line(s):
  2011 "You've just advanced 3 Summoning levels! You have reached level 4." and (if it applies)
  "Congratulations! You've just reached combat level 8!" (2EfU; "combat level 64" JQZn; 2009 HemC:
  "Congratulations! You've just reached Combat level 56!" with capital C).
- Inventory after: 275 gold charms as one stack, the 9 Howl scrolls kept (HemC @263, 2EfU @341, JQZn @504).
- Level-up, 2011 (JQZn @512-524, QlQ_ @398, oqiS @359 Aug 2010 already): parchment window "Congratulations!" /
  "You've just advanced 3 Summoning levels!" / "You have now reached level 4." with a scrolling unlock list:
  "You can now infuse dreadfowl pouches." "You can now summon meerkats." "You can now use Dreadfowl Strike scrolls."
  "You can now use Fetch Casket scrolls." "You can now raise bulldogs." ... "You can now raise terriers."
  "You can now keep creeping hands." "You can now keep minitrices." "You can now keep baby basilisks."
  "You can now keep baby kurasks." (also greyhounds, dalmatians, labradors, sheepdogs, abyssal minions,
  "rune guardians (after Rune ...", "cub deathslinger pouches within ..." by OCR only) and a link
  "Open Skill Advance Guide". This is the generic level-up window, not quest text.

## 8. Quest journal

Only one stage is opened, and only its lower part (HemC @91-94 read by eye; EwNM @96-99 same text, tiny). Title
"Wolf Whistle". Struck-through (done) paragraph, first line partly hidden by the scroll edge:

> ~~It appears that I have underestimated the rabbit in this case; it is some huge rabbit-wolf-monster-bird-thing. I think I should speak to Pikkupstix to find out what is going on.~~

Current paragraph (blue text, red highlights on Pikkupstix, Summoning, giant wolpertinger, 2, wolf bones):

> I have spoken to Pikkupstix, who has promised to teach me the secrets of Summoning if I can help dismiss the giant wolpertinger. To do this, I need to bring him 2 lots of wolf bones.
> I need to get 2 lots of wolf bones.

("I need to get 2 lots of wolf bones." is on its own line, red.) Anything above the struck paragraph, and every other
stage including the completed journal: **not shown**.

## 9. Destroy / drop texts

- Options only: Spirit wolf pouch has **Destroy** (HemC @169, JQZn @328, 9 recordings); Howl scroll has **Destroy**
  (HemC @174); Trapdoor key has **Destroy** (WKPg @213.5, OCR "Destroy trapdoor key", option list only);
  Wolf bones **Drop** (HemC @129, WKPg @183); Gold charm **Drop** (JQZn @496).
- **No recording opens a destroy confirmation**, so the destroy texts are not sourced. Empty pouch and spirit shard
  options are not shown.

## 10. Item hand-out

There is **no item box (objbox) anywhere in the quest**. At e03 ("Here are the pouches, spirit shards and charms you
will ⏎ need to make the spirit wolf pouch and Howl scrolls. ⏎ This key is to the trapdoor over there, which leads to ⏎
the obelisk.") the items are simply in the inventory afterwards: 2 pouches, 2 gold charms, a key, a stack of shards
(HemC @162 inventory: two charm icons, key, two pouches, shards "14"; JQZn @318). Whether the 2 wolf bones are taken
and handed back is not visible (they are still in the inventory after the talk, HemC @162). The 275 gold charms at the
end also arrive without a box (h15 → h16 → scroll).

## 11. Not shown by any recording ("No source found")

- "Talk about Pikkenmix." branch; anything about the shop, enchanting headgear, or talking after completion.
- Re-talk lines at intermediate stages (no bones yet; has bones but talked before cutscene 1; lost the key / pouches /
  charms / shards / scrolls; has pouch but no scrolls; talking again before renewing points).
- Climbing the ladder: before the first talk, between cutscene 1 and having both items, with a familiar already out,
  after the quest.
- Starting without the requirement / as a free player.
- Searching the dead body again after taking bones (none held), or before starting the quest.
- How many wolf bones the body gives.
- Trapdoor Close/second-Open messages; using the wrong item on it.
- Destroy confirmation text for key, pouches, scrolls; drop behaviour of shards and charms.
- Obelisk Infuse with ingredients missing; "Use pouch on obelisk"; Transform with no pouch.
- Summoning the wolf outside the attic at the right stage (only the cave's "Summoning-free area" line is seen),
  dismissing it, or letting its timer run out; casting Howl on anything else.
- Quest journal: start text, all stages except the "get 2 lots of wolf bones" one, and the completed text.
- Chathead animation ids; music track names other than "Animal Apogee" (cave).
- The quest-start confirmation interface (none appears in any recording: the quest starts inside the b-dialogue;
  2011 recordings show no accept/decline window either).
