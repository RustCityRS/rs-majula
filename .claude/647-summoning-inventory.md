# Summoning (internal name: Animal Lore) inventory for rev 647

Stage 0 of the Summoning work: everything cache 786 holds for the skill, identified before any build work. Stat 23 `animallore`; cache prefixes `lore_*` (skill), `wolf_*` (Wolf Whistle and obelisks), `rand_lore*` (Daemonheim), `sc_lore*` (Stealing Creation), `sfa_*` (Familiarisation).

- Sources: `content/647/pack/*.pack`, `content/647/scripts/_unpack/all.*`, `content/647/interfaces/*.if3`, `content/647/maps/*.jm2`, client scripts under `content/647/scripts/**/client/`.
- The tables under "Generated tables" are produced by `python3 .claude/647-summoning-inventory.py`; rerun it after a cache change and replace that section.
- 2011 cross-checks use the wiki mirror frozen at 23 May 2011 (`http://138.197.104.72:8090/wiki/Summoning`).
- Nothing here is server behaviour yet. Today the server only opens the orb at login (`login_logout/scripts/login.rs2:94-125`).

## 1. Interfaces

| Id | Name | Role | Opens in |
|---|---|---|---|
| 747 | `topstat_lore` | Summoning orb | orb slot 548:187 (fixed) / 746:177 (resizable) |
| 662 | `lore_stats_side` | Follower Details: the interface shown when a familiar is summoned | side tab 95: 548:220 / 746:104 |
| 663 | `lore_cats_side` | cat details (Call Cat, Dismiss Cat / Dismiss Now) | side tab (slot unconfirmed) |
| 880 | `lore_topstat_confirmop` | chooser for the orb's left-click option | side-tab sized; slot not in cache |
| 79 | `lore_pouch_creation_combined` | pouch creation + scroll transformation, tabbed | main |
| 672 / 666 | `interface_672` / `interface_666` | Wolf Whistle versions of pouch / scroll creation | main |
| 78 | `interface_78` | Bogrog trade-in | main |
| 671 + 665 | `lore_bank` + `lore_bank_side` | beast of burden ("Familiar Inventory") | main + side modal |
| 668 | `lore_buydog` | "Pick a Puppy" | main |
| 878 / 879 | `poh_pet_house_side` / `_main` | POH pet house | side / main |

Summoning pieces on other interfaces: bank 762:37 `bank_bob_button` and deposit box 11:22 ("Deposit beast of burden inventory"); equipment 667:40 `summoningdef`; stats 320:143-150 (150 has View / Set Level Target / Set XP Target / Clear Target); XP lamp 134:53; level-up 740/741; clan wars 791:132; duel 631:65, 641/635.

Not Summoning despite the name: 722 `summoning_side` (Grand Exchange), 775/776 `sum1_spirit_overlay` (Summer's End), 716 (Easter 2008 clone of 662).

### 1.1 The orb (747)

| Comp | Name | Op | Shown |
|---|---|---|---|
| 7 | `select_leftclick_layer` | op10 "Select left-click option" | always |
| 9 / 18 | `details_layer` / `details_op1` | "Follower Details" (client-side switch to tab 95) | op6 / left-click twin |
| 10 / 19 | `call_layer` / `call_op1` | "Call Follower" | |
| 11 / 20 | `dismiss_layer` / `dismiss_op1` | "Dismiss" | |
| 12 / 21 | `bob_layer` / `bob_op1` | "Take BoB" | |
| 13 / 22 | `renew_layer` / `renew_op1` | "Renew Familiar" | |
| 14 / 23 | `attack_layer` / `attack_op1` | target verb "Attack" (npc / player) | |
| 15 / 26 | `interact_layer` / `interact_op1` | "Interact" | |
| 17 | `spell_icon` | child 0 built by script606: target verb "Cast", plus op1 "Cast" when varc 1436 = 1; label is varcstr 204 | per familiar (enum 1092) |
| 24 / 25 | `nullspell` / `lore_nullspell` | "Spell" → "Cast" | familiars without a special icon |
| 2-6 | background, fill, drain, text, icon | | members world and varbit 4280 = 1 |
| 8 | `main_ops` | parent of the action layers | only while side tab 95 holds an interface and 4280 = 1 |

- Right-click menu with no follower: "Select left-click option" only.
- With a follower: the left-click option, Cast, Interact, Attack, Renew Familiar, Take BoB, Dismiss, Call Follower, Follower Details, Select left-click option. The order is inferred from component ids.
- Left-click option = varbit 6454 (varp 1493 bits 0-3): 0 details, 1 special move, 2 attack, 3 call, 4 dismiss, 5 take BoB, 6 renew, 7 interact. `script2671` shows the matching `*_op1` twin.
- The orb is tab 95's icon: tab 95 has no stone of its own.
- Clientscript 2471 (server-invoked) hides the action layers again on dismiss.

### 1.2 Follower Details (662)

| Comp | Name | Content / op | Driven by |
|---|---|---|---|
| 1 | `lore_chathead` | familiar head | varp 1174, varbit 4282 (`lore_head_update`) |
| 54 | `lore_name` | familiar name | enum 1279 |
| 41 | `points_left` | summoning points | stat `animallore` |
| 43 | `animal_time_left` | time left, "M.00" / "M.30" | varp 1176: varbit 4534 minutes, 4290 half-minute |
| 40 | `lore_magic` | "S P E C I A L   M O V E" bar (20 steps) | varp 1177 (0-60) |
| 66 | `lore_pouch_counter` | scrolls carried | inv + enum 1283 |
| 74 | `spell_icon` | special move, tooltip cost from varbit 4288 | varp 448, varcstr 205 |
| 49 | `call_animal_button` | op1 "Call Follower" | |
| 51 | `dismiss_button` | op1 "Dismiss Familiar", op2 "Dismiss Now" | |
| 65 | `attack_button` | target verb "Attack" | |
| 67 | `bobdump` | op1 "Take BoB" | |
| 69 | `lore_renew_button` | op1 "Renew Familiar" | |
| 71 / 72 | `lore_pets` / `lore_familairs` | pet layer (size 4285, hunger 4286) / familiar layer | toggled by `lore_head_update` |

### 1.3 Left-click chooser (880)

Radios write varbit 6455 (varp 1494 bits 0-3): 7/8 Follower details (0), 9/10 Special move (1), 11/12 Attack (2), 13/14 Call follower (3), 15/16 Dismiss follower (4), 17/18 Take BoB (5), 19/20 Renew familiar (6), 25/26 Interact (7). 880:21 "Confirm Selection" (the server copies 6455 to 6454; inferred).

### 1.4 Creation, Bogrog, beast of burden

| Interface | Tabs | Grids | Populated by (server-invoked clientscript) |
|---|---|---|---|
| 79 | 79:20 "Transform Scrolls", 79:22 "Infuse Pouches" | 79:16 `lore_pouch_layer`, 79:17 `lore_scroll_layer` | 757 `lore_interface_inv_init`, 763 `lore_interface_magic_inv_init` |
| 672 | 672:19 | 672:16 | 791 (quest; uses `wolf_pouch_checker`) |
| 666 | 666:18 | 666:16 | 796 (quest; uses `wolf_magic_pouch_checker`) |
| 78 | 78:20 "Trade Scrolls", 78:24 "Trade Pouches" | 78:14, 78:15 | 317, 320 |
| 671 / 665 | 671:29 "Take BoB" | 671:27, 665:0 `lore_bob_layer` (inv 530) | generic `interface_inv_init_big` (150) |

Grid ops are passed in as strings by the server (op6 is always "Examine"). Each grid slot makes 5 children; the clickable one is `slot * 5 + 2`.

## 2. Vars

| Var | Name | Bits / use |
|---|---|---|
| varp 276 | `follower_uid` | temp, npc uid of the follower |
| varp 448 | `follower_obj` | pouch obj of the current familiar |
| varp 1174 | `lore_npc_head` | chathead npc |
| varp 1175 | `lore_npc_stats` | 4285 size % (1-8), 4286 hunger % (9-16), 4287 (17-22), 4288 special cost (23-27), 6282 (28-31) |
| varp 1176 | `lore_timer_var` | 4289 (1-5), 4290 half-minute (6), 4534 minutes (7-31) |
| varp 1177 | `lore_spell_points` | special-move points 0-60 |
| varp 1160 | `lore_incubators_var` | 4277/4278 incubator 1, 4221/4279 incubator 2, 4280 orb available (23), 4281, 4282 chathead anim (25-31) |
| varp 1171 | `lore_general_var_1` | 4291 `lore_farming`, 4292, 4294-4297 pouch returns, 4298, 4299 |
| varp 1493 / 1494 | `lore_general_varp_1` / unnamed | 6454 left-click option / 6455 pending choice |
| varp 1178 | `wolf_var` | 4305 chat skip (0), 4302 `wolf_quest` (1-10), 4303 `wolf_trapdoor` (11-12), 4304 `wolf_items` (13), 4306 `wolf_bones_taken` (14-15) |
| varc 1436 | `lore_spell_op_binary` | 1 adds a click "Cast" op to the special icon |
| varc 1266 / 1492 | `xpdisplay_summoning` / `lvl_summoning_before_levelup_varc` | XP drop / level-up details |
| varcstr 204 / 205 | `varclient_1723` / `varclient_1724` | special-move name / description |

Summoning points are the current level of stat `animallore`.

## 3. Params, enums, inventories

| Param | Meaning | Used on |
|---|---|---|
| 394 | Summoning level required | 452 objs, 482 npcs |
| 538/539, 540/541, 542/543 | empty pouch, shards, charm + counts | 78 pouches |
| 697/698, 699/700 | tertiary 1, 2 + counts (701-708 defined, unused) | pouches |
| 457 | shards returned by Bogrog | pouches, some scrolls |
| 599 | scrolls needed per Bogrog swap | scrolls without 457 |
| 764 / 765 | quest gate: 131 Wolf Whistle / 146 In Pyre Need | pouches, scrolls |
| 770 / 771 | skill gate (stat, level) | pouches, scrolls |
| 1323 | beast-of-burden marker | 12 npcs |

| Enum | Maps |
|---|---|
| 1182 / 1277 | slot → pouch (creation screen / scroll screen) |
| 1188 | slot → scroll |
| 1183 / 1184 | slot → greyed pouch / greyed scroll |
| 1185 | pouch → level |
| 1186 | pouch → "This pouch requires …" text |
| 1187 | pouch or scroll → name |
| 1283 | pouch → scroll |
| 1320 | pouch → familiar npc |
| 1278 / 1279 | index / npc → familiar name |
| 1092 / 1282 | npc → orb / side-panel component |
| 1275 / 1276 | chathead animations |
| 1472 | Bogrog swap level |

Inventories: 530 `lore_bob` (30), 531 `lore_pet_shop_supplies` (5), 532 `lore_summoning_supplies` (10), 552 `poh_pethouse` (15). The cache holds no stock for the two shops.

## 4. Other objs

- Charms: 12158 gold `lore_creature_charm`, 12159 green `lore_insect_charm`, 12160 crimson `lore_leech_charm`, 12163 blue `lore_outsider_charm`, 12161 abyssal, 12162 talon beast, 12164-12167 ravager / shifter / spinner / torcher, 12168 obsidian.
- Basics: 12155 `lore_pouch`, 12183 `lore_spirit_shard`, 15262 shard pack.
- Greyed placeholders: 12225-12292, 12293-12421, 14620, 14625.
- Ingredients that exist only for Summoning: cockatrice eggs 12109-12121 (odd), evil turnip 12134-12153, honeycomb 12156, spirit weed 12172-12181, phoenix quill 14616.
- Potions 12140-12146 (even). Capes 12169-12171; spirit cape 19893.
- Enchanted headgear ("(e)" / charged, Commune / Uncharge): 12658-12681 (12 pairs), slayer helms 14636/14637 and 15496/15497; headdresses 12204-12224 (antlers, lizard skull, five feathered).
- Pets: inventory forms 12184-12203; eggs 11964, 11965, 12477-12480, 12483, 12494, 12495; food 12125-12132; growth stages 12469-12523 and 12682-12775; plus platypus 12547-12557, parrot 13335/13336, broav 14533, egglings 14626/14627, slayer pets 14656-14660, rune guardians 15483 and 15626-15639, tooth 18668-18674, sneakerpeeper 19894/19895.
- Wolf Whistle: 12525 pouch, 12526 spirit wolf pouch, 12527 gold charm, 12528 trapdoor key, 12529 Howl scroll, 12530 spirit shards (all Destroy, no destroy text in cache).

## 5. Other NPCs and locs

- Pikkupstix: 6988 shows 6971 (worried; Talk-to) while `wolf_quest` is 0-100; 6989 shows 6970 (calm; Talk-to, Trade, Enchant) at 110.
- Bogrog 4472 (Talk-To, Trade, Swap). Pet shop owners 6892 Yanille, 6893 Taverley (Talk-to, Trade, Sell-shards). Giant wolpertinger 6990 has no map spawn.
- Right-click abilities on familiar NPCs (op5): Special (dreadfowl), Strike (void torcher), Drain (cockatrices), Drown (karamthulhu), Flames (smoke devil), Cannon (barker toad), Fireball (forge regent), Cure (unicorn), Ash-blast (phoenix); op3 Burrow (desert wyrm), Store (beasts of burden), Withdraw (foragers).
- Each charged obelisk site also has two inert obelisks (28717/28718 … 28735/28736), an `obelisk_icon` and `bgsound_wolf_obelisk`. All obelisks animate with `seq_8510`.
- Underground access: Pollnivneach ladder 28737 (3304,2897) / 28740; Piscatoris trapdoor 28741 (2328,3645) / ladder 28743. No multiloc wraps the Piscatoris trapdoor.
- Wolf Whistle: ladder 28572 (2925,3442) / top 28653; trapdoor multiloc 4039 at 2927,3444 (28675 closed, 28676 open, on `wolf_trapdoor`); basement ladder 28714 (2209,5349); dead body 28586 (2853,3473).
- Incubators: 28550 Taverley (2927,3435), 28352 Yanille (2581,3079); state 0 shows 28336, 1-23 shows 28359 (Take-egg, Inspect).
- Others: evil turnip patch 23760 (3086,3367); POH menagerie hotspot 44911; Stealing Creation table 39545; Familiarisation obelisk 48190.

## 6. Cross-checks against the 23 May 2011 wiki

- Charged obelisks: 7 sites in both. The Nature Grotto obelisk is placed on levels 0 and 1 of the same square.
- Small obelisks: 59 placements in the cache, 52 on the wiki (its list is dated 7 Sep 2009).
- The cache confirms the wiki values that look wrong: 1 shard for spirit mosquito, war tortoise, spirit dagannoth and rune minotaur; green charm for void ravager while the other void familiars use blue.
- Stranger plant: pouch says level 64 (wiki agrees), npc param 394 says 80.
- Other npc param 394 oddities: 6882 and 7371 say 4 where their non-combat forms say 43 and 34; beaver 6808 carries both 22 and 33.
- Six familiars define `op1` twice (talk-to, then Interact): 6798/6799, 6837, 6849/6850, 6867/6868, 7372/7373.

## 7. Identification items

### Closed

- **Level value the client uses.** The client reads `param_394` only from the pouch obj (`oc_param(%follower_obj, param_394)` in `lore/client/[clientscript,lore_head_update].cs2:41` and `unpack/client/[proc,script606].cs2:11`). No client script reads it from an npc, so the npc oddities in section 6 are never shown; the server should take the level from the pouch.
- **Pikkupstix's shop (inv 532)**, wiki "Pikkupstix" as of 23 May 2011: Pouch 5,000 at 1 coin; Spirit shards 65,000 at 25; Antlers 10 at 100; Lizard skull 10 at 200; Summoning cape 99,000 coins at level 99 (sold through him, not a shop row). The wiki also says shops carry 65,000 shard packs. Stock is per player since 2 Sep 2009. He enchants helms for free (the Enchant op).
- **Small obelisks.** The wiki's 52-entry list is dated 7 Sep 2009; the cache is May 2011 and decides. All 59 placements carry Renew-points, so all 59 work. Wiki entries by region for reference: Ape Atoll 1, Asgarnia 6, Dwarven Realm 2, Fremennik 5 (+2 Waterbirth), Karamja 5, Kharidian Desert 6, Kandarin 9, Miscellania 1, Misthalin 5, Morytania 3, Tirannwn 2, Troll Country 1, Feldip Hills 3, Mos Le'Harmless 1, Wilderness 2, Zanaris 1.
- **Summoning shops.** The wiki names three: Pikkupstix's house, Bogrog's shop, and the Wishing Well at the Nature Grotto (loc 28715 `wolf_wishing_well`, Make-wish), which is why the grotto has a shop icon and no shopkeeper.

### Needs a familiar in the client (closed during the lifecycle phase)

1. Exact right-click order on the orb, and whether Take BoB / Renew show for familiars that are not beasts of burden, and for pets.
2. Where 880 and 663 open.
3. Whether May 2011 used 79 or 672/666 outside the quest (the cache points to 79; 672/666 are the only ones the quest's client scripts can fill).
4. Pet shop stock (inv 531): the mirror's "Pet shop" page is an empty redirect.

## Generated tables

### Standard pouches (78)

| Lvl | Pouch obj | Name | Shards | Charm | Tertiary | Bogrog | Scroll | Familiar npc | Combat npc |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 12047 `lore_wolf` | Spirit wolf pouch | 7 | `lore_creature_charm` | `wolf_bones` | 5 | 12425 `lore_howl` | 6829 `lore_wolf` | 6830 (cb 26) |
| 4 | 12043 `lore_dreadfowl` | Dreadfowl pouch | 8 | `lore_creature_charm` | `raw_chicken` | 6 | 12445 `lore_dreadfowl_strike` | 6825 `lore_dreadfowl` | 6826 (cb 26) |
| 10 | 12059 `lore_spider` | Spirit spider pouch | 8 | `lore_creature_charm` | `village_spider_carcass` | 6 | 12428 `lore_egg_spawn` | 6841 `lore_spider` | 6842 (cb 25) |
| 13 | 12019 `lore_snail` | Thorny snail pouch | 9 | `lore_creature_charm` | `snail_corpse1` | 7 | 12459 `lore_slime_trail` | 6806 `lore_snail` | 6807 (cb 26) |
| 16 | 12009 `lore_rock_crab` | Granite crab pouch | 7 | `lore_creature_charm` | `iron_ore` | 5 | 12533 `lore_stony_shell` | 6796 `lore_rock_crab` | 6797 (cb 26) |
| 17 | 12778 `lore_mosquito` | Spirit mosquito pouch | 1 | `lore_creature_charm` | `mosquito_proboscis` | 1 | 12838 `lore_mosquito_attack` | 7331 `lore_mosquito` | 7332 (cb 32) |
| 18 | 12049 `lore_desert_worm` | Desert wyrm pouch | 45 | `lore_insect_charm` | `bucket_sand` | 32 | 12460 `lore_electric_lash` | 6831 `lore_desert_worm` | 6832 (cb 31) |
| 19 | 12055 `lore_scorpion` | Spirit scorpion pouch | 57 | `lore_leech_charm` | `bronze_claws` | 40 | 12432 `lore_envenom_weapon` | 6837 `spirit_scorpion` | 6838 (cb 51) |
| 22 | 12808 `lore_tz_kih` | Spirit tz-kih pouch | 64 | `lore_leech_charm` | `lore_abby_titan_token` | 45 | 12839 `lore_lava_assault` | 7361 `lore_tz_kih` | 7362 (cb 36) |
| 23 | 12067 `lore_giant_rat` | Albino rat pouch | 75 | `lore_outsider_charm` | `raw_rat_meat` | 53 | 12430 `lore_cheese_feast` | 6847 `lore_giant_rat` | 6848 (cb 37) |
| 25 | 12063 `lore_kalphite` | Spirit kalphite pouch | 51 | `lore_outsider_charm` | `cactus_potato` | 36 | 12446 `lore_sandstorm` | 6994 `spirit_kalphite` | 6995 (cb 39) |
| 28 | 12091 `lore_compost_mound` | Compost mound pouch | 47 | `lore_insect_charm` | `bucket_compost` | 33 | 12440 `lore_generate_compost` | 6871 `lore_compost_mound` | 6872 (cb 37) |
| 29 | 12800 `lore_chinchompa` | Giant chinchompa pouch | 84 | `lore_outsider_charm` | `chinchompa_captured` | 59 | 12834 `lore_explode` | 7353 `lore_chinchompa` | 7354 (cb 42) |
| 31 | 12053 `lore_vampire_bat` | Vampire bat pouch | 81 | `lore_leech_charm` | `vampire_dust` | 57 | 12447 `lore_vampire_touch` | 6835 `vampire_bat` | 6836 (cb 44) |
| 32 | 12065 `lore_honey_badger` | Honey badger pouch | 84 | `lore_leech_charm` | `lore_honeycomb` | 59 | 12433 `lore_insane_ferocity` | 6845 `lore_honey_badger` | 6846 (cb 45) |
| 33 | 12021 `lore_beaver` | Beaver pouch | 72 | `lore_insect_charm` | `willow_logs` | 51 | 12429 `lore_multichop` | 6808 `lore_beaver` | none |
| 34 | 12780 `lore_spinner` | Void spinner pouch | 74 | `lore_outsider_charm` | `lore_pest_spinner_token` | 52 | 12443 `lore_call_to_arms` | 7333 `lore_spinner` | 7334 (cb 40) |
| 34 | 12798 `lore_torcher` | Void torcher pouch | 74 | `lore_outsider_charm` | `lore_pest_torcher_token` | 52 | 12443 `lore_call_to_arms` | 7351 `lore_torcher` | 7352 (cb 46) |
| 34 | 12814 `lore_shifter` | Void shifter pouch | 74 | `lore_outsider_charm` | `lore_pest_shifter_token` | 52 | 12443 `lore_call_to_arms` | 7367 `lore_shifter` | 7368 (cb 46) |
| 34 | 12818 `lore_ravager` | Void ravager pouch | 74 | `lore_insect_charm` | `lore_pest_ravager_token` | 52 | 12443 `lore_call_to_arms` | 7370 `lore_ravager` | 7371 (cb 46) |
| 36 | 12073 `lore_minotaur` | Bronze minotaur pouch | 102 | `lore_outsider_charm` | `bronze_bar` | 72 | 12461 `lore_bull_rush` | 6853 `lore_minotaur` | 6854 (cb 50) |
| 40 | 12087 `lore_giant_ant` | Bull ant pouch | 11 | `lore_creature_charm` | `marigold` | 8 | 12431 `lore_unburden` | 6867 `lore_giant_ant` | 6868 (cb 58) |
| 41 | 12071 `lore_macaw` | Macaw pouch | 78 | `lore_insect_charm` | `guam_leaf` | 55 | 12422 `lore_herbcall` | 6851 `lore_macaw` | none |
| 42 | 12051 `lore_turnip` | Evil turnip pouch | 104 | `lore_leech_charm` | `lore_evil_turnip_face` | 73 | 12448 `lore_evil_flames` | 6833 `lore_turnip` | 6834 (cb 62) |
| 43 | 12095 `lore_cockatrice` | Sp. cockatrice pouch | 88 | `lore_insect_charm` | `lore_cockatrice_egg_1` | 62 | 12458 `lore_petrify` | 6875 `lore_cockatrice` | 6876 (cb 64) |
| 43 | 12097 `lore_cockatrice_2` | Sp. guthatrice pouch | 88 | `lore_insect_charm` | `lore_cockatrice_egg_2` | 62 | 12458 `lore_petrify` | 6877 `lore_cockatrice_2` | 6878 (cb 64) |
| 43 | 12099 `lore_cockatrice_3` | Sp. saratrice pouch | 88 | `lore_insect_charm` | `lore_cockatrice_egg_3` | 62 | 12458 `lore_petrify` | 6879 `lore_cockatrice_3` | 6880 (cb 64) |
| 43 | 12101 `lore_cockatrice_4` | Sp. zamatrice pouch | 88 | `lore_insect_charm` | `lore_cockatrice_egg_4` | 62 | 12458 `lore_petrify` | 6881 `lore_cockatrice_4` | 6882 (cb 64) |
| 43 | 12103 `lore_cockatrice_5` | Sp. pengatrice pouch | 88 | `lore_insect_charm` | `lore_cockatrice_egg_5` | 62 | 12458 `lore_petrify` | 6883 `lore_cockatrice_5` | 6884 (cb 64) |
| 43 | 12105 `lore_cockatrice_6` | Sp. coraxatrice pouch | 88 | `lore_insect_charm` | `lore_cockatrice_egg_6` | 62 | 12458 `lore_petrify` | 6885 `lore_cockatrice_6` | 6886 (cb 64) |
| 43 | 12107 `lore_cockatrice_7` | Sp. vulatrice pouch | 88 | `lore_insect_charm` | `lore_cockatrice_egg_7` | 62 | 12458 `lore_petrify` | 6887 `lore_cockatrice_7` | 6888 (cb 64) |
| 46 | 12075 `lore_minotaur_2` | Iron minotaur pouch | 125 | `lore_outsider_charm` | `iron_bar` | 88 | 12462 `lore_bull_rush_2` | 6855 `lore_minotaur_2` | 6856 (cb 70) |
| 46 | 12816 `lore_pyrefiend` | Pyrelord pouch | 111 | `lore_leech_charm` | `tinderbox` | 78 | 12829 `lore_immense_heat` | 7377 `pyrelord` | 7378 (cb 70) |
| 47 | 12041 `lore_magpie` | Magpie pouch | 88 | `lore_insect_charm` | `gold_ring` | 62 | 12426 `lore_thieving_fingers` | 6824 `lore_magpie` | none |
| 49 | 12061 `lore_leech` | Bloated leech pouch | 117 | `lore_leech_charm` | `raw_beef` | 82 | 12444 `lore_blood_drain` | 6843 `lore_leech` | 6844 (cb 76) |
| 52 | 12007 `lore_terrorbird` | Spirit terrorbird pouch | 12 | `lore_creature_charm` | `spit_raw_bird_meat` | 9 | 12441 `lore_tireless_run` | 6794 `lore_terrorbird` | 6795 (cb 62) |
| 54 | 12035 `lore_abyssal_leech` | Abyssal parasite pouch | 106 | `lore_insect_charm` | `lore_monster_charm` | 75 | 12454 `lore_abyssal_drain` | 6818 `lore_abyssal_leech` | 6819 (cb 86) |
| 55 | 12027 `lore_jelly` | Spirit jelly pouch | 151 | `lore_outsider_charm` | `jug_water` | 106 | 12453 `lore_dissolve` | 6992 `spirit_jelly` | 6993 (cb 88) |
| 56 | 12077 `lore_minotaur_3` | Steel minotaur pouch | 141 | `lore_outsider_charm` | `steel_bar` | 99 | 12463 `lore_bull_rush_3` | 6857 `lore_minotaur_3` | 6858 (cb 90) |
| 56 | 12531 `lore_stork` | Ibis pouch | 109 | `lore_insect_charm` | `harpoon` | 77 | 12424 `lore_fish_rain` | 6991 `lore_stork` | none |
| 57 | 12784 `lore_larupita` | Spirit larupia pouch | 155 | `lore_outsider_charm` | `hunting_fur_jaguar_perfect` | 109 | 12840 `lore_rending` | 7337 `lore_larupita` | 7338 (cb 93) |
| 57 | 12810 `lore_graahk` | Spirit graahk pouch | 154 | `lore_outsider_charm` | `hunting_fur_leopard_perfect` | 108 | 12835 `lore_goad` | 7363 `spirit_graahk` | 7364 (cb 93) |
| 57 | 12812 `lore_kyatt` | Spirit kyatt pouch | 153 | `lore_outsider_charm` | `hunting_fur_tiger_perfect` | 108 | 12836 `lore_ambush` | 7365 `lore_kyatt` | 7366 (cb 93) |
| 58 | 12023 `lore_karamthulhu` | Karam. overlord pouch | 144 | `lore_outsider_charm` | `fishbowl_empty` | 101 | 12455 `lore_overwhelming_waters` | 6809 `lore_karamthulhu` | 6810 (cb 95) |
| 61 | 12085 `lore_dust_devil` | Smoke devil pouch | 141 | `lore_leech_charm` | `ground_desert_goat_horn` | 99 | 12468 `lore_dust_cloud` | 6865 `lore_dust_devil` | 6866 (cb 101) |
| 62 | 12037 `lore_abyssal_lurker` | Abyssal lurker pouch | 119 | `lore_insect_charm` | `lore_monster_charm` | 84 | 12427 `lore_abyssal_stealth` | 6820 `lore_abyssal_lurker` | 6821 (cb 93) |
| 63 | 12015 `lore_snake` | Spirit cobra pouch | 116 | `lore_leech_charm` | `village_snake_hide` | 82 | 12436 `lore_ophedian_incubation` | 6802 `lore_snake` | 6803 (cb 105) |
| 64 | 12045 `lore_stranger_plant` | Stranger plant pouch | 128 | `lore_leech_charm` | `poh_sapling_plant_1` | 90 | 12467 `lore_poisonous_blast` | 6827 `lore_stranger_plant` | 6828 (cb 107) |
| 66 | 12079 `lore_minotaur_4` | Mithril minotaur pouch | 152 | `lore_outsider_charm` | `mithril_bar` | 107 | 12464 `lore_bull_rush_4` | 6859 `lore_minotaur_4` | 6860 (cb 112) |
| 66 | 12123 `lore_barker_toad` | Barker toad pouch | 11 | `lore_creature_charm` | `swamp_toad` | 8 | 12452 `lore_toad_bark` | 6889 `lore_barker_toad` | 6890 (cb 112) |
| 67 | 12031 `lore_war_turtle` | War tortoise pouch | 1 | `lore_creature_charm` | `battle_tortoise_shell` | 1 | 12439 `lore_adamantite_shell` | 6815 `war_tortoise` | 6816 (cb 86) |
| 68 | 12029 `lore_bunyip` | Bunyip pouch | 110 | `lore_insect_charm` | `raw_shark` | 77 | 12438 `lore_swallow_whole` | 6813 `lore_bunyip` | 6814 (cb 70) |
| 69 | 12033 `lore_fruit_bat` | Fruit bat pouch | 130 | `lore_insect_charm` | `banana` | 91 | 12423 `lore_fruitfall` | 6817 `fruit_bat` | none |
| 70 | 12820 `lore_locust` | Ravenous locust pouch | 79 | `lore_leech_charm` | `pot_flour` | 56 | 12830 `lore_famine` | 7372 `lore_locust` | 7373 (cb 120) |
| 71 | 12057 `lore_polar_bear` | Arctic bear pouch | 14 | `lore_creature_charm` | `huntingbeast_polar_fur` | 10 | 12451 `lore_arctic_blast` | 6839 `arctic_bear` | 6840 (cb 122) |
| 72 | 14623 `lore_phoenix` | Phoenix pouch | 165 | `lore_leech_charm` | `lore_phoenix_pinion` | 114 | 14622 `lore_rise_from_the_ashes` | 8575 `lore_phoenix` | 8576 (cb 124) |
| 73 | 12792 `lore_obsidian_golem` | Obsidian golem pouch | 195 | `lore_outsider_charm` | `lore_abby_titan_token` | 137 | 12826 `lore_mountain_strength` | 7345 `lore_obsidian_golem` | 7346 (cb 126) |
| 74 | 12069 `lore_rock_lobster` | Granite lobster pouch | 166 | `lore_leech_charm` | `enakh_granite_tiny` | 117 | 12449 `lore_crushing_claw` | 6849 `lore_rock_lobster` | 6850 (cb 129) |
| 75 | 12011 `lore_praying_mantis` | Praying mantis pouch | 168 | `lore_leech_charm` | `flowers_waterfall_quest_red` | 118 | 12450 `lore_mantis_strike` | 6798 `lore_praying_mantis` | 6799 (cb 131) |
| 76 | 12081 `lore_minotaur_5` | Adamant minotaur pouch | 144 | `lore_outsider_charm` | `adamantite_bar` | 101 | 12465 `lore_bull_rush_5` | 6861 `lore_minotaur_5` | 6862 (cb 133) |
| 76 | 12782 `lore_flaming_prince` | Forge regent pouch | 141 | `lore_insect_charm` | `butterfly_jar_ruby` | 99 | 12841 `lore_inferno` | 7335 `lore_flaming_prince` | 7336 (cb 133) |
| 77 | 12794 `lore_nail_beast` | Talon beast pouch | 174 | `lore_leech_charm` | `lore_nailbeast_charm` | 122 | 12831 `lore_deadly_claw` | 7347 `lore_nail_beast` | 7348 (cb 135) |
| 78 | 12013 `lore_ent` | Giant ent pouch | 124 | `lore_insect_charm` | `willow_branch` | 87 | 12457 `lore_nesting` | 6800 `lore_ent` | 6801 (cb 137) |
| 79 | 12802 `lore_fire_titan` | Fire titan pouch | 198 | `lore_outsider_charm` | `fire_talisman` | 139 | 12824 `lore_titans_constitution` | 7355 `lore_fire_titan` | 7356 (cb 139) |
| 79 | 12804 `lore_moss_titan` | Moss titan pouch | 202 | `lore_outsider_charm` | `earth_talisman` | 142 | 12824 `lore_titans_constitution` | 7357 `lore_moss_titan` | 7358 (cb 139) |
| 79 | 12806 `lore_ice_titan` | Ice titan pouch | 198 | `lore_outsider_charm` | `air_talisman + water_talisman` | 139 | 12824 `lore_titans_constitution` | 7359 `lore_ice_titan` | 7360 (cb 139) |
| 80 | 12025 `lore_hydra` | Hydra pouch | 128 | `lore_insect_charm` | `water_orb` | 90 | 12442 `lore_regrowth` | 6811 `lore_hydra` | 6812 (cb 141) |
| 83 | 12017 `lore_dagganoth` | Spirit dagannoth pouch | 1 | `lore_leech_charm` | `dagganoth_hide` | 1 | 12456 `lore_spike_shot` | 6804 `lore_dagganoth` | 6805 (cb 148) |
| 83 | 12788 `lore_lava_titan` | Lava titan pouch | 219 | `lore_outsider_charm` | `lore_abby_titan_token` | 154 | 12837 `lore_obsidian_thunder` | 7341 `lore_lava_titan` | 7342 (cb 148) |
| 85 | 12776 `lore_swamp_titan` | Swamp titan pouch | 150 | `lore_leech_charm` | `green_salamander` | 105 | 12832 `lore_swamp_plague` | 7329 `lore_swamp_titan` | 7330 (cb 152) |
| 86 | 12083 `lore_minotaur_6` | Rune minotaur pouch | 1 | `lore_outsider_charm` | `runite_bar` | 1 | 12466 `lore_bull_rush_6` | 6863 `lore_minotaur_6` | 6864 (cb 154) |
| 88 | 12039 `lore_unicorn` | Unicorn stallion pouch | 140 | `lore_insect_charm` | `unicorn_horn` | 98 | 12434 `lore_healing_aura` | 6822 `unicorn_stallion` | 6823 (cb 70) |
| 89 | 12786 `lore_geyser_titan` | Geyser titan pouch | 222 | `lore_outsider_charm` | `water_talisman` | 156 | 12833 `lore_boil` | 7339 `lore_geyser_titan` | 7340 (cb 200) |
| 92 | 12089 `lore_wolpertinger` | Wolpertinger pouch | 203 | `lore_leech_charm` | `wolf_bones + raw_rabbit` | 143 | 12437 `lore_magic_might` | 6869 `wolpertinger` | 6870 (cb 210) |
| 93 | 12796 `lore_abyssal_titan` | Abyssal titan pouch | 113 | `lore_insect_charm` | `lore_monster_charm` | 80 | 12827 `lore_chaotic_runecrafting` | 7349 `lore_abyssal_titan` | 7350 (cb 215) |
| 95 | 12822 `lore_iron_titan` | Iron titan pouch | 198 | `lore_leech_charm` | `iron_platebody` | 139 | 12828 `lore_iron_within` | 7375 `lore_iron_titan` | 7376 (cb 220) |
| 96 | 12093 `lore_pack_yak` | Pack yak pouch | 211 | `lore_leech_charm` | `yak_hide` | 148 | 12435 `lore_winter_storage` | 6873 `pack_yak` | 6874 (cb 175) |
| 99 | 12790 `lore_steel_titan` | Steel titan pouch | 178 | `lore_leech_charm` | `steel_platebody` | 125 | 12825 `lore_steel_of_legends` | 7343 `lore_steel_titan` | 7344 (cb 230) |

### Other objs with `iop4=Summon` (67)

- 12526 `wolf_wolf_pouch` "Spirit wolf pouch" (level -)
- 14422 `sc_lore_beast` "Sacred clay pouch (class 1)" (level 3)
- 14424 `sc_lore_beast_2` "Sacred clay pouch (class 2)" (level 20)
- 14426 `sc_lore_beast_3` "Sacred clay pouch (class 3)" (level 40)
- 14428 `sc_lore_beast_4` "Sacred clay pouch (class 4)" (level 60)
- 14430 `sc_lore_beast_5` "Sacred clay pouch (class 5)" (level 80)
- 17935 `rand_loremel_01` "Cub bloodrager pouch" (level 1)
- 17936 `rand_loremel_02` "Little bloodrager pouch" (level 11)
- 17937 `rand_loremel_03` "Naïve bloodrager pouch" (level 21)
- 17938 `rand_loremel_04` "Keen bloodrager pouch" (level 31)
- 17939 `rand_loremel_05` "Brave bloodrager pouch" (level 41)
- 17940 `rand_loremel_06` "Brah bloodrager pouch" (level 51)
- 17941 `rand_loremel_07` "Naabe bloodrager pouch" (level 61)
- 17942 `rand_loremel_08` "Wise bloodrager pouch" (level 71)
- 17943 `rand_loremel_09` "Adept bloodrager pouch" (level 81)
- 17944 `rand_loremel_10` "Sachem bloodrager pouch" (level 91)
- 17945 `rand_loremag_01` "Cub stormbringer pouch" (level 3)
- 17946 `rand_loremag_02` "Little stormbringer pouch" (level 13)
- 17947 `rand_loremag_03` "Naïve stormbringer pouch" (level 23)
- 17948 `rand_loremag_04` "Keen stormbringer pouch" (level 33)
- 17949 `rand_loremag_05` "Brave stormbringer pouch" (level 43)
- 17950 `rand_loremag_06` "Brah stormbringer pouch" (level 53)
- 17951 `rand_loremag_07` "Naabe stormbringer pouch" (level 63)
- 17952 `rand_loremag_08` "Wise stormbringer pouch" (level 73)
- 17953 `rand_loremag_09` "Adept stormbringer pouch" (level 83)
- 17954 `rand_loremag_10` "Sachem stormbringer pouch" (level 93)
- 17955 `rand_loreskill_01` "Cub hoardstalker pouch" (level 5)
- 17956 `rand_loreskill_02` "Little hoardstalker pouch" (level 15)
- 17957 `rand_loreskill_03` "Naïve hoardstalker pouch" (level 25)
- 17958 `rand_loreskill_04` "Keen hoardstalker pouch" (level 35)
- 17959 `rand_loreskill_05` "Brave hoardstalker pouch" (level 45)
- 17960 `rand_loreskill_06` "Brah hoardstalker pouch" (level 55)
- 17961 `rand_loreskill_07` "Naabe hoardstalker pouch" (level 65)
- 17962 `rand_loreskill_08` "Wise hoardstalker pouch" (level 75)
- 17963 `rand_loreskill_09` "Adept hoardstalker pouch" (level 85)
- 17964 `rand_loreskill_10` "Sachem hoardstalker pouch" (level 95)
- 17965 `rand_loreheal_01` "Cub skinweaver pouch" (level 9)
- 17966 `rand_loreheal_02` "Little skinweaver pouch" (level 19)
- 17967 `rand_loreheal_03` "Naïve skinweaver pouch" (level 29)
- 17968 `rand_loreheal_04` "Keen skinweaver pouch" (level 39)
- 17969 `rand_loreheal_05` "Brave skinweaver pouch" (level 49)
- 17970 `rand_loreheal_06` "Brah skinweaver pouch" (level 59)
- 17971 `rand_loreheal_07` "Naabe skinweaver pouch" (level 69)
- 17972 `rand_loreheal_08` "Wise skinweaver pouch" (level 79)
- 17973 `rand_loreheal_09` "Adept skinweaver pouch" (level 89)
- 17974 `rand_loreheal_10` "Sachem skinweaver pouch" (level 99)
- 17975 `rand_lorebob_01` "Cub worldbearer pouch" (level 7)
- 17976 `rand_lorebob_02` "Little worldbearer pouch" (level 17)
- 17977 `rand_lorebob_03` "Naïve worldbearer pouch" (level 27)
- 17978 `rand_lorebob_04` "Keen worldbearer pouch" (level 37)
- 17979 `rand_lorebob_05` "Brave worldbearer pouch" (level 47)
- 17980 `rand_lorebob_06` "Brah worldbearer pouch" (level 57)
- 17981 `rand_lorebob_07` "Naabe worldbearer pouch" (level 67)
- 17982 `rand_lorebob_08` "Wise worldbearer pouch" (level 77)
- 17983 `rand_lorebob_09` "Adept worldbearer pouch" (level 87)
- 17984 `rand_lorebob_10` "Sachem worldbearer pouch" (level 97)
- 17985 `rand_loreran_01` "Cub deathslinger pouch" (level 2)
- 17986 `rand_loreran_02` "Little deathslinger pouch" (level 12)
- 17987 `rand_loreran_03` "Naïve deathslinger pouch" (level 22)
- 17988 `rand_loreran_04` "Keen deathslinger pouch" (level 32)
- 17989 `rand_loreran_05` "Brave deathslinger pouch" (level 42)
- 17990 `rand_loreran_06` "Brah deathslinger pouch" (level 52)
- 17991 `rand_loreran_07` "Naabe deathslinger pouch" (level 62)
- 17992 `rand_loreran_08` "Wise deathslinger pouch" (level 72)
- 17993 `rand_loreran_09` "Adept deathslinger pouch" (level 82)
- 17994 `rand_loreran_10` "Sachem deathslinger pouch" (level 92)
- 19622 `lore_meerkats` "Meerkats pouch" (level 4)

### Scrolls reachable from enum 1283 (129)

| Obj | Name | Lvl | Bogrog shards (457) | Bogrog batch (599) |
|---|---|---|---|---|
| 12422 `lore_herbcall` | Herbcall scroll | 41 | 4 | - |
| 12423 `lore_fruitfall` | Fruitfall scroll | 69 | 7 | - |
| 12424 `lore_fish_rain` | Fish rain scroll | 56 | 6 | - |
| 12425 `lore_howl` | Howl scroll | 1 | - | 3 |
| 12426 `lore_thieving_fingers` | Thieving fingers scroll | 47 | 4 | - |
| 12427 `lore_abyssal_stealth` | Abyssal stealth scroll | 62 | 6 | - |
| 12428 `lore_egg_spawn` | Egg spawn scroll | 10 | - | 3 |
| 12429 `lore_multichop` | Multichop scroll | 33 | 4 | - |
| 12430 `lore_cheese_feast` | Cheese feast scroll | 23 | 4 | - |
| 12431 `lore_unburden` | Unburden scroll | 40 | - | 2 |
| 12432 `lore_envenom_weapon` | Venom shot scroll | 19 | 3 | - |
| 12433 `lore_insane_ferocity` | Insane ferocity scroll | 32 | 4 | - |
| 12434 `lore_healing_aura` | Healing aura scroll | 88 | 7 | - |
| 12435 `lore_winter_storage` | Winter storage scroll | 96 | 11 | - |
| 12436 `lore_ophedian_incubation` | Oph. incubation scroll | 63 | 6 | - |
| 12437 `lore_magic_might` | Magic focus scroll | 92 | 11 | - |
| 12438 `lore_swallow_whole` | Swallow whole scroll | 68 | 6 | - |
| 12439 `lore_adamantite_shell` | Testudo scroll | 67 | - | 18 |
| 12440 `lore_generate_compost` | Generate compost scroll | 28 | 2 | - |
| 12441 `lore_tireless_run` | Tireless run scroll | 52 | - | 2 |
| 12442 `lore_regrowth` | Regrowth scroll | 80 | 7 | - |
| 12443 `lore_call_to_arms` | Call to arms scroll | 34 | 4 | - |
| 12444 `lore_blood_drain` | Blood drain scroll | 49 | 6 | - |
| 12445 `lore_dreadfowl_strike` | Dreadfowl strike scroll | 4 | - | 3 |
| 12446 `lore_sandstorm` | Sandstorm scroll | 25 | 2 | - |
| 12447 `lore_vampire_touch` | Vampire touch scroll | 31 | 4 | - |
| 12448 `lore_evil_flames` | Evil flames scroll | 42 | 5 | - |
| 12449 `lore_crushing_claw` | Crushing claw scroll | 74 | 9 | - |
| 12450 `lore_mantis_strike` | Mantis strike scroll | 75 | 9 | - |
| 12451 `lore_arctic_blast` | Arctic blast scroll | 71 | - | 2 |
| 12452 `lore_toad_bark` | Toad bark scroll | 66 | - | 2 |
| 12453 `lore_dissolve` | Dissolve scroll | 55 | 8 | - |
| 12454 `lore_abyssal_drain` | Abyssal drain scroll | 54 | 5 | - |
| 12455 `lore_overwhelming_waters` | Doomsphere scroll | 58 | 8 | - |
| 12456 `lore_spike_shot` | Spike shot scroll | 83 | - | 18 |
| 12457 `lore_nesting` | Acorn missile scroll | 78 | 6 | - |
| 12458 `lore_petrify` | Petrifying gaze scroll | 43 | 4 | - |
| 12459 `lore_slime_trail` | Slime spray scroll | 13 | - | 2 |
| 12460 `lore_electric_lash` | Electric lash scroll | 18 | 2 | - |
| 12461 `lore_bull_rush` | Bronze bull rush scroll | 36 | 5 | - |
| 12462 `lore_bull_rush_2` | Iron bull rush scroll | 46 | 7 | - |
| 12463 `lore_bull_rush_3` | Steel bull rush scroll | 56 | 7 | - |
| 12464 `lore_bull_rush_4` | Mith bull rush scroll | 66 | 8 | - |
| 12465 `lore_bull_rush_5` | Addy bull rush scroll | 76 | 8 | - |
| 12466 `lore_bull_rush_6` | Rune bull rush scroll | 86 | - | 18 |
| 12467 `lore_poisonous_blast` | Poisonous blast scroll | 64 | 7 | - |
| 12468 `lore_dust_cloud` | Dust cloud scroll | 61 | 7 | - |
| 12533 `lore_stony_shell` | Stony shell scroll | 16 | - | 3 |
| 12824 `lore_titans_constitution` | Titan's con. scroll | 79 | 11 | - |
| 12825 `lore_steel_of_legends` | Steel of legends scroll | 99 | 9 | - |
| 12826 `lore_mountain_strength` | Volcanic str. scroll | 73 | 10 | - |
| 12827 `lore_chaotic_runecrafting` | Essence shipment scroll | 93 | 6 | - |
| 12828 `lore_iron_within` | Iron within scroll | 95 | 11 | - |
| 12829 `lore_immense_heat` | Immense heat scroll | 46 | 6 | - |
| 12830 `lore_famine` | Famine scroll | 70 | 6 | - |
| 12831 `lore_deadly_claw` | Deadly claw scroll | 77 | 9 | - |
| 12832 `lore_swamp_plague` | Swamp plague scroll | 85 | 8 | - |
| 12833 `lore_boil` | Boil scroll | 89 | 12 | - |
| 12834 `lore_explode` | Explode scroll | 29 | 4 | - |
| 12835 `lore_goad` | Goad scroll | 57 | 8 | - |
| 12836 `lore_ambush` | Ambush scroll | 57 | 8 | - |
| 12837 `lore_obsidian_thunder` | Ebon thunder scroll | 83 | 12 | - |
| 12838 `lore_mosquito_attack` | Pester scroll | 17 | - | 18 |
| 12839 `lore_lava_assault` | Fireball assault scroll | 22 | 3 | - |
| 12840 `lore_rending` | Rending scroll | 57 | 8 | - |
| 12841 `lore_inferno` | Inferno scroll | 76 | 7 | - |
| 14421 `sc_lore_clay_deposit` | Clay deposit scroll | 3 | - | - |
| 14622 `lore_rise_from_the_ashes` | Rise from the ashes scroll | 72 | 9 | - |
| 18027 `rand_loremel_01_spell` | Sundering strike (tier 1) | 1 | - | 3 |
| 18028 `rand_loremel_02_spell` | Sundering strike (tier 2) | 11 | - | 3 |
| 18029 `rand_loremel_03_spell` | Sundering strike (tier 3) | 21 | - | 3 |
| 18030 `rand_loremel_04_spell` | Sundering strike (tier 4) | 31 | - | 3 |
| 18031 `rand_loremel_05_spell` | Sundering strike (tier 5) | 41 | - | 3 |
| 18032 `rand_loremel_06_spell` | Sundering strike (tier 6) | 51 | - | 3 |
| 18033 `rand_loremel_07_spell` | Sundering strike (tier 7) | 61 | - | 3 |
| 18034 `rand_loremel_08_spell` | Sundering strike (tier 8) | 71 | - | 3 |
| 18035 `rand_loremel_09_spell` | Sundering strike (tier 9) | 81 | - | 3 |
| 18036 `rand_loremel_10_spell` | Sundering strike (tier 10) | 91 | - | 3 |
| 18037 `rand_loreran_01_spell` | Poisonous shot (tier 1) | 2 | - | 3 |
| 18038 `rand_loreran_02_spell` | Poisonous shot (tier 2) | 12 | - | 3 |
| 18039 `rand_loreran_03_spell` | Poisonous shot (tier 3) | 22 | - | 3 |
| 18040 `rand_loreran_04_spell` | Poisonous shot (tier 4) | 32 | - | 3 |
| 18041 `rand_loreran_05_spell` | Poisonous shot (tier 5) | 42 | - | 3 |
| 18042 `rand_loreran_06_spell` | Poisonous shot (tier 6) | 52 | - | 3 |
| 18043 `rand_loreran_07_spell` | Poisonous shot (tier 7) | 62 | - | 3 |
| 18044 `rand_loreran_08_spell` | Poisonous shot (tier 8) | 72 | - | 3 |
| 18045 `rand_loreran_09_spell` | Poisonous shot (tier 9) | 82 | - | 3 |
| 18046 `rand_loreran_10_spell` | Poisonous shot (tier 10) | 92 | - | 3 |
| 18047 `rand_loremag_01_spell` | Snaring wave (tier 1) | 3 | - | 3 |
| 18048 `rand_loremag_02_spell` | Snaring wave (tier 2) | 13 | - | 3 |
| 18049 `rand_loremag_03_spell` | Snaring wave (tier 3) | 23 | - | 3 |
| 18050 `rand_loremag_04_spell` | Snaring wave (tier 4) | 33 | - | 3 |
| 18051 `rand_loremag_05_spell` | Snaring wave (tier 5) | 43 | - | 3 |
| 18052 `rand_loremag_06_spell` | Snaring wave (tier 6) | 53 | - | 3 |
| 18053 `rand_loremag_07_spell` | Snaring wave (tier 7) | 63 | - | 3 |
| 18054 `rand_loremag_08_spell` | Snaring wave (tier 8) | 73 | - | 3 |
| 18055 `rand_loremag_09_spell` | Snaring wave (tier 9) | 83 | - | 3 |
| 18056 `rand_loremag_10_spell` | Snaring wave (tier 10) | 93 | - | 3 |
| 18057 `rand_loreskill_01_spell` | Aptitude (tier 1) | 5 | - | 3 |
| 18058 `rand_loreskill_02_spell` | Aptitude (tier 2) | 15 | - | 3 |
| 18059 `rand_loreskill_03_spell` | Aptitude (tier 3) | 25 | - | 3 |
| 18060 `rand_loreskill_04_spell` | Aptitude (tier 4) | 35 | - | 3 |
| 18061 `rand_loreskill_05_spell` | Aptitude (tier 5) | 45 | - | 3 |
| 18062 `rand_loreskill_06_spell` | Aptitude (tier 6) | 55 | - | 3 |
| 18063 `rand_loreskill_07_spell` | Aptitude (tier 7) | 65 | - | 3 |
| 18064 `rand_loreskill_08_spell` | Aptitude (tier 8) | 75 | - | 3 |
| 18065 `rand_loreskill_09_spell` | Aptitude (tier 9) | 85 | - | 3 |
| 18066 `rand_loreskill_10_spell` | Aptitude (tier 10) | 95 | - | 3 |
| 18067 `rand_lorebob_01_spell` | Second wind (tier 1) | 7 | - | 3 |
| 18068 `rand_lorebob_02_spell` | Second wind (tier 2) | 17 | - | 3 |
| 18069 `rand_lorebob_03_spell` | Second wind (tier 3) | 27 | - | 3 |
| 18070 `rand_lorebob_04_spell` | Second wind (tier 4) | 37 | - | 3 |
| 18071 `rand_lorebob_05_spell` | Second wind (tier 5) | 47 | - | 3 |
| 18072 `rand_lorebob_06_spell` | Second wind (tier 6) | 57 | - | 3 |
| 18073 `rand_lorebob_07_spell` | Second wind (tier 7) | 67 | - | 3 |
| 18074 `rand_lorebob_08_spell` | Second wind (tier 8) | 77 | - | 3 |
| 18075 `rand_lorebob_09_spell` | Second wind (tier 9) | 87 | - | 3 |
| 18076 `rand_lorebob_10_spell` | Second wind (tier 10) | 97 | - | 3 |
| 18077 `rand_loreheal_01_spell` | Glimmer of light (tier 1) | 9 | - | 3 |
| 18078 `rand_loreheal_02_spell` | Glimmer of light (tier 2) | 19 | - | 3 |
| 18079 `rand_loreheal_03_spell` | Glimmer of light (tier 3) | 29 | - | 3 |
| 18080 `rand_loreheal_04_spell` | Glimmer of light (tier 4) | 39 | - | 3 |
| 18081 `rand_loreheal_05_spell` | Glimmer of light (tier 5) | 49 | - | 3 |
| 18082 `rand_loreheal_06_spell` | Glimmer of light (tier 6) | 59 | - | 3 |
| 18083 `rand_loreheal_07_spell` | Glimmer of light (tier 7) | 69 | - | 3 |
| 18084 `rand_loreheal_08_spell` | Glimmer of light (tier 8) | 79 | - | 3 |
| 18085 `rand_loreheal_09_spell` | Glimmer of light (tier 9) | 89 | - | 3 |
| 18086 `rand_loreheal_10_spell` | Glimmer of light (tier 10) | 99 | - | 3 |
| 19621 `lore_fetch_casket` | Fetch casket scroll | 4 | - | - |

### NPCs with `familiar=yes` (527)

- **Daemonheim**: 120 — 11106-11225
- **Stealing Creation**: 10 — 8240-8249
- **familiars (Interact)**: 152 — 6794-6851, 6853-6890, 6991-6995, 7329-7368, 7370-7373, 7375-7378, 8575-8576, 11640
- **no ops**: 10 — 6852, 6921, 6997, 7275-7276, 7309-7312, 7374
- **pets (Pick-up op)**: 235 — 761-766, 768-779, 2662-2673, 3503-3507, 6900-6916, 6919-6920, 6922-6943, 6945-6969, 7015-7020, 7210-7227, 7237-7274, 7277-7284, 7293-7308, 7313-7328, 7844, 8214-8218, 8491, 8577-8578, 8619-8622, 8624, 9461, 9656-9669, 11411, 13089-13090

### Standard familiar NPCs

| Lvl (394) | Npc | Name | Size | Ops | Combat npc | Combat lvl | BoB (1323) |
|---|---|---|---|---|---|---|---|
| 1 | 6829 `lore_wolf` | Spirit wolf | 2 | 1:Interact | 6830 `lore_wolf_combat` | 26 |  |
| 4 | 6825 `lore_dreadfowl` | Dreadfowl | 1 | 1:Interact, 5:Special | 6826 `lore_dreadfowl_combat` | 26 |  |
| 10 | 6841 `lore_spider` | Spirit spider | 1 | 1:Interact | 6842 `lore_spider_combat` | 25 |  |
| 13 | 6806 `lore_snail` | Thorny snail | 1 | 1:Interact, 3:Store | 6807 `lore_snail_combat` | 26 | 1 |
| 16 | 6796 `lore_rock_crab` | Granite crab | 1 | 1:Interact, 3:Withdraw | 6797 `lore_rock_crab_combat` | 26 |  |
| 17 | 7331 `lore_mosquito` | Spirit mosquito | 1 | 1:Interact | 7332 `lore_mosquito_combat` | 32 |  |
| 18 | 6831 `lore_desert_worm` | Desert wyrm | 2 | 1:Interact, 3:Burrow | 6832 `lore_desert_worm_combat` | 31 |  |
| 19 | 6837 `spirit_scorpion` | Spirit scorpion | 1 | 1:Interact, 1:talk-to | 6838 `spirit_scorpion_6838` | 51 |  |
| 22 | 7361 `lore_tz_kih` | Spirit Tz-Kih | 1 | 1:Interact | 7362 `lore_tz_kih_combat` | 36 |  |
| 23 | 6847 `lore_giant_rat` | Albino rat | 2 | 1:Interact, 3:Withdraw | 6848 `lore_giant_rat_combat` | 37 |  |
| 25 | 6994 `spirit_kalphite` | Spirit kalphite | 2 | 1:Interact, 3:Store | 6995 `spirit_kalphite_6995` | 39 | 1 |
| 28 | 6871 `lore_compost_mound` | Compost mound | 2 | 1:Interact, 3:Withdraw | 6872 `lore_compost_mound_combat` | 37 |  |
| 29 | 7353 `lore_chinchompa` | Giant chinchompa | 1 | 1:Interact | 7354 `lore_chinchompa_combat` | 42 |  |
| 31 | 6835 `vampire_bat` | Vampire bat | 1 | 1:Interact | 6836 `vampire_bat_6836` | 44 |  |
| 32 | 6845 `lore_honey_badger` | Honey badger | 1 | 1:Interact | 6846 `lore_honey_badger_combat` | 45 |  |
| 22/33 | 6808 `lore_beaver` | Beaver | 1 | 1:Interact, 3:Withdraw | none | - |  |
| 34 | 7333 `lore_spinner` | Void spinner | 1 | 1:Interact | 7334 `lore_spinner_combat` | 40 |  |
| 34 | 7351 `lore_torcher` | Void torcher | 1 | 1:Interact, 5:Strike | 7352 `lore_torcher_combat` | 46 |  |
| 34 | 7367 `lore_shifter` | Void shifter | 1 | 1:Interact | 7368 `lore_shifter_combat` | 46 |  |
| 34 | 7370 `lore_ravager` | Void ravager | 1 | 1:Interact, 3:Withdraw | 7371 `lore_ravager_combat` | 46 |  |
| 36 | 6853 `lore_minotaur` | Bronze minotaur | 2 | 1:Interact | 6854 `lore_minotaur_combat` | 50 |  |
| 40 | 6867 `lore_giant_ant` | Bull ant | 1 | 1:Interact, 1:talk-to, 3:Store | 6868 `lore_giant_ant_combat` | 58 | 1 |
| 41 | 6851 `lore_macaw` | Macaw | 1 | 1:Interact, 3:Withdraw | none | - |  |
| 42 | 6833 `lore_turnip` | Evil turnip | 1 | 1:Interact, 3:Withdraw | 6834 `lore_turnip_combat` | 62 |  |
| 43 | 6875 `lore_cockatrice` | Spirit cockatrice | 2 | 1:Interact, 3:Withdraw, 5:Drain | 6876 `lore_cockatrice_combat` | 64 |  |
| 43 | 6877 `lore_cockatrice_2` | Spirit guthatrice | 2 | 1:Interact, 3:Withdraw, 5:Drain | 6878 `lore_cockatrice_combat_2` | 64 |  |
| 43 | 6879 `lore_cockatrice_3` | Spirit saratrice | 2 | 1:Interact, 3:Withdraw, 5:Drain | 6880 `lore_cockatrice_combat_3` | 64 |  |
| 43 | 6881 `lore_cockatrice_4` | Spirit zamatrice | 2 | 1:Interact, 3:Withdraw, 5:Drain | 6882 `lore_cockatrice_combat_4` | 64 |  |
| 43 | 6883 `lore_cockatrice_5` | Spirit pengatrice | 2 | 1:Interact, 3:Withdraw, 5:Drain | 6884 `lore_cockatrice_combat_5` | 64 |  |
| 43 | 6885 `lore_cockatrice_6` | Spirit coraxatrice | 2 | 1:Interact, 3:Withdraw, 5:Drain | 6886 `lore_cockatrice_combat_6` | 64 |  |
| 43 | 6887 `lore_cockatrice_7` | Spirit vulatrice | 2 | 1:Interact, 3:Withdraw, 5:Drain | 6888 `lore_cockatrice_combat_7` | 64 |  |
| 46 | 6855 `lore_minotaur_2` | Iron minotaur | 2 | 1:Interact | 6856 `lore_minotaur_combat_2` | 70 |  |
| 46 | 7377 `pyrelord` | Pyrelord | 1 | 1:Interact | 7378 `pyrelord_7378` | 70 |  |
| 47 | 6824 `lore_magpie` | Magpie | 1 | 1:Interact, 3:Withdraw | none | - |  |
| 49 | 6843 `lore_leech` | Bloated leech | 1 | 1:Interact | 6844 `lore_leech_combat` | 76 |  |
| 52 | 6794 `lore_terrorbird` | Spirit terrorbird | 2 | 1:Interact, 3:Store | 6795 `lore_terrorbird_combat` | 62 | 1 |
| 54 | 6818 `lore_abyssal_leech` | Abyssal parasite | 1 | 1:Interact, 3:Store | 6819 `lore_abyssal_leech_combat` | 86 |  |
| 55 | 6992 `spirit_jelly` | Spirit jelly | 1 | 1:Interact | 6993 `spirit_jelly_6993` | 88 |  |
| 56 | 6857 `lore_minotaur_3` | Steel minotaur | 2 | 1:Interact | 6858 `lore_minotaur_combat_3` | 90 |  |
| 56 | 6991 `lore_stork` | Ibis | 1 | 1:Interact, 3:Withdraw | none | - |  |
| 57 | 7337 `lore_larupita` | Spirit larupia | 2 | 1:Interact | 7338 `lore_larupita_combat` | 93 |  |
| 57 | 7363 `spirit_graahk` | Spirit graahk | 2 | 1:Interact | 7364 `spirit_graahk_7364` | 93 |  |
| 57 | 7365 `lore_kyatt` | Spirit kyatt | 2 | 1:Interact | 7366 `lore_kyatt_combat` | 93 |  |
| 58 | 6809 `lore_karamthulhu` | Karamthulhu overlord | 1 | 1:Interact, 5:Drown | 6810 `lore_karamthulhu_combat` | 95 |  |
| 61 | 6865 `lore_dust_devil` | Smoke devil | 1 | 1:Interact, 5:Flames | 6866 `lore_dust_devil_combat` | 101 |  |
| 62 | 6820 `lore_abyssal_lurker` | Abyssal lurker | 1 | 1:Interact, 3:Store | 6821 `lore_abyssal_lurker_combat` | 93 |  |
| 63 | 6802 `lore_snake` | Spirit cobra | 2 | 1:Interact | 6803 `lore_snake_combat` | 105 |  |
| 80 | 6827 `lore_stranger_plant` | Stranger plant | 1 | 1:Interact, 3:Withdraw | 6828 `lore_stranger_plant_combat` | 107 |  |
| 66 | 6859 `lore_minotaur_4` | Mithril minotaur | 2 | 1:Interact | 6860 `lore_minotaur_combat_4` | 112 |  |
| 66 | 6889 `lore_barker_toad` | Barker toad | 1 | 1:Interact, 5:Cannon | 6890 `lore_barker_toad_combat` | 112 |  |
| 67 | 6815 `war_tortoise` | War tortoise | 2 | 1:Interact, 3:Store | 6816 `war_tortoise_6816` | 86 | 1 |
| 68 | 6813 `lore_bunyip` | Bunyip | 2 | 1:Interact | 6814 `lore_bunyip_combat` | 70 |  |
| 69 | 6817 `fruit_bat` | Fruit bat | 1 | 1:Interact, 3:Withdraw | none | - |  |
| 70 | 7372 `lore_locust` | Ravenous locust | 1 | 1:Interact, 1:talk-to | 7373 `lore_locust_combat` | 120 |  |
| 71 | 6839 `arctic_bear` | Arctic bear | 2 | 1:Interact | 6840 `arctic_bear_6840` | 122 |  |
| 72 | 8575 `lore_phoenix` | Phoenix | 2 | 1:Interact, 5:Ash-blast | 8576 `lore_phoenix_combat` | 124 |  |
| 73 | 7345 `lore_obsidian_golem` | Obsidian golem | 2 | 1:Interact | 7346 `lore_obsidian_golem_combat` | 126 |  |
| 74 | 6849 `lore_rock_lobster` | Granite lobster | 1 | 1:Interact, 1:talk-to, 3:Withdraw | 6850 `lore_rock_lobster_combat` | 129 |  |
| 75 | 6798 `lore_praying_mantis` | Praying mantis | 1 | 1:Interact, 1:talk-to | 6799 `lore_praying_mantis_combat` | 131 |  |
| 76 | 6861 `lore_minotaur_5` | Adamant minotaur | 2 | 1:Interact | 6862 `lore_minotaur_combat_5` | 133 |  |
| 76 | 7335 `lore_flaming_prince` | Forge regent | 1 | 1:Interact, 5:Fireball | 7336 `lore_flaming_prince_combat` | 133 |  |
| 77 | 7347 `lore_nail_beast` | Talon beast | 2 | 1:Interact | 7348 `lore_nail_beast_combat` | 135 |  |
| 78 | 6800 `lore_ent` | Giant ent | 2 | 1:Interact, 3:Withdraw | 6801 `lore_ent_combat` | 137 |  |
| 79 | 7355 `lore_fire_titan` | Fire titan | 2 | 1:Interact | 7356 `lore_fire_titan_combat` | 139 |  |
| 79 | 7357 `lore_moss_titan` | Moss titan | 2 | 1:Interact | 7358 `lore_moss_titan_combat` | 139 |  |
| 79 | 7359 `lore_ice_titan` | Ice titan | 2 | 1:Interact | 7360 `lore_ice_titan_combat` | 139 |  |
| 80 | 6811 `lore_hydra` | Hydra | 2 | 1:Interact | 6812 `lore_hydra_combat` | 141 |  |
| 83 | 6804 `lore_dagganoth` | Spirit dagannoth | 2 | 1:Interact | 6805 `lore_dagganoth_combat` | 148 |  |
| 83 | 7341 `lore_lava_titan` | Lava titan | 2 | 1:Interact | 7342 `lore_lava_titan_combat` | 148 |  |
| 85 | 7329 `lore_swamp_titan` | Swamp titan | 2 | 1:Interact | 7330 `lore_swamp_titan_combat` | 152 |  |
| 86 | 6863 `lore_minotaur_6` | Rune minotaur | 2 | 1:Interact | 6864 `lore_minotaur_combat_6` | 154 |  |
| 88 | 6822 `unicorn_stallion` | Unicorn stallion | 2 | 1:Interact, 5:Cure | 6823 `unicorn_stallion_6823` | 70 |  |
| 89 | 7339 `lore_geyser_titan` | Geyser titan | 2 | 1:Interact | 7340 `lore_geyser_titan_combat` | 200 |  |
| 92 | 6869 `wolpertinger` | Wolpertinger | 1 | 1:Interact | 6870 `wolpertinger_6870` | 210 |  |
| 93 | 7349 `lore_abyssal_titan` | Abyssal titan | 2 | 1:Interact, 3:Store | 7350 `lore_abyssal_titan_combat` | 215 |  |
| 95 | 7375 `lore_iron_titan` | Iron titan | 2 | 1:Interact | 7376 `lore_iron_titan_combat` | 220 |  |
| 96 | 6873 `pack_yak` | Pack yak | 2 | 1:Interact, 3:Store | 6874 `pack_yak_6874` | 175 | 1 |
| 99 | 7343 `lore_steel_titan` | Steel titan | 2 | 1:Interact | 7344 `lore_steel_titan_combat` | 230 |  |

### Obelisk locs (39 configs with Infuse-pouch or Renew-points)

| Loc | Name | Ops | Placements |
|---|---|---|---|
| 5787 `ess002_mini_obelisk_light_rock` | Small obelisk | 1:Renew-points | 1 |
| 28716 `wolf_obelisk_druid01` | Obelisk | 1:Infuse-pouch, 2:Renew-points | 1 |
| 28719 `wolf_obelisk_nature01` | Obelisk | 1:Infuse-pouch, 2:Renew-points | 2 |
| 28722 `wolf_obelisk_ogrish01` | Obelisk | 1:Infuse-pouch, 2:Renew-points | 1 |
| 28725 `wolf_obelisk_desert01` | Obelisk | 1:Infuse-pouch, 2:Renew-points | 1 |
| 28728 `wolf_obelisk_druid_elf_01` | Obelisk | 1:Infuse-pouch, 2:Renew-points | 1 |
| 28731 `wolf_obelisk_druid_jungle_01` | Obelisk | 1:Infuse-pouch, 2:Renew-points | 1 |
| 28734 `wolf_obelisk_druid_fishing_colony_01` | Obelisk | 1:Infuse-pouch, 2:Renew-points | 1 |
| 29938 `lore_mini_obelisk_jungle` | Small obelisk | 1:Renew-points | 1 |
| 29939 `lore_mini_obelisk_lava` | Small obelisk | 1:Renew-points | 1 |
| 29940 `lore_mini_obelisk_tzhaar` | Small obelisk | 1:Renew-points | 1 |
| 29942 `lore_mini_obelisk_ice` | Small obelisk | 1:Renew-points | 1 |
| 29943 `lore_mini_obelisk_desert` | Small obelisk | 1:Renew-points | 8 |
| 29944 `lore_mini_obelisk_rocky` | Small obelisk | 1:Renew-points | 8 |
| 29945 `lore_mini_obelisk_rocky2` | Small obelisk | 1:Renew-points | 3 |
| 29946 `lore_mini_obelisk_rocky3` | Small obelisk | 1:Renew-points | 1 |
| 29947 `lore_mini_obelisk_rocky4` | Small obelisk | 1:Renew-points | 6 |
| 29948 `lore_mini_obelisk_rocky5` | Small obelisk | 1:Renew-points | 1 |
| 29950 `lore_mini_obelisk_contact` | Small obelisk | 1:Renew-points | 1 |
| 29951 `lore_mini_obelisk_mountain` | Small obelisk | 1:Renew-points | 1 |
| 29952 `lore_mini_obelisk_swamp` | Small obelisk | 1:Renew-points | 1 |
| 29953 `lore_mini_obelisk_grass` | Small obelisk | 1:Renew-points | 6 |
| 29954 `lore_mini_obelisk_light_grass` | Small obelisk | 1:Renew-points | 9 |
| 29955 `lore_mini_obelisk_wilderness` | Small obelisk | 1:Renew-points | 2 |
| 29956 `lore_mini_obelisk_sos` | Small obelisk | 1:Renew-points | 2 |
| 29957 `lore_mini_obelisk_morytania` | Small obelisk | 1:Renew-points | 1 |
| 29958 `lore_mini_obelisk_dagannoth` | Small obelisk | 1:Renew-points | 3 |
| 29959 `lore_mini_obelisk_fairy` | Small obelisk | 1:Renew-points | 1 |
| 44837 `poh_menagerie_obelisk_mini` | Small obelisk | 1:Renew-points, 5:Remove | 0 |
| 44838 `poh_menagerie_obelisk_garden` | Small obelisk | 1:Renew-points, 5:Remove | 0 |
| 44839 `poh_menagerie_obelisk_jungle` | Small obelisk | 1:Renew-points, 5:Remove | 0 |
| 44840 `poh_menagerie_obelisk_desert` | Small obelisk | 1:Renew-points, 5:Remove | 0 |
| 44841 `poh_menagerie_obelisk_polar` | Small obelisk | 1:Renew-points, 5:Remove | 0 |
| 44842 `poh_menagerie_obelisk_volcanic` | Small obelisk | 1:Renew-points, 5:Remove | 0 |
| 50205 `rand_summoning_obelisk_frzn` | Summoning obelisk | 1:Infuse-pouch, 2:Renew-points | 44 |
| 50206 `rand_summoning_obelisk_abnd` | Summoning obelisk | 1:Infuse-pouch, 2:Renew-points | 44 |
| 50207 `rand_summoning_obelisk_furn` | Summoning obelisk | 1:Infuse-pouch, 2:Renew-points | 44 |
| 53883 `rand_summoning_obelisk_oclt` | Summoning obelisk | 1:Infuse-pouch, 2:Renew-points | 44 |
| 55605 `rand_summoning_obelisk_wrpd` | Summoning obelisk | 1:Infuse-pouch, 2:Renew-points | 44 |

### Charged obelisk placements (overworld) (8)

| Loc | File | Level | x,z |
|---|---|---|---|
| 28716 `wolf_obelisk_druid01` | l34_83.jm2 | 0 | 2209,5344 |
| 28719 `wolf_obelisk_nature01` | l53_152.jm2 | 0 | 3441,9749 |
| 28719 `wolf_obelisk_nature01` | l53_152.jm2 | 1 | 3441,9749 |
| 28722 `wolf_obelisk_ogrish01` | l39_47.jm2 | 0 | 2521,3055 |
| 28725 `wolf_obelisk_desert01` | l51_145.jm2 | 0 | 3295,9311 |
| 28728 `wolf_obelisk_druid_elf_01` | l36_150.jm2 | 0 | 2323,9629 |
| 28731 `wolf_obelisk_druid_jungle_01` | l42_50.jm2 | 0 | 2716,3211 |
| 28734 `wolf_obelisk_druid_fishing_colony_01` | l36_156.jm2 | 0 | 2332,10009 |

### Small obelisk placements (59)

| Loc | File | Level | x,z |
|---|---|---|---|
| 5787 `ess002_mini_obelisk_light_rock` | l47_153.jm2 | 0 | 3032,9823 |
| 29938 `lore_mini_obelisk_jungle` | l44_47.jm2 | 0 | 2851,3027 |
| 29939 `lore_mini_obelisk_lava` | l44_49.jm2 | 0 | 2848,3161 |
| 29940 `lore_mini_obelisk_tzhaar` | l38_80.jm2 | 0 | 2439,5138 |
| 29942 `lore_mini_obelisk_ice` | l44_155.jm2 | 0 | 2857,9952 |
| 29943 `lore_mini_obelisk_desert` | l39_56.jm2 | 0 | 2516,3591 |
| 29943 `lore_mini_obelisk_desert` | l44_46.jm2 | 0 | 2820,2967 |
| 29943 `lore_mini_obelisk_desert` | l44_53.jm2 | 0 | 2853,3439 |
| 29943 `lore_mini_obelisk_desert` | l51_48.jm2 | 0 | 3301,3112 |
| 29943 `lore_mini_obelisk_desert` | l51_50.jm2 | 0 | 3306,3244 |
| 29943 `lore_mini_obelisk_desert` | l51_43.jm2 | 0 | 3325,2803 |
| 29943 `lore_mini_obelisk_desert` | l52_46.jm2 | 0 | 3333,3000 |
| 29943 `lore_mini_obelisk_desert` | l58_46.jm2 | 0 | 3752,2968 |
| 29944 `lore_mini_obelisk_rocky` | l32_61.jm2 | 0 | 2111,3917 |
| 29944 `lore_mini_obelisk_rocky` | l33_76.jm2 | 0 | 2134,4884 |
| 29944 `lore_mini_obelisk_rocky` | l38_154.jm2 | 0 | 2455,9917 |
| 29944 `lore_mini_obelisk_rocky` | l40_53.jm2 | 0 | 2593,3423 |
| 29944 `lore_mini_obelisk_rocky` | l40_60.jm2 | 0 | 2606,3854 |
| 29944 `lore_mini_obelisk_rocky` | l41_56.jm2 | 0 | 2674,3640 |
| 29944 `lore_mini_obelisk_rocky` | l48_50.jm2 | 0 | 3094,3221 |
| 29944 `lore_mini_obelisk_rocky` | l48_155.jm2 | 0 | 3112,9956 |
| 29945 `lore_mini_obelisk_rocky2` | l37_59.jm2 | 0 | 2415,3796 |
| 29945 `lore_mini_obelisk_rocky2` | l44_159.jm2 | 0 | 2849,10211 |
| 29945 `lore_mini_obelisk_rocky2` | l44_158.jm2 | 0 | 2857,10168 |
| 29946 `lore_mini_obelisk_rocky3` | l43_156.jm2 | 0 | 2786,9990 |
| 29947 `lore_mini_obelisk_rocky4` | l27_80.jm2 | 0 | 1752,5183 |
| 29947 `lore_mini_obelisk_rocky4` | l35_73.jm2 | 0 | 2256,4711 |
| 29947 `lore_mini_obelisk_rocky4` | l40_147.jm2 | 0 | 2567,9448 |
| 29947 `lore_mini_obelisk_rocky4` | l42_148.jm2 | 0 | 2705,9479 |
| 29947 `lore_mini_obelisk_rocky4` | l45_152.jm2 | 0 | 2889,9787 |
| 29947 `lore_mini_obelisk_rocky4` | l45_154.jm2 | 0 | 2931,9885 |
| 29948 `lore_mini_obelisk_rocky5` | l49_154.jm2 | 0 | 3165,9897 |
| 29950 `lore_mini_obelisk_contact` | l51_144.jm2 | 0 | 3279,9271 |
| 29951 `lore_mini_obelisk_mountain` | l45_57.jm2 | 0 | 2891,3677 |
| 29952 `lore_mini_obelisk_swamp` | l57_55.jm2 | 0 | 3660,3538 |
| 29953 `lore_mini_obelisk_grass` | l36_59.jm2 | 0 | 2318,3805 |
| 29953 `lore_mini_obelisk_grass` | l39_44.jm2 | 0 | 2499,2832 |
| 29953 `lore_mini_obelisk_grass` | l43_43.jm2 | 0 | 2778,2765 |
| 29953 `lore_mini_obelisk_grass` | l45_55.jm2 | 0 | 2916,3520 |
| 29953 `lore_mini_obelisk_grass` | l46_49.jm2 | 0 | 3000,3193 |
| 29953 `lore_mini_obelisk_grass` | l53_54.jm2 | 0 | 3452,3491 |
| 29954 `lore_mini_obelisk_light_grass` | l34_48.jm2 | 0 | 2177,3135 |
| 29954 `lore_mini_obelisk_light_grass` | l36_49.jm2 | 0 | 2307,3197 |
| 29954 `lore_mini_obelisk_light_grass` | l38_53.jm2 | 0 | 2458,3442 |
| 29954 `lore_mini_obelisk_light_grass` | l38_48.jm2 | 0 | 2463,3087 |
| 29954 `lore_mini_obelisk_light_grass` | l39_52.jm2 | 0 | 2527,3360 |
| 29954 `lore_mini_obelisk_light_grass` | l39_50.jm2 | 0 | 2533,3255 |
| 29954 `lore_mini_obelisk_light_grass` | l39_46.jm2 | 0 | 2549,2962 |
| 29954 `lore_mini_obelisk_light_grass` | l41_49.jm2 | 0 | 2626,3155 |
| 29954 `lore_mini_obelisk_light_grass` | l48_54.jm2 | 0 | 3128,3515 |
| 29955 `lore_mini_obelisk_wilderness` | l42_67.jm2 | 0 | 2731,4307 |
| 29955 `lore_mini_obelisk_wilderness` | l51_61.jm2 | 0 | 3298,3922 |
| 29956 `lore_mini_obelisk_sos` | l29_81.jm2 | 0 | 1865,5242 |
| 29956 `lore_mini_obelisk_sos` | l51_68.jm2 | 0 | 3298,4353 |
| 29957 `lore_mini_obelisk_morytania` | l54_50.jm2 | 0 | 3479,3253 |
| 29958 `lore_mini_obelisk_dagannoth` | l25_82.jm2 | 0 | 1658,5259 |
| 29958 `lore_mini_obelisk_dagannoth` | l30_68.jm2 | 0 | 1935,4362 |
| 29958 `lore_mini_obelisk_dagannoth` | l38_158.jm2 | 0 | 2490,10152 |
| 29959 `lore_mini_obelisk_fairy` | l37_69.jm2 | 0 | 2405,4443 |

### Skill NPC spawns

| Npc | File | Level | x,z |
|---|---|---|---|
| 4472 `wolf_bogrog_ogre` | n39_47.jm2 | 0 | 2523,3057 |
| 6891 `lore_penguin_keeper` | n40_51.jm2 | 0 | 2598,3272 |
| 6892 `lore_petshop_owner_yanille` | n40_48.jm2 | 0 | 2576,3078 |
| 6893 `pet_shop_owner_6893` | n45_53.jm2 | 0 | 2922,3435 |
| 6894 `lore_pet_dog_terrier_puppy_pen` | n45_53.jm2 | 0 | 2928,3432 |
| 6896 `lore_pet_dog_dalmation_puppy_pen` | n40_48.jm2 | 0 | 2574,3082 |
| 6896 `lore_pet_dog_dalmation_puppy_pen` | n45_53.jm2 | 0 | 2927,3431 |
| 6897 `lore_pet_dog_bulldog_puppy_pen` | n40_48.jm2 | 0 | 2573,3082 |
| 6988 `npc_6988` | n45_53.jm2 | 0 | 2925,3443 |
| 6989 `wolf_druid_piccupstix_2` | n45_53.jm2 | 0 | 2925,3443 |
