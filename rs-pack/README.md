# rs-pack -- Cache Packing & Unpacking

This crate handles the complete pipeline for packing game assets from human-readable source files into binary cache
archives, and unpacking original Jagex rev-225 cache files back into editable content. It produces byte-identical output
verified by CRC against original archives.

## Table of Contents

- [Cache Providers](#cache-providers)
    - [Architecture](#architecture)
    - [Shared Enums](#shared-enums-rs-packsrctypesrs)
    - [Config Type Reference](#config-type-reference)
    - [Custom Providers](#custom-providers)
    - [Script System](#script-system-cachescriptrs)
    - [Loading Flow](#loading-flow)
    - [Source Code Reference](#source-code-reference)
- [Cache Unpacking](#cache-unpacking)
    - [Overview](#overview)
    - [Roundtrip Verification](#roundtrip-verification)
    - [Pack Registry Generation](#pack-registry-generation)
    - [Recolor Value Handling](#recolor-value-handling)
    - [Unpacking Implementation](#unpacking-implementation)
- [Jag Archive Format](#jag-archive-format)
    - [Binary Structure](#binary-structure)
    - [Compression Modes](#compression-modes)
    - [Archive Inventory](#archive-inventory)
    - [CRC Table](#crc-table)
- [Jag Archive Name Hashes](#jag-archive-name-hashes)
- [Config](#config)
    - [Config Packing](#config-packing)
    - [Config Unpacking](#config-unpacking-1)
- [Interface](#interface)
    - [Interface Packing](#interface-packing)
    - [Interface Unpacking](#interface-unpacking)
- [Map](#map)
    - [Map Packing](#map-packing)
    - [Map Unpacking](#map-unpacking)
- [Media (Sprites)](#media-sprites)
    - [Media Packing](#media-packing)
    - [Media Unpacking](#media-unpacking)
- [Model](#model)
    - [Model Packing](#model-packing)
    - [Model Unpacking](#model-unpacking)
- [Sound, Song, and Jingle](#sound-song-and-jingle)
    - [Sound Packing](#sound-packing)
    - [Sound Unpacking](#sound-unpacking)
- [Texture](#texture)
    - [Texture Packing](#texture-packing)
    - [Texture Unpacking](#texture-unpacking)
- [Title](#title)
    - [Title Packing](#title-packing)
    - [Title Unpacking](#title-unpacking)
- [Word Encoding](#word-encoding)
    - [WordEnc Packing](#wordenc-packing)
    - [WordEnc Unpacking](#wordenc-unpacking)

---

## Cache Providers

This section describes the `CacheStore` and all cache type providers in `rs-pack/src/cache/`. The cache layer decodes
packed binary data produced by
the [config packer](#config-packing), [interface packer](#interface-packing), [title packer](#title-packing), [wordenc packer](#wordenc-packing),
and [sound packer](#sound-packing) into typed Rust structs accessible at runtime.

### Architecture

#### CacheStore

`CacheStore` is the root struct holding all decoded game data. It is constructed by `pack_all()` in
`rs-pack/src/lib.rs`, leaked to `'static`, and shared across the engine and server.

```
rs-pack/src/cache/mod.rs -> CacheStore
rs-pack/src/cache/provider.rs -> CacheType trait, TypeProvider<T>
rs-pack/src/types.rs -> shared enums (ScriptVarType, MoveRestrict, BlockWalk, etc.)
```

#### Provider Patterns

There are three provider patterns used across the cache:

**1. TypeProvider\<T\> (opcode-based configs)**

Used by all standard config types. Binary format: `[p2(count)] [entry0] [entry1] ...` where each entry is a sequence of
`[p1(opcode) data...]` terminated by `p1(0)`.

The `CacheType` trait provides:

- `new(id: u16) -> Self` -- create with defaults
- `decode(&mut self, buf: &mut Packet)` -- parse opcodes from binary
- `post_decode(types: &mut Vec<Self>, ctx: &Self::Context)` -- cross-reference pass (e.g. obj certificates)
- `debugname(&self) -> Option<&str>` -- name for lookup

`TypeProvider::from_bytes(dat, ctx)` reads the count, decodes each entry, builds a `HashMap<Box<str>, u16>` name-to-id
map from debugnames, and calls `post_decode`.

Config types using this pattern: ObjType, NpcType, LocType, InvType, VarPlayerType, VarnType, VarsType, EnumType,
ParamType, StructType, SeqType, SpotAnimType, MesAnimType, IdkType, HuntType, CategoryType, FloType, DbRowType,
DbTableType.

**2. Custom sequential providers**

Used for data that doesn't follow the opcode-terminated format:

- `IfTypeProvider` -- interface components use a sequential binary format with group headers (`0xFFFF` markers). Loaded
  via `from_bytes()`.
- `FontTypeProvider` -- fonts are decoded from the title JAG archive's sprite data (`{name}.dat` + `index.dat`). Loaded
  via `from_jag()`.
- `MidiProvider` -- MIDI files are decompressed from bz2 and parsed for duration. Loaded via `from_compressed()`.
- `WordEncProvider` -- word filter data decoded from 4 files within the wordenc JAG. Loaded via `from_jag()`.

**3. Raw data**

Some data is stored without decoding:

- `jags: HashMap<&'static str, Arc<[u8]>>` -- raw JAG archives served to the client (Arc-shared, zero-copy for game
  protocol)
- `mapsquares: MapSquares` -- compressed map tiles (Arc-shared, zero-copy for game protocol)
- `scripts: HashMap<&'static str, Vec<u8>>` -- compiled RuneScript bytecode

---

### Shared Enums (`rs-pack/src/types.rs`)

All enums use `#[repr(u8)]` with `TryFromPrimitive` for binary decoding. Each has a `from_config_str(&str) -> Self`
method for pack config encoding. These are the single source of truth -- the engine re-exports them rather than defining
its own copies.

| Enum                  | Values                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Used by                                                                                     |
|-----------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| ScriptVarType         | Int=105, AutoInt=255, String=115, Enum=103, Obj=111, Loc=108, Component=73, NamedObj=79, Struct=74, Boolean=49, Coord=99, Category=121, Spotanim=116, Npc=110, Inv=118, Synth=80, Seq=65, Stat=83, Varp=86, PlayerUid=112, NpcUid=78, Interface=97, NpcStat=254, Idkit=75, DbRow=208 (25 variants)                                                                                                                                                                                                           | all config decoders, script system                                                          |
| ParamValue            | Int(i32), String(String)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | obj, loc, npc, struct params (stored in `Box<HashMap>` to minimize inline size when absent) |
| InvScope              | Temp=0, Perm=1, Shared=2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | inv                                                                                         |
| VarPlayerScope        | Temp=0, Perm=1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | varp                                                                                        |
| MoveRestrict          | Normal=0, Blocked=1, BlockedNormal=2, Indoors=3, Outdoors=4, NoMove=5, Passthru=6                                                                                                                                                                                                                                                                                                                                                                                                                            | npc, engine pathfinding                                                                     |
| BlockWalk             | None=0, All=1, Npc=2                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | npc, engine collision                                                                       |
| NpcMode               | None=0, Wander=1, Patrol=2, PlayerEscape=3, PlayerFollow=4, PlayerFace=5, PlayerFaceClose=6, OpPlayer1-5=7-11, ApPlayer1-5=12-16, OpLoc1-5=17-21, ApLoc1-5=22-26, OpObj1-5=27-31, ApObj1-5=32-36, OpNpc1-5=37-41, ApNpc1-5=42-46, Queue1-20=47-66 (67 variants)                                                                                                                                                                                                                                              | npc, hunt, engine AI                                                                        |
| HuntModeType          | Off=0, Player=1, Npc=2, Obj=3, Scenery=4                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | hunt                                                                                        |
| HuntCheckVis          | Off=0, LineOfSight=1, LineOfWalk=2                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | hunt                                                                                        |
| HuntCheckNotTooStrong | Off=0, OutsideWilderness=1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | hunt                                                                                        |
| HuntNobodyNear        | KeepHunting=0, PauseHunt=1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | hunt                                                                                        |
| HuntCheckNotBusy      | Off=0, On=1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | hunt                                                                                        |
| HuntFindKeepHunting   | Off=0, On=1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | hunt                                                                                        |
| HuntCheckAfk          | On=0, Off=1                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | hunt                                                                                        |
| WearPos               | Hat=0, Back=1, Front=2, RightHand=3, Torso=4, LeftHand=5, Arms=6, Legs=7, Head=8, Hands=9, Feet=10, Jaw=11, Ring=12, Quiver=13 (14 variants)                                                                                                                                                                                                                                                                                                                                                                 | obj                                                                                         |
| DummyItem             | None=0, GraphicOnly=1, InvOnly=2                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | obj                                                                                         |
| BodyType              | ManHair=0, ManJaw=1, ManTorso=2, ManArms=3, ManHands=4, ManLegs=5, ManFeet=6, WomanHair=7, WomanJaw=8, WomanTorso=9, WomanArms=10, WomanHands=11, WomanLegs=12, WomanFeet=13 (14 variants)                                                                                                                                                                                                                                                                                                                   | idk                                                                                         |
| LocShape              | WallStraight=0, WallDiagonalCorner=1, WallL=2, WallSquareCorner=3, WallDecorStraightNoOffset=4, WallDecorStraightOffset=5, WallDecorDiagonalOffset=6, WallDecorDiagonalNoOffset=7, WallDecorDiagonalBoth=8, WallDiagonal=9, CentrepieceStraight=10, CentrepieceDiagonal=11, RoofStraight=12, RoofDiagonalWithRoofEdge=13, RoofDiagonal=14, RoofLConcave=15, RoofLConvex=16, RoofFlat=17, RoofEdgeStraight=18, RoofEdgeDiagonalCorner=19, RoofEdgeL=20, RoofEdgeSquareCorner=21, GroundDecor=22 (23 variants) | loc packing, engine collision                                                               |
| ForceApproach         | None=0, North=14, East=13, South=11, West=7                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | loc                                                                                         |
| IfComponentType       | Layer=0, Inv=2, Rect=3, Text=4, Graphic=5, Model=6, InvText=7                                                                                                                                                                                                                                                                                                                                                                                                                                                | interface                                                                                   |
| IfButtonType          | None=0, Normal=1, Target=2, Close=3, Toggle=4, Select=5, Pause=6                                                                                                                                                                                                                                                                                                                                                                                                                                             | interface                                                                                   |
| Font                  | P11=0, P12=1, B12=2, Q8=3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | font, interface                                                                             |
| PlayerStat            | Attack=0, Defence=1, Strength=2, Hitpoints=3, Ranged=4, Prayer=5, Magic=6, Cooking=7, Woodcutting=8, Fletching=9, Fishing=10, Firemaking=11, Crafting=12, Smithing=13, Mining=14, Herblore=15, Agility=16, Thieving=17, Stat18=18, Stat19=19, Runecraft=20 (21 variants)                                                                                                                                                                                                                                     | interface scripts, engine stats                                                             |
| NpcStat               | Hitpoints=0, Attack=1, Strength=2, Defence=3, Magic=4, Ranged=5 (6 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                 | npc combat stats                                                                            |
| IfScriptOp            | StatLevel=1, StatBaseLevel=2, StatXp=3, InvCount=4, PushVar=5, StatXpRemaining=6, Op7=7, Op8=8, Op9=9, InvContains=10, RunEnergy=11, RunWeight=12, TestBit=13 (13 variants)                                                                                                                                                                                                                                                                                                                                  | interface scripts                                                                           |
| IfComparator          | Eq=1, Lt=2, Gt=3, Neq=4                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | interface scripts                                                                           |

---

### Config Type Reference

#### ObjType (`cache/obj.rs`)

Item definitions. CacheStore field: `objs`.

| Opcode  | Field                                    | Type                              | Default |
|---------|------------------------------------------|-----------------------------------|---------|
| 1       | model                                    | u16                               | 0       |
| 2       | name                                     | Option\<String\>                  | None    |
| 3       | desc                                     | Option\<String\>                  | None    |
| 4       | zoom2d                                   | u16                               | 2000    |
| 5       | xan2d                                    | u16                               | 0       |
| 6       | yan2d                                    | u16                               | 0       |
| 7       | xof2d                                    | i16                               | 0       |
| 8       | yof2d                                    | i16                               | 0       |
| 9       | code9                                    | bool                              | false   |
| 10      | code10                                   | Option\<u16\>                     | None    |
| 11      | stackable                                | bool                              | false   |
| 12      | cost                                     | i32                               | 1       |
| 13      | wearpos                                  | Option\<WearPos\>                 | None    |
| 14      | wearpos2                                 | Option\<WearPos\>                 | None    |
| 15      | tradeable=false                          | bool                              | false   |
| 16      | members                                  | bool                              | false   |
| 23      | manwear, manweary                        | u16, i8                           | None, 0 |
| 24      | manwear2                                 | Option\<u16\>                     | None    |
| 25      | womanwear, womanweary                    | u16, i8                           | None, 0 |
| 26      | womanwear2                               | Option\<u16\>                     | None    |
| 27      | wearpos3                                 | Option\<WearPos\>                 | None    |
| 30-34   | op[0-4]                                  | Option\<String\>                  | None    |
| 35-39   | iop[0-4]                                 | Option\<String\>                  | None    |
| 40      | recol_s, recol_d                         | Box\<[u16]\>                      | None    |
| 75      | weight                                   | i16                               | 0       |
| 78      | manwear3                                 | Option\<u16\>                     | None    |
| 79      | womanwear3                               | Option\<u16\>                     | None    |
| 90-93   | manhead, womanhead, manhead2, womanhead2 | Option\<u16\>                     | None    |
| 94      | category                                 | Option\<u16\>                     | None    |
| 95      | zan2d                                    | u16                               | 0       |
| 96      | dummyitem                                | DummyItem                         | None    |
| 97      | certlink                                 | Option\<u16\>                     | None    |
| 98      | certtemplate                             | Option\<u16\>                     | None    |
| 100-109 | countobj[0-9], countco[0-9]              | Vec\<u16\>                        | None    |
| 200     | tradeable=true                           | bool                              |         |
| 201     | respawnrate                              | u16                               | 100     |
| 249     | params                                   | Box\<HashMap\<i32, ParamValue\>\> | None    |
| 250     | debugname                                | Box\<str\>                        |         |

Post-decode: certificate items copy visual properties from their template and item properties from their link.

#### NpcType (`cache/npc.rs`)

NPC definitions. CacheStore field: `npcs`.

| Opcode | Field                                               | Type                              | Default |
|--------|-----------------------------------------------------|-----------------------------------|---------|
| 1      | models                                              | Box\<[u16]\>                      | None    |
| 2      | name                                                | Option\<String\>                  | None    |
| 3      | desc                                                | Option\<String\>                  | None    |
| 12     | size                                                | u8                                | 1       |
| 13     | readyanim                                           | Option\<u16\>                     | None    |
| 14     | walkanim                                            | Option\<u16\>                     | None    |
| 16     | hasalpha                                            | bool                              | false   |
| 17     | walkf, walkb, walkr, walkl                          | Option\<u16\>                     | None    |
| 18     | category                                            | Option\<u16\>                     | None    |
| 30-34  | op[0-4]                                             | Option\<String\>                  | None    |
| 40     | recol_s, recol_d                                    | Box\<[u16]\>                      | None    |
| 60     | head_models                                         | Box\<[u16]\>                      | None    |
| 74-79  | attack, defence, strength, hitpoints, ranged, magic | u16                               | 1       |
| 90-92  | resizex, resizey, resizez                           | u16                               | 128     |
| 93     | minimap=false                                       | bool                              | true    |
| 95     | vislevel                                            | u16                               | 1       |
| 97-98  | resizeh, resizev                                    | u16                               | 128     |
| 200    | wanderrange                                         | u16                               | 5       |
| 201    | maxrange                                            | u16                               | 7       |
| 202    | huntrange                                           | u8                                | 0       |
| 203    | timer                                               | u16                               | 0       |
| 204    | respawnrate                                         | u16                               | 100     |
| 206    | moverestrict                                        | MoveRestrict                      | Normal  |
| 207    | attackrange                                         | u16                               | 0       |
| 208    | blockwalk                                           | BlockWalk                         | Npc     |
| 209    | huntmode                                            | Option\<NpcMode\>                 | None    |
| 210    | defaultmode                                         | NpcMode                           | Wander  |
| 211    | members                                             | bool                              | false   |
| 212    | patrol                                              | Vec\<NpcPatrol\>                  | None    |
| 213    | givechase=false                                     | bool                              | true    |
| 214    | regenrate                                           | u16                               | 100     |
| 249    | params                                              | Box\<HashMap\<i32, ParamValue\>\> | None    |
| 250    | debugname                                           | Box\<str\>                        |         |

#### LocType (`cache/loc.rs`)

Location/scenery definitions. CacheStore field: `locs`.

| Opcode | Field            | Type                                           | Default |
|--------|------------------|------------------------------------------------|---------|
| 1      | models           | Box\<[LocModelShape]\> (model: u16, shape: u8) | None    |
| 2      | name             | Option\<String\>                               | None    |
| 3      | desc             | Option\<String\>                               | None    |
| 14     | width            | u8                                             | 1       |
| 15     | length           | u8                                             | 1       |
| 17     | blockwalk=false  | bool                                           | true    |
| 18     | blockrange=false | bool                                           | true    |
| 19     | active           | Option\<bool\>                                 | None    |
| 21     | hillskew=true    | bool                                           | false   |
| 22     | sharelight=true  | bool                                           | false   |
| 23     | occlude=true     | bool                                           | false   |
| 24     | anim             | Option\<u16\>                                  | None    |
| 25     | hasalpha=true    | bool                                           | false   |
| 28     | wallwidth        | u8                                             | 16      |
| 29     | ambient          | i8                                             | 0       |
| 30-34  | op[0-4]          | Option\<String\>                               | None    |
| 39     | contrast         | i8                                             | 0       |
| 40     | recol_s, recol_d | Box\<[u16]\>                                   | None    |
| 60     | mapfunction      | Option\<u16\>                                  | None    |
| 61     | category         | Option\<u16\>                                  | None    |
| 62     | mirror=true      | bool                                           | false   |
| 64     | shadow=false     | bool                                           | true    |
| 65     | resizex          | u16                                            | 128     |
| 66     | resizey          | u16                                            | 128     |
| 67     | resizez          | u16                                            | 128     |
| 68     | mapscene         | Option\<u16\>                                  | None    |
| 69     | forceapproach    | ForceApproach                                  | None    |
| 70     | offsetx          | i16                                            | 0       |
| 71     | offsety          | i16                                            | 0       |
| 72     | offsetz          | i16                                            | 0       |
| 73     | forcedecor=true  | bool                                           | false   |
| 249    | params           | Box\<HashMap\<i32, ParamValue\>\>              | None    |
| 250    | debugname        | Box\<str\>                                     |         |

#### HuntType (`cache/hunt.rs`)

NPC hunt behavior definitions. CacheStore field: `hunts`.

| Opcode | Field                | Type                                                                    | Default   |
|--------|----------------------|-------------------------------------------------------------------------|-----------|
| 1      | hunt_type            | HuntModeType                                                            | Off       |
| 2      | check_vis            | HuntCheckVis                                                            | Off       |
| 3      | check_nottoostrong   | HuntCheckNotTooStrong                                                   | Off       |
| 4      | check_notbusy=On     | HuntCheckNotBusy                                                        | Off       |
| 5      | find_keephunting=On  | HuntFindKeepHunting                                                     | Off       |
| 6      | find_newmode         | NpcMode                                                                 | None      |
| 7      | nobodynear           | HuntNobodyNear                                                          | PauseHunt |
| 8      | check_notcombat      | Option\<u16\>                                                           | None      |
| 9      | check_notcombat_self | Option\<u16\>                                                           | None      |
| 10     | check_afk=Off        | HuntCheckAfk                                                            | On        |
| 11     | rate                 | u16                                                                     | 1         |
| 12     | check_category       | Option\<u16\>                                                           | None      |
| 13     | check_npc            | Option\<u16\>                                                           | None      |
| 14     | check_obj            | Option\<u16\>                                                           | None      |
| 15     | check_loc            | Option\<u16\>                                                           | None      |
| 16     | check_inv            | HuntCheckInv (inv: u16, obj: u16, condition: String, value: i32)        | None      |
| 17     | check_invparam       | HuntCheckInvParam (inv: u16, param: u16, condition: String, value: i32) | None      |
| 18-20  | extracheck_vars      | Vec\<HuntExtraCheckVar\> (varp: u16, condition: String, value: i32)     | []        |
| 250    | debugname            | Box\<str\>                                                              |           |

#### InvType (`cache/inv.rs`)

Inventory definitions. CacheStore field: `invs`.

| Opcode | Field                           | Type                               | Default |
|--------|---------------------------------|------------------------------------|---------|
| 1      | scope                           | InvScope                           | Temp    |
| 2      | size                            | u16                                | 1       |
| 3      | stackall=true                   | bool                               | false   |
| 4      | stockobj, stockcount, stockrate | Vec\<u16\>, Vec\<u16\>, Vec\<i32\> | None    |
| 5      | restock=true                    | bool                               | false   |
| 6      | allstock=true                   | bool                               | false   |
| 7      | protect=false                   | bool                               | true    |
| 8      | runweight=true                  | bool                               | false   |
| 9      | dummyinv=true                   | bool                               | false   |
| 250    | debugname                       | Box\<str\>                         |         |

#### VarPlayerType (`cache/varp.rs`)

Player variable definitions. CacheStore field: `varps`.

| Opcode | Field         | Type           | Default |
|--------|---------------|----------------|---------|
| 1      | scope         | VarPlayerScope | Temp    |
| 2      | var_type      | ScriptVarType  | Int     |
| 4      | protect=false | bool           | true    |
| 5      | clientcode    | u16            | 0       |
| 6      | transmit=true | bool           | false   |
| 250    | debugname     | Box\<str\>     |         |

#### SeqType (`cache/seq.rs`)

Animation sequences. CacheStore field: `seqs`.

| Opcode | Field                   | Type                                     | Default |
|--------|-------------------------|------------------------------------------|---------|
| 1      | frames, iframes, delays | Box\<[u16]\>, Box\<[u16]\>, Box\<[u16]\> | None    |
| 2      | loops                   | Option\<u16\>                            | None    |
| 3      | walkmerge               | Box\<[u8]\>                              | None    |
| 4      | stretches=true          | bool                                     | false   |
| 5      | priority                | u8                                       | 5       |
| 6      | replaceheldleft         | Option\<u16\>                            | None    |
| 7      | replaceheldright        | Option\<u16\>                            | None    |
| 8      | maxloops                | u8                                       | 99      |
| 250    | debugname               | Box\<str\>                               |         |

#### VarnType (`cache/varn.rs`)

NPC variable definitions. CacheStore field: `varns`.

| Opcode | Field     | Type          | Default |
|--------|-----------|---------------|---------|
| 1      | var_type  | ScriptVarType | Int     |
| 250    | debugname | Box\<str\>    |         |

#### VarsType (`cache/vars.rs`)

Shared variable definitions. CacheStore field: `varss`.

| Opcode | Field     | Type          | Default |
|--------|-----------|---------------|---------|
| 1      | var_type  | ScriptVarType | Int     |
| 250    | debugname | Box\<str\>    |         |

#### EnumType (`cache/enum.rs`)

Enum lookup tables. CacheStore field: `enums`.

| Opcode | Field           | Type                                        | Default |
|--------|-----------------|---------------------------------------------|---------|
| 1      | inputtype       | ScriptVarType                               | Int     |
| 2      | outputtype      | ScriptVarType                               | Int     |
| 3      | default_str     | Option\<String\>                            | None    |
| 4      | default_int     | i32                                         | 0       |
| 5      | values (string) | HashMap\<i32, ParamValue\> (String entries) | {}      |
| 6      | values (int)    | HashMap\<i32, ParamValue\> (Int entries)    | {}      |
| 250    | debugname       | Box\<str\>                                  |         |

#### ParamType (`cache/param.rs`)

Parameter definitions. CacheStore field: `params`.

| Opcode | Field             | Type             | Default |
|--------|-------------------|------------------|---------|
| 1      | var_type          | ScriptVarType    | Int     |
| 2      | default_int       | i32              | -1      |
| 4      | autodisable=false | bool             | true    |
| 5      | default_str       | Option\<String\> | None    |
| 250    | debugname         | Box\<str\>       |         |

#### StructType (`cache/struct.rs`)

Struct definitions. CacheStore field: `structs`.

| Opcode | Field     | Type                              | Default |
|--------|-----------|-----------------------------------|---------|
| 249    | params    | Box\<HashMap\<i32, ParamValue\>\> | None    |
| 250    | debugname | Box\<str\>                        |         |

#### SpotAnimType (`cache/spotanim.rs`)

Spot animation (graphic) definitions. CacheStore field: `spotanims`.

| Opcode | Field         | Type          | Default |
|--------|---------------|---------------|---------|
| 1      | model         | u16           | 0       |
| 2      | anim          | Option\<u16\> | None    |
| 3      | hasalpha=true | bool          | false   |
| 4      | resizeh       | u16           | 128     |
| 5      | resizev       | u16           | 128     |
| 6      | angle         | u16           | 0       |
| 7      | ambient       | u8            | 0       |
| 8      | contrast      | u8            | 0       |
| 40-49  | recol_s[0-9]  | Vec\<u16\>    | None    |
| 50-59  | recol_d[0-9]  | Vec\<u16\>    | None    |
| 250    | debugname     | Box\<str\>    |         |

#### MesAnimType (`cache/mesanim.rs`)

Message animation definitions. CacheStore field: `mesanims`.

| Opcode | Field     | Type          | Default |
|--------|-----------|---------------|---------|
| 1-4    | len[0-3]  | Option\<u16\> | None    |
| 250    | debugname | Box\<str\>    |         |

#### IdkType (`cache/idk.rs`)

Identity kit definitions. CacheStore field: `idks`.

| Opcode | Field        | Type         | Default |
|--------|--------------|--------------|---------|
| 1      | body_type    | BodyType     | ManHair |
| 2      | models       | Box\<[u16]\> | None    |
| 3      | disable=true | bool         | false   |
| 40-49  | recol_s[0-9] | Vec\<u16\>   | None    |
| 50-59  | recol_d[0-9] | Vec\<u16\>   | None    |
| 60-69  | heads[0-9]   | Vec\<u16\>   | None    |
| 250    | debugname    | Box\<str\>   |         |

#### FloType (`cache/flo.rs`)

Floor overlay/underlay definitions. CacheStore field: `flos`.

| Opcode | Field         | Type         | Default |
|--------|---------------|--------------|---------|
| 1      | colour        | i32 (g3)     | 0       |
| 2      | texture       | Option\<u8\> | None    |
| 3      | overlay=true  | bool         | false   |
| 5      | occlude=false | bool         | true    |
| 6      | debugname     | Box\<str\>   |         |

#### CategoryType (`cache/category.rs`)

Category definitions. CacheStore field: `categories`.

| Opcode | Field     | Type       | Default |
|--------|-----------|------------|---------|
| 1      | debugname | Box\<str\> |         |

#### DbRowType (`cache/dbrow.rs`)

Database row definitions. CacheStore field: `dbrows`.

| Opcode | Field          | Type                                                 | Default |
|--------|----------------|------------------------------------------------------|---------|
| 3      | types, columns | Vec\<Vec\<u8\>\>, Vec\<Option\<Vec\<DbRowValue\>\>\> | [], []  |
| 4      | table          | u16                                                  | 0       |
| 250    | debugname      | Box\<str\>                                           |         |

Opcode 3 format: `[p1(column_count)]` then repeating `[p1(column_index) p1(type_count) types... values...]` terminated
by `0xFF`. Values are decoded per-type (string for ScriptVarType::String, i32 otherwise).

#### DbTableType (`cache/dbtable.rs`)

Database table definitions. CacheStore field: `dbtables`.

| Opcode | Field           | Type                                                   | Default |
|--------|-----------------|--------------------------------------------------------|---------|
| 1      | types, defaults | Vec\<Vec\<u8\>\>, Vec\<Option\<Vec\<DbTableValue\>\>\> | [], []  |
| 250    | debugname       | Box\<str\>                                             |         |
| 251    | columns         | Vec\<String\>                                          | []      |
| 252    | props           | Vec\<u8\>                                              | []      |

Opcode 1 format: `[p1(column_count)]` then repeating `[p1(info)]` where `info & 0x7F` = column index, `info & 0x80` =
has default values. Each column reads `[p1(type_count) types...]` and optionally default values. Terminated by `0xFF`.

---

### Custom Providers

#### IfType (`cache/if.rs`)

Interface components. CacheStore field: `interfaces`.

Uses `IfTypeProvider` with sequential binary format (not opcode-terminated). Stored as `Vec<Option<Box<IfType>>>` --
boxing the large IfType struct (~480 bytes) so that empty slots cost only 8 bytes. Groups start with `0xFFFF` marker
followed by `root_layer` (u16). Each component reads fields sequentially:

**Common header (all components):**

| Order | Field                             | Type                  | Default |
|-------|-----------------------------------|-----------------------|---------|
| 1     | com_name                          | String                | None    |
| 2     | overlay                           | bool (u8)             | false   |
| 3     | com_type                          | IfComponentType (u8)  | Layer   |
| 4     | button_type                       | IfButtonType (u8)     | None    |
| 5     | client_code                       | u16                   | 0       |
| 6     | width                             | u16                   | 0       |
| 7     | height                            | u16                   | 0       |
| 8     | over_layer                        | u8 (+u8 if non-zero)  | -1      |
| 9     | script_comparator, script_operand | Vec\<u8\>, Vec\<u16\> | None    |
| 10    | scripts                           | Vec\<Vec\<u16\>\>     | None    |

**Type-specific fields (by IfComponentType):**

| ComponentType | Fields                                                                                                                                                 |
|---------------|--------------------------------------------------------------------------------------------------------------------------------------------------------|
| Layer         | scroll (u16), hide (bool), child_x/child_y (Vec\<i16\>)                                                                                                |
| Inv           | draggable, interactable, usable (bool), margin_x/y (u8), slot_offset_x/y (20x i16), slot_graphic (20x String), inventory_options (5x Option\<String\>) |
| Rect          | fill (bool)                                                                                                                                            |
| Text          | center (bool), font (u8), shadowed (bool), text (String), active_text (String)                                                                         |
| Graphic       | graphic (String), active_graphic (String)                                                                                                              |
| Model         | model, active_model, anim, active_anim (encoded i32), zoom (u16), xan (u16), yan (u16)                                                                 |
| InvText       | center (bool), font (u8), shadowed (bool), colour (i32), margin_x/y (i16), interactable (bool), inventory_options (5x Option\<String\>)                |

**Conditional trailing fields:**

| Condition                                | Fields                                                     |
|------------------------------------------|------------------------------------------------------------|
| Rect or Text                             | colour (i32), active_colour (i32), over_colour (i32)       |
| ButtonType=Target or ComponentType=Inv   | action_verb (String), action (String), action_target (u16) |
| ButtonType=Normal\|Toggle\|Select\|Pause | option (String)                                            |

#### FontType (`cache/font.rs`)

Bitmap font metrics. CacheStore field: `fonts`.

Loaded from the title JAG archive via `FontTypeProvider::from_jag()`. Decodes 4 fonts (P11, P12, B12, Q8) identified by
`FontId` enum. Each font reads glyph dimensions from `index.dat` and pixel data from `{name}.dat`, computing character
advances and draw widths.

Provides `string_width(&str) -> u32` for measuring text (skips `@col@` colour tags) and
`split(&str, max_width) -> Vec<String>` for word-wrapping.

#### MidiType (`cache/midi.rs`)

MIDI duration metadata. CacheStore fields: `songs`, `jingles`.

`MidiProvider::from_compressed()` takes ownership of the song/jingle `HashMap<String, Vec<u8>>`, decompresses each
entry (bz2 with 4-byte size header), and parses the MIDI to extract duration in milliseconds. Names are stored as
`Box<str>` with a `HashMap<Box<str>, usize>` lookup. The compressed bytes are retained in `MidiType.data` for HTTP
serving.

Handles RIFF-wrapped MIDIs (`attack1.mid`), multi-track format 0/1/2, tempo change events (meta event 0x51), SMPTE
timing, and variable-length delta encoding.

`tick_length()` converts milliseconds to game ticks: `ceil(ms / 600) + 1`.

#### WordEncProvider (`cache/wordenc.rs`)

Chat filter data. CacheStore field: `wordenc`.

Decodes 4 files from the wordenc JAG: bad words with character substitution pairs, fragment values for IP detection, TLD
entries with classification types, and domain filter strings.

Provides `filter(&str) -> String` which runs the full RuneScape word filter pipeline: TLD filtering, bad word
detection (with leet-speak substitution like `@`=`a`, `$`=`s`, `0`=`o`), domain filtering, IP fragment filtering,
whitelist restoration, and uppercase formatting.

---

### Script System (`cache/script.rs`)

Compiled RuneScript bytecode. Stored in CacheStore as raw bytes (`scripts["script.dat"]`, `scripts["script.idx"]`).

`ScriptProvider::from_bytes(dat, idx)` decodes the paired files into `Script` structs wrapped in `Arc<Script>`. Each
Script contains:

| Field                 | Type                         | Notes                                                       |
|-----------------------|------------------------------|-------------------------------------------------------------|
| opcodes               | Box\<[u16]\>                 | Instruction stream                                          |
| int_operands          | Box\<[i32]\>                 | Integer operands per instruction                            |
| string_operands       | HashMap\<u16, Box\<str\>\>   | Sparse: only PUSH_CONSTANT_STRING instructions have entries |
| switch_tables         | Box\<[HashMap\<i32, i32\>]\> | Jump tables for switch statements                           |
| info.name             | Box\<str\>                   | Script name (e.g. "[proc,heal]")                            |
| info.path             | Box\<str\>                   | Source file path                                            |
| info.pcs / info.lines | Box\<[i32]\>                 | PC-to-line-number mapping for error reporting               |

String operands use a sparse HashMap keyed by instruction index instead of a full-width Vec, avoiding 24-byte empty
String allocations for non-string instructions.

---

### Loading Flow

```
pack_all() in rs-pack/src/lib.rs:
  1. Compile RuneScript sources -> script.dat/script.idx
  2. Pack all config types -> assets HashMap<String, PackedFile>
  3. Assemble JAG archives (config, interface, media, textures, title, models, sounds, wordenc)
  4. Pack songs and jingles (individual compressed files)
  5. Pack maps (individual compressed tiles)
  6. Build TypeProviders from server-side packed data
  7. Build custom providers (interfaces, fonts, wordenc, midi)
  8. Construct CacheStore with all providers
```

The CacheStore is then leaked to `'static` in the server's `main()` and passed to the engine's `World` and network
`PlayerHandle`.

---

### Source Code Reference

| Component                      | Path                                          |
|--------------------------------|-----------------------------------------------|
| CacheStore definition          | `rs-pack/src/cache/mod.rs`                    |
| CacheType trait / TypeProvider | `rs-pack/src/cache/provider.rs`               |
| Shared enums                   | `rs-pack/src/types.rs`                        |
| Pack-all orchestrator          | `rs-pack/src/lib.rs`                          |
| Config packers                 | `rs-pack/src/pack/config/*.rs`                |
| Interface packer               | `rs-pack/src/pack/interface/mod.rs`           |
| Title packer                   | `rs-pack/src/pack/title/mod.rs`               |
| WordEnc packer                 | `rs-pack/src/pack/wordenc/mod.rs`             |
| Song/Jingle packers            | `rs-pack/src/pack/other/song.rs`, `jingle.rs` |
| Map packer                     | `rs-pack/src/pack/other/map.rs`               |

---

## Cache Unpacking

This section describes the process of extracting all game assets from original Jagex rev-225 cache files into editable
content files that can be re-packed into byte-identical archives. The unpacking process requires no external metadata --
all naming, ordering, and registry files are derived from the binary data itself.

### Overview

The unpacking pipeline reads original cache files from the `expected/` directory and produces a self-contained
`content_unpack/` directory with all source files, pack registries, and ordering files needed for re-packing.

#### Commands

```
cargo unpack                    # Unpack expected/ -> content_unpack/
cargo verify                    # Verify roundtrip CRC match
```

#### Input

```
expected/
├── config          -- config Jag archive (511217062)
├── interface       -- interface Jag archive (1614084464)
├── media           -- media Jag archive (-343404987)
├── models          -- models Jag archive (-2000991154)
├── sounds          -- sounds Jag archive (-1532605973)
├── textures        -- textures Jag archive (1703545114)
├── title           -- title Jag archive (-430779560)
├── wordenc         -- wordenc Jag archive (1570981179)
├── maps/           -- individual compressed map files
│   ├── m50_50      -- terrain for map square (50, 50)
│   ├── l50_50      -- locations for map square (50, 50)
│   └── ...
└── songs/          -- individual compressed MIDI files
    ├── harmony_mid
    └── ...
```

#### Output

```
content_unpack/
├── pack/                    -- generated registry and ordering files
│   ├── obj.pack             -- obj ID-to-name mapping
│   ├── npc.pack             -- npc ID-to-name mapping
│   ├── model.pack           -- model ID-to-name mapping
│   ├── model.order          -- model packing order
│   ├── anim.pack            -- animation frame ID-to-name mapping
│   ├── anim.order           -- frame packing order
│   ├── base.pack            -- skeleton base ID-to-name mapping
│   ├── base.order           -- base packing order
│   ├── synth.pack           -- synth ID-to-name mapping
│   ├── synth.order          -- synth packing order
│   ├── texture.pack         -- texture ID-to-name mapping
│   ├── seq.pack, loc.pack, flo.pack, idk.pack, spotanim.pack, varp.pack
│   └── category.pack
├── all.obj                  -- obj config text (excludes cert entries)
├── all.npc                  -- npc config text
├── all.loc                  -- loc config text
├── all.seq                  -- seq config text
├── all.flo                  -- flo config text
├── all.idk                  -- idk config text
├── all.spotanim             -- spotanim config text
├── all.varp                 -- varp config text
├── sprites/                 -- media sprite TGAs
├── textures/                -- texture sprite TGAs
├── fonts/                   -- font sprite TGAs
├── title/                   -- title sprite TGAs
├── binary/title.jpg         -- title background JPEG
├── models/                  -- extracted model files
├── synth/                   -- extracted synth instrument files
├── wordenc/                 -- chat filter text files
├── songs/                   -- decompressed MIDI files
├── maps/                    -- decoded map text files (.jm2)
└── _raw/                    -- raw Jag entries for CRC verification
    ├── config/              -- raw config dat/idx pairs
    └── interface/           -- raw interface data entry
```

### Roundtrip Verification

The verify command re-packs each asset type from the unpacked content and compares the CRC against the original expected
file. All 10 asset types must produce byte-identical output:

| Asset     | Verification Method                                            |
|-----------|----------------------------------------------------------------|
| config    | Raw dat/idx reassembly into Jag (per-entry compression)        |
| interface | Raw data entry reassembly into Jag (whole-archive compression) |
| media     | Full TGA -> indexed sprite re-encoding                         |
| title     | Full TGA -> indexed sprite re-encoding + raw JPEG              |
| textures  | Full TGA -> indexed sprite re-encoding                         |
| wordenc   | Full text -> binary re-encoding                                |
| sounds    | Full synth file -> sounds.dat re-concatenation                 |
| models    | Raw stream reassembly into Jag                                 |
| maps      | Full .jm2 text -> binary terrain/loc re-encoding + compression |
| songs     | Full MIDI -> bzip2 re-compression                              |

### Pack Registry Generation

All `.pack` files are generated from the binary data during unpacking. No external registry files are required.

#### Config Type Registries

For each config type, a `.pack` file is generated mapping sequential IDs (0 through max, ascending) to generated names:

```
0=npc_0
1=npc_1
2=npc_2
...
```

FLO entries are a special case -- they use the debug name transmitted via opcode 6 in the binary instead of a generic
`flo_{id}` name:

```
0=cliff
1=cliff2
2=cliff3
...
5=water
6=gungywater
```

FLO entries that lack an opcode 6 debug name fall back to `flo_{id}`.

#### Cert Object Handling

OBJ entries that contain only `certlink` + `certtemplate` opcodes are identified as certificate objects. These are:

- Named `cert_obj_{linked_id}` in `obj.pack` (e.g., `7=cert_obj_6`)
- Excluded from the `all.obj` text file (the packer auto-generates them from the `cert_` prefix)
- The cert template entry (typically ID 799) is named `template_for_cert`

#### Model Name Registry

Model names are derived from how each model ID is referenced in config entries:

| Config Context                         | Name Pattern                 | Example              |
|----------------------------------------|------------------------------|----------------------|
| NPC body (opcode 1)                    | `model_{id}_npc`             | `model_152_npc`      |
| NPC head (opcode 60)                   | `model_{id}_npc_head`        | `model_0_npc_head`   |
| OBJ inventory (opcode 1)               | `model_{id}_obj`             | `model_1105_obj`     |
| OBJ wear (opcodes 23-26, 78-79, 90-93) | `model_{id}_obj_wear`        | `model_28_obj_wear`  |
| IDK body (opcode 2)                    | `model_{id}_idk`             | `model_151_idk`      |
| IDK head (opcodes 60-69)               | `model_{id}_idk_head`        | `model_105_idk_head` |
| SPOTANIM (opcode 1)                    | `model_{id}_spotanim`        | `model_411_spotanim` |
| LOC (opcode 1)                         | `model_loc_{loc_id}_{shape}` | `model_loc_3_3`      |
| No config reference                    | `model_{id}`                 | `model_42`           |

Config types are processed in priority order: IDK -> OBJ -> NPC -> SPOTANIM -> LOC. The first config type to reference a
model determines its name.

#### Cross-Reference Registries

Config entries reference other types by ID. These cross-referenced IDs are tracked and their `.pack` files generated
with ascending key order (0 through max_id):

- `seq.pack` -- sequence IDs referenced by NPC walkanim, LOC anim, OBJ code10, SPOTANIM anim
- `anim.pack` -- animation frame IDs from the frame_head stream
- `base.pack` -- skeleton base IDs from the base_head stream
- `synth.pack` -- synth instrument IDs from sounds.dat
- `texture.pack` -- texture IDs referenced by FLO texture entries, using a hardcoded name table that maps IDs to real
  texture names (e.g., `0=door`, `3=planks`, `31=lava`)

### Recolor Value Handling

Config types store recolor values differently depending on the type:

#### OBJ and NPC (conditional conversion)

The packer converts RGB15 values >= 100 to HSL16 via `rgb15_to_hsl16()`. Values < 100 are stored unchanged. To reverse:

- If both source and dest values in the binary are < 100: use as-is (no conversion was applied)
- If either value is >= 100: reverse-lookup the HSL16 value to find the original RGB15

A precomputed reverse table maps all 32,768 possible HSL16 outputs back to their RGB15 inputs.

#### LOC (always converted)

LOC recolor values are always converted through `rgb15_to_hsl16()`. The binary stores raw HSL16 values which are written
directly to the text config (they remain < 100 threshold during re-pack, avoiding double-conversion).

#### IDK and SPOTANIM (always converted, individual opcodes)

Each recolor slot uses a separate opcode (40-49 for source, 50-59 for dest). Values are always `rgb15_to_hsl16()`
converted. The reverse table recovers the original RGB15 values.

### Unpacking Implementation

- **Entry point**: `rs-pack/src/unpack/mod.rs` -- `unpack_all` function
- **Config decoder**: `rs-pack/src/unpack/config.rs` -- code-based binary -> text conversion
- **Sprite decoder**: `rs-pack/src/unpack/mod.rs` (`decode_group` + `write_group_sheet`) -- indexed pixel -> per-group indexed sprite-sheet TGA
- **Synth parser**: `rs-pack/src/unpack/sound.rs` -- SoundEffect/Tone/Envelope format parser
- **Verification**: `rs-pack/src/unpack/verify.rs` -- per-type CRC comparison

---

## Jag Archive Format

The Jag (Java Archive Group) format is the container format used by the RuneScape rev-225 client to bundle multiple
files into a single archive. Each cache archive (config, interface, media, models, textures, wordenc, sounds, title) is
a Jag file.

### Binary Structure

Header (6 bytes):

```
p3(unpacked_size)    -- total uncompressed size of the file table + data
p3(packed_size)      -- compressed size (equals unpacked_size if not whole-archive compressed)
```

If packed_size != unpacked_size, the entire payload is bzip2 compressed (whole-archive mode). Otherwise, individual
entries are compressed independently.

After decompression of the payload:

```
p2(file_count)       -- number of files in the archive

File table (file_count * 10 bytes):
for each file:
    p4(name_hash)    -- hash of the filename
    p3(unpacked_size) -- uncompressed size of this file
    p3(packed_size)   -- compressed size of this file

File data follows the table sequentially.
```

#### Filename Hashing

The hash function for file names:

```
hash = 0
for each character (uppercase):
    hash = hash * 61 + (char_code - 32)
```

Result is a signed 32-bit integer (i32).

### Compression Modes

- **Per-entry compression**: Each file is individually bzip2 compressed. Used by most archives.
- **Whole-archive compression**: The entire payload (file table + all data) is compressed as one block. Used only by the
  interface archive.

The packing tool automatically selects the mode that produces the smaller output. This is deterministic (same bzip2
parameters always produce the same result) and matches the original Jagex archives byte-for-byte.

#### Bzip2 Details

- Block size: 100k (parameter 1)
- Uses the original bzip2-1.0.8 C library (via rs-bzip2 crate) for byte-identical output
- The 4-byte BZh1 header is stripped when storing compressed data

### Archive Inventory

All CRC values are from the original Jagex rev-225 cache files in the expected/ directory.

| Archive   | CRC         | Compression   | Contents                                                        |
|-----------|-------------|---------------|-----------------------------------------------------------------|
| title     | -430779560  | Per-entry     | 4 fonts + 4 title sprites + title.jpg + shared index.dat        |
| config    | 511217062   | Per-entry     | seq, loc, flo, spotanim, obj, npc, idk, varp (paired .dat/.idx) |
| interface | 1614084464  | Whole-archive | Single "data" entry with all UI components                      |
| media     | -343404987  | Per-entry     | Sprite pixel data + shared index.dat                            |
| models    | -2000991154 | Per-entry     | 21 data streams for meshes, frames, and bases                   |
| textures  | 1703545114  | Per-entry     | Texture pixel data + shared index.dat                           |
| wordenc   | 1570981179  | Per-entry     | 4 chat filter data files                                        |
| sounds    | -1532605973 | Per-entry     | Single sounds.dat with all synth instruments                    |

#### Non-Jag Assets

These assets are NOT Jag archives but are still packed by the system:

- **Songs** -- individual bzip2-compressed MIDI files, served on demand
- **Jingles** -- individual bzip2-compressed MIDI files, served on demand
- **Maps** -- individual bzip2-compressed terrain/loc/npc/obj data per map square

### CRC Table

The server sends a CRC table to the client for cache validation. It contains 9 entries (index 0 unused):

| Index | Archive            |
|-------|--------------------|
| 0     | (unused, always 0) |
| 1     | title              |
| 2     | config             |
| 3     | interface          |
| 4     | media              |
| 5     | models             |
| 6     | textures           |
| 7     | wordenc            |
| 8     | sounds             |

---

## Jag Archive Name Hashes

Resolved 158/158 entry name hashes using `JagFile::hash` against known entry names.

| Archive   | Entry | Hash        | Unpacked | Packed | Name                  |
|-----------|-------|-------------|----------|--------|-----------------------|
| title     | 0     | 788735113   | 13117    | 452    | p11.dat               |
| title     | 1     | 802580954   | 14608    | 607    | p12.dat               |
| title     | 2     | -1891508522 | 72002    | 14491  | titlebox.dat          |
| title     | 3     | -566502255  | 17803    | 17634  | title.dat             |
| title     | 4     | -1668775416 | 65746    | 2117   | runes.dat             |
| title     | 5     | -1228840272 | 16587    | 864    | q8.dat                |
| title     | 6     | -1929337337 | 3685     | 1049   | index.dat             |
| title     | 7     | 1955686745  | 6029     | 1321   | titlebutton.dat       |
| title     | 8     | -1752651416 | 63050    | 12767  | logo.dat              |
| title     | 9     | 1071845628  | 15436    | 706    | b12.dat               |
| config    | 0     | 886159288   | 91841    | 20633  | seq.dat               |
| config    | 1     | 886178080   | 2056     | 845    | seq.idx               |
| config    | 2     | 682978269   | 139320   | 36690  | loc.dat               |
| config    | 3     | 682997061   | 6776     | 2560   | loc.idx               |
| config    | 4     | -1569261396 | 1320     | 719    | flo.dat               |
| config    | 5     | -1569242604 | 158      | 91     | flo.idx               |
| config    | 6     | -955170442  | 3569     | 1453   | spotanim.dat          |
| config    | 7     | -955151650  | 490      | 158    | spotanim.idx          |
| config    | 8     | -1667617738 | 150744   | 39591  | obj.dat               |
| config    | 9     | -1667598946 | 5774     | 1778   | obj.idx               |
| config    | 10    | 1489108188  | 96891    | 26768  | npc.dat               |
| config    | 11    | 1489126980  | 2036     | 1011   | npc.idx               |
| config    | 12    | 150819851   | 692      | 318    | idk.dat               |
| config    | 13    | 150838643   | 166      | 59     | idk.idx               |
| config    | 14    | 383739196   | 321      | 60     | varp.dat              |
| config    | 15    | 383757988   | 592      | 50     | varp.idx              |
| interface | 0     | 8297314     | 297761   | 297761 | data                  |
| media     | 0     | -1868599050 | 4354     | 1485   | combatboxes.dat       |
| media     | 1     | 661681639   | 8446     | 1721   | staticons.dat         |
| media     | 2     | 22834782    | 2725     | 1073   | gnomeball_buttons.dat |
| media     | 3     | -1823467094 | 12170    | 713    | miscgraphics2.dat     |
| media     | 4     | -1809621253 | 1252     | 502    | miscgraphics3.dat     |
| media     | 5     | 1354546316  | 2674     | 421    | backleft1.dat         |
| media     | 6     | 1368392157  | 2114     | 517    | backleft2.dat         |
| media     | 7     | -1000916878 | 5282     | 540    | tradebacking.dat      |
| media     | 8     | 1043559214  | 3002     | 366    | steelborder.dat       |
| media     | 9     | 392041951   | 11656    | 2321   | prayeron.dat          |
| media     | 10    | -709488597  | 122      | 60     | mapflag.dat           |
| media     | 11    | -427405255  | 2603     | 357    | compass.dat           |
| media     | 12    | 1644583778  | 26882    | 2613   | mapback.dat           |
| media     | 13    | -288954319  | 5332     | 970    | headicons.dat         |
| media     | 14    | 839488367   | 3115     | 771    | mapscene.dat          |
| media     | 15    | 1758274153  | 11252    | 191    | staticons2.dat        |
| media     | 16    | 529843337   | 1378     | 213    | cross.dat             |
| media     | 17    | 661178691   | 14538    | 3006   | magicoff.dat          |
| media     | 18    | -1448902313 | 19942    | 307    | magicon2.dat          |
| media     | 19    | 2081559868  | 13590    | 3133   | miscgraphics.dat      |
| media     | 20    | -1929337337 | 7461     | 4135   | index.dat             |
| media     | 21    | -869490323  | 14538    | 3111   | magicon.dat           |
| media     | 22    | 53973365    | 11896    | 1860   | combaticons.dat       |
| media     | 23    | 612871759   | 82       | 92     | mapdots.dat           |
| media     | 24    | -1102299012 | 6173     | 958    | backtop1.dat          |
| media     | 25    | -1088453171 | 1142     | 311    | backtop2.dat          |
| media     | 26    | 1766681864  | 45986    | 3512   | chatback.dat          |
| media     | 27    | 125902192   | 30563    | 5415   | backbase1.dat         |
| media     | 28    | 139748033   | 11522    | 2779   | backbase2.dat         |
| media     | 29    | -1502153170 | 1658     | 401    | hitmarks.dat          |
| media     | 30    | 1889496696  | 7692     | 1916   | sideicons.dat         |
| media     | 31    | -1571073093 | 514      | 110    | scrollbar.dat         |
| media     | 32    | 1694123055  | 1158     | 200    | prayerglow.dat        |
| media     | 33    | -1392068576 | 1226     | 313    | redstone1.dat         |
| media     | 34    | -1378222735 | 1112     | 319    | redstone2.dat         |
| media     | 35    | -1364376894 | 1542     | 419    | redstone3.dat         |
| media     | 36    | -1623648789 | 17756    | 3799   | backhmid1.dat         |
| media     | 37    | -1609802948 | 16862    | 2890   | backhmid2.dat         |
| media     | 38    | -952192193  | 13134    | 1774   | combaticons2.dat      |
| media     | 39    | -938346352  | 14811    | 851    | combaticons3.dat      |
| media     | 40    | 1464846521  | 6316     | 1268   | backvmid1.dat         |
| media     | 41    | 1478692362  | 4790     | 1295   | backvmid2.dat         |
| media     | 42    | 1492538203  | 7139     | 1895   | backvmid3.dat         |
| media     | 43    | 1152574301  | 6886     | 645    | wornicons.dat         |
| media     | 44    | -884827257  | 2966     | 611    | sworddecor.dat        |
| media     | 45    | -1568083395 | 49592    | 4779   | invback.dat           |
| media     | 46    | 1922934081  | 1622     | 707    | leftarrow.dat         |
| media     | 47    | 1727594325  | 19942    | 254    | magicoff2.dat         |
| media     | 48    | -1204854137 | 11027    | 3000   | mapfunction.dat       |
| media     | 49    | 305236077   | 11656    | 1753   | prayeroff.dat         |
| media     | 50    | -716997548  | 434      | 147    | steelborder2.dat      |
| media     | 51    | 1442199444  | 1622     | 718    | rightarrow.dat        |
| media     | 52    | -1593819477 | 9962     | 2000   | backright1.dat        |
| media     | 53    | -1579973636 | 9659     | 2725   | backright2.dat        |
| models    | 0     | 382250581   | 19656    | 6806   | base_label.dat        |
| models    | 1     | -268804774  | 312699   | 73872  | ob_point1.dat         |
| models    | 2     | -254958933  | 242348   | 148377 | ob_point2.dat         |
| models    | 3     | -241113092  | 226500   | 121339 | ob_point3.dat         |
| models    | 4     | -227267251  | 245902   | 151941 | ob_point4.dat         |
| models    | 5     | -213421410  | 104707   | 15156  | ob_point5.dat         |
| models    | 6     | 1996729425  | 41450    | 16795  | ob_head.dat           |
| models    | 7     | 659053171   | 682      | 660    | base_head.dat         |
| models    | 8     | 690528443   | 36959    | 12188  | frame_head.dat        |
| models    | 9     | 1186107867  | 269890   | 24471  | frame_tran1.dat       |
| models    | 10    | 1199953708  | 150003   | 75903  | frame_tran2.dat       |
| models    | 11    | 1350899006  | 587269   | 186335 | ob_vertex1.dat        |
| models    | 12    | 1364744847  | 456725   | 63594  | ob_vertex2.dat        |
| models    | 13    | -1313359330 | 7391     | 2862   | frame_del.dat         |
| models    | 14    | -1121516105 | 4318     | 634    | base_type.dat         |
| models    | 15    | -113454781  | 913450   | 52802  | ob_face1.dat          |
| models    | 16    | -99608940   | 155603   | 13410  | ob_face2.dat          |
| models    | 17    | -85763099   | 264208   | 14204  | ob_face3.dat          |
| models    | 18    | -71917258   | 43619    | 1577   | ob_face4.dat          |
| models    | 19    | -58071417   | 86804    | 4969   | ob_face5.dat          |
| models    | 20    | -371539808  | 56886    | 17301  | ob_axis.dat           |
| textures  | 0     | 224847211   | 16386    | 2913   | 0.dat                 |
| textures  | 1     | 238693052   | 16386    | 3797   | 1.dat                 |
| textures  | 2     | 252538893   | 16386    | 1845   | 2.dat                 |
| textures  | 3     | 266384734   | 16386    | 1822   | 3.dat                 |
| textures  | 4     | 280230575   | 16386    | 5207   | 4.dat                 |
| textures  | 5     | 294076416   | 16386    | 3712   | 5.dat                 |
| textures  | 6     | 307922257   | 16386    | 3178   | 6.dat                 |
| textures  | 7     | 321768098   | 4098     | 181    | 7.dat                 |
| textures  | 8     | 335613939   | 15490    | 4123   | 8.dat                 |
| textures  | 9     | 349459780   | 16386    | 4361   | 9.dat                 |
| textures  | 10    | -1929337337 | 2427     | 2280   | index.dat             |
| textures  | 11    | 1698082440  | 16386    | 4826   | 10.dat                |
| textures  | 12    | 1711928281  | 16386    | 5359   | 11.dat                |
| textures  | 13    | 1725774122  | 16386    | 1644   | 12.dat                |
| textures  | 14    | 1739619963  | 4098     | 952    | 13.dat                |
| textures  | 15    | 1753465804  | 4098     | 913    | 14.dat                |
| textures  | 16    | 1767311645  | 16386    | 3405   | 15.dat                |
| textures  | 17    | 1781157486  | 16386    | 4281   | 16.dat                |
| textures  | 18    | 1795003327  | 15130    | 892    | 17.dat                |
| textures  | 19    | 1808849168  | 16386    | 5046   | 18.dat                |
| textures  | 20    | 1822695009  | 16386    | 3462   | 19.dat                |
| textures  | 21    | -1752288555 | 16386    | 5018   | 20.dat                |
| textures  | 22    | -1738442714 | 14978    | 3213   | 21.dat                |
| textures  | 23    | -1724596873 | 16386    | 2966   | 22.dat                |
| textures  | 24    | -1710751032 | 16386    | 3864   | 23.dat                |
| textures  | 25    | -1696905191 | 16386    | 3794   | 24.dat                |
| textures  | 26    | -1683059350 | 16386    | 5286   | 25.dat                |
| textures  | 27    | -1669213509 | 16386    | 1822   | 26.dat                |
| textures  | 28    | -1655367668 | 16386    | 4137   | 27.dat                |
| textures  | 29    | -1641521827 | 15374    | 2519   | 28.dat                |
| textures  | 30    | -1627675986 | 15490    | 2635   | 29.dat                |
| textures  | 31    | -907692254  | 16386    | 3368   | 30.dat                |
| textures  | 32    | -893846413  | 16386    | 4937   | 31.dat                |
| textures  | 33    | -880000572  | 16386    | 5286   | 32.dat                |
| textures  | 34    | -866154731  | 16002    | 3233   | 33.dat                |
| textures  | 35    | -852308890  | 15131    | 3794   | 34.dat                |
| textures  | 36    | -838463049  | 16386    | 47     | 35.dat                |
| textures  | 37    | -824617208  | 16386    | 3023   | 36.dat                |
| textures  | 38    | -810771367  | 4098     | 841    | 37.dat                |
| textures  | 39    | -796925526  | 4098     | 235    | 38.dat                |
| textures  | 40    | -783079685  | 16386    | 2459   | 39.dat                |
| textures  | 41    | -63095953   | 3656     | 747    | 40.dat                |
| textures  | 42    | -49250112   | 15627    | 1967   | 41.dat                |
| textures  | 43    | -35404271   | 14618    | 2000   | 42.dat                |
| textures  | 44    | -21558430   | 15008    | 2721   | 43.dat                |
| textures  | 45    | -7712589    | 16386    | 2824   | 44.dat                |
| textures  | 46    | 6133252     | 16386    | 3585   | 45.dat                |
| textures  | 47    | 19979093    | 16386    | 4394   | 46.dat                |
| textures  | 48    | 33824934    | 16386    | 5125   | 47.dat                |
| textures  | 49    | 47670775    | 16386    | 5288   | 48.dat                |
| textures  | 50    | 61516616    | 16386    | 4729   | 49.dat                |
| wordenc   | 0     | 1648736955  | 3255     | 1914   | badenc.txt            |
| wordenc   | 1     | -573349193  | 8626     | 8424   | fragmentsenc.txt      |
| wordenc   | 2     | -840867198  | 1058     | 532    | tldlist.txt           |
| wordenc   | 3     | 1694783164  | 4309     | 2381   | domainenc.txt         |
| sounds    | 0     | 232787039   | 100482   | 22817  | sounds.dat            |

---

## Config

### Config Packing

This section describes how game configuration data is packed into binary format for the RuneScape rev-225 server. The
config packing pipeline reads human-readable text source files and produces paired `.dat`/`.idx` binary files that are
bundled into the config Jag archive.

#### Config Jag Archive

The config Jag archive contains all game configuration data. Its original CRC is **511217062**.

##### Jag Archive Structure

A Jag archive is a container format with per-entry bzip2 compression (not whole-archive compression). The archive header
consists of:

```
p3(unpacked_size)
p3(packed_size)
```

If `packed_size != unpacked_size`, the remaining data after the header is bzip2-compressed and must be decompressed
before reading the file table. After decompression (or directly, if uncompressed), the file table is:

```
p2(file_count)
[for each file]
    p4(name_hash)     -- hash of the filename (e.g. "loc.dat")
    p3(unpacked_size)  -- decompressed size of this file
    p3(packed_size)    -- compressed size of this file
[end]
[file data blocks, contiguous]
```

File names are not stored directly; instead, a hash is computed. The hash algorithm converts the name to uppercase and
iterates each character: `hash = hash * 61 + (char_code - 32)`.

When the archive uses per-entry compression (as the config Jag does), each individual file's data block is
bzip2-compressed independently. To read a file, the entry's data slice is decompressed using its `unpacked_size`. When
the archive uses whole-archive compression, the data is stored uncompressed within the decompressed archive body.

##### Files Inside the Config Jag

Each config type produces paired `.dat` and `.idx` files. Client-visible types go into the config Jag archive;
server-only types are loaded directly from packed data.

##### Client + Server types (in config Jag)

| Type     | Files                          | Source Extension |
|----------|--------------------------------|------------------|
| seq      | `seq.dat`, `seq.idx`           | `.seq`           |
| loc      | `loc.dat`, `loc.idx`           | `.loc`           |
| flo      | `flo.dat`, `flo.idx`           | `.flo`           |
| spotanim | `spotanim.dat`, `spotanim.idx` | `.spotanim`      |
| obj      | `obj.dat`, `obj.idx`           | `.obj`           |
| npc      | `npc.dat`, `npc.idx`           | `.npc`           |
| idk      | `idk.dat`, `idk.idx`           | `.idk`           |
| varp     | `varp.dat`, `varp.idx`         | `.varp`          |

##### Server-only types (not in Jag)

| Type     | Source Extension | Description                                |
|----------|------------------|--------------------------------------------|
| inv      | `.inv`           | Inventory definitions (scope, size, stock) |
| param    | `.param`         | Parameter type and default value           |
| enum     | `.enum`          | Key-value lookups with input/output types  |
| struct   | `.struct`        | Parameter containers                       |
| hunt     | `.hunt`          | NPC hunt behavior rules                    |
| dbrow    | `.dbrow`         | Database row entries                       |
| dbtable  | `.dbtable`       | Database table schemas                     |
| varn     | `.varn`          | NPC variable definitions                   |
| vars     | `.vars`          | Shared variable definitions                |
| mesanim  | `.mesanim`       | Message animation sequences                |
| category | `.category`      | Virtual type (debugname only)              |

#### Binary Entry Format

Every config type uses the same binary encoding for its `.dat` and `.idx` pair.

##### dat file

```
p2(count)
[entry_0][entry_1]...[entry_{count-1}]
```

Each entry is a sequence of opcode-data pairs terminated by opcode 0:

```
[p1(opcode) data...]* p1(0)
```

Opcodes are unsigned bytes. Opcode 0 always marks the end of an entry. Opcode 250 is conventionally used to store the
debugname (as a JSTR string) for server-side data.

##### idx file

```
p2(count)
[p2(length_0)][p2(length_1)]...[p2(length_{count-1})]
```

Each `p2(length)` records the byte length of the corresponding entry in the dat file (including the terminating opcode
0).

##### Data Primitives

| Primitive  | Description                                                 |
|------------|-------------------------------------------------------------|
| `p1(v)`    | Write 1 byte (unsigned)                                     |
| `p2(v)`    | Write 2 bytes big-endian (unsigned short)                   |
| `p3(v)`    | Write 3 bytes big-endian                                    |
| `p4(v)`    | Write 4 bytes big-endian (signed int)                       |
| `pbool(v)` | Write 1 byte: 0 = false, 1 = true                           |
| `pjstr(v)` | Write string bytes followed by byte 10 (newline terminator) |

#### Source Text Format

Config source files use a simple section-based text format:

```
[section_name]
key=value
key2=value2

[another_section]
key=value
```

Lines beginning with `//` are comments. Empty lines are ignored.

##### Constants

Constants are defined in `.constant` files with the format `^CONSTANT_NAME=value` (or `CONSTANT_NAME=value`). In config
files, constants are referenced using `^CONSTANT_NAME` syntax, which is substituted at parse time before opcode
encoding.

#### Pack Registry

The pack registry maps human-readable debugnames to numeric IDs. Registry files are stored as `.pack` files in
`content/pack/` with the format:

```
0=debugname_zero
1=debugname_one
2=debugname_two
```

Each line is `ID=debugname`. The registry is loaded into a bidirectional map (`PackFile`) that supports both
`get_by_id(u16) -> &str` and `get_by_debugname(&str) -> u16` lookups. The `max` field is set to one past the highest ID
found, and all IDs from 0 to max-1 must have corresponding config entries.

The `PackRegistry` aggregates all pack files for cross-type resolution (e.g., an obj config referencing a model by
debugname).

#### Client vs. Server Data

Config types that have client-visible data produce **both** a server `PackedData` and a client `PackedData`. The client
data is written into the config Jag for the game client, while the server data is used by the game server.

**Client + Server types:** seq, loc, npc, obj, spotanim, idk, varp, flo

**Server-only types:** param, inv, enum, struct, varn, vars, hunt, mesanim, dbtable, dbrow

Server-only types set `client: None` in their `PackedFile` output.

#### Config Type Packing Reference

##### SEQ (Animation Sequences)

Source extension: `.seq`

| Opcode | Field                     | Format                                                                                                                             |
|--------|---------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| 1      | frames + iframes + delays | `p1(count)` then per frame: `p2(frame_id) p2(iframe_id) p2(delay)`. iframe defaults to 0xFFFF (-1) if absent; delay defaults to 0. |
| 2      | loops                     | `p2(value)`                                                                                                                        |
| 3      | walkmerge                 | `p1(label_count)` then `p1(label)` per label. Labels are parsed from `label_N` format.                                             |
| 4      | stretches                 | Flag opcode (no data). Emitted when `stretches=yes`.                                                                               |
| 5      | priority                  | `p1(value)`                                                                                                                        |
| 6      | replaceheldleft           | `p2(value)`. `hide` = 0, otherwise `obj_id + 512`.                                                                                 |
| 7      | replaceheldright          | `p2(value)`. Same encoding as opcode 6.                                                                                            |
| 8      | maxloops                  | `p1(value)`                                                                                                                        |
| 250    | debugname                 | `pjstr(debugname)` (server only)                                                                                                   |

Source keys: `frame{N}` (frame references by anim debugname), `delay{N}` (frame delays), `iframe{N}` (interpolation
frames).

Client CRC: 1638136604

##### LOC (Locations / Scenery)

Source extension: `.loc`

Loc configs are special: one `[section_name]` can fan out to multiple loc IDs through the pack registry. The compiler
emits opcodes to every matching ID.

**Wall Shape Constants**

Locs use shape suffixes to find model variants:

| Constant                    | Shape ID | Suffix |
|-----------------------------|----------|--------|
| WALL_STRAIGHT               | 0        | `_1`   |
| WALL_DIAGONAL_CORNER        | 1        | `_2`   |
| WALL_L                      | 2        | `_3`   |
| WALL_SQUARE_CORNER          | 3        | `_4`   |
| WALLDECOR_STRAIGHT_NOOFFSET | 4        | `_q`   |
| WALLDECOR_STRAIGHT_OFFSET   | 5        | `_w`   |
| WALLDECOR_DIAGONAL_OFFSET   | 6        | `_r`   |
| WALLDECOR_DIAGONAL_NOOFFSET | 7        | `_e`   |
| WALLDECOR_DIAGONAL_BOTH     | 8        | `_t`   |
| WALL_DIAGONAL               | 9        | `_5`   |
| CENTREPIECE_STRAIGHT        | 10       | `_8`   |
| CENTREPIECE_DIAGONAL        | 11       | `_9`   |
| ROOF_STRAIGHT               | 12       | `_a`   |
| ROOF_DIAGONAL_WITH_ROOFEDGE | 13       | `_s`   |
| ROOF_DIAGONAL               | 14       | `_d`   |
| ROOF_L_CONCAVE              | 15       | `_f`   |
| ROOF_L_CONVEX               | 16       | `_g`   |
| ROOF_FLAT                   | 17       | `_h`   |
| ROOFEDGE_STRAIGHT           | 18       | `_z`   |
| ROOFEDGE_DIAGONAL_CORNER    | 19       | `_x`   |
| ROOFEDGE_L                  | 20       | `_c`   |
| ROOFEDGE_SQUARE_CORNER      | 21       | `_v`   |
| GROUND_DECOR                | 22       | `_0`   |

The model resolution algorithm checks if the model name directly exists without any shape suffixes (other than `_8`). If
so, it is treated as a centrepiece. Otherwise, it iterates all shapes, appending the suffix to find model variants.
Centrepiece (`_8`) is checked first.

**Opcodes**

| Opcode | Field         | Format                                                                                              |
|--------|---------------|-----------------------------------------------------------------------------------------------------|
| 1      | models        | `p1(count)` then per model: `p2(model_id) p1(shape)`                                                |
| 2      | name          | `pjstr(name)`                                                                                       |
| 3      | desc          | `pjstr(desc)`                                                                                       |
| 14     | width         | `p1(value)`                                                                                         |
| 15     | length        | `p1(value)`                                                                                         |
| 17     | blockwalk     | Flag opcode. Emitted when `blockwalk=no`.                                                           |
| 18     | blockrange    | Flag opcode. Emitted when `blockrange=no`.                                                          |
| 19     | active        | `p1(value)` (0 or 1)                                                                                |
| 21     | hillskew      | Flag opcode. Emitted when `hillskew=yes`.                                                           |
| 22     | sharelight    | Flag opcode. Emitted when `sharelight=yes`.                                                         |
| 23     | occlude       | Flag opcode. Emitted when `occlude=yes`.                                                            |
| 24     | anim          | `p2(seq_id)`                                                                                        |
| 25     | hasalpha      | Flag opcode. Emitted when `hasalpha=yes`.                                                           |
| 28     | wallwidth     | `p1(value)`                                                                                         |
| 29     | ambient       | `p1(value)` (signed byte)                                                                           |
| 30-34  | op1-op5       | `pjstr(text)` (interaction options)                                                                 |
| 39     | contrast      | `p1(value)` (signed byte)                                                                           |
| 40     | recol/retex   | `p1(count)` then per pair: `p2(src_hsl) p2(dst_hsl)`. Retextures are stored in recol until rev 465. |
| 60     | mapfunction   | `p2(value)`                                                                                         |
| 61     | category      | `p2(value)` (server only)                                                                           |
| 62     | mirror        | Flag opcode. Emitted when `mirror=yes`.                                                             |
| 64     | shadow        | Flag opcode. Emitted when `shadow=no`.                                                              |
| 65     | resizex       | `p2(value)`                                                                                         |
| 66     | resizey       | `p2(value)`                                                                                         |
| 67     | resizez       | `p2(value)`                                                                                         |
| 68     | mapscene      | `p2(value)`                                                                                         |
| 69     | forceapproach | `p1(flags)`: north=0b1110, east=0b1101, south=0b1011, west=0b0111                                   |
| 70     | offsetx       | `p2(value)`                                                                                         |
| 71     | offsety       | `p2(value)`                                                                                         |
| 72     | offsetz       | `p2(value)`                                                                                         |
| 73     | forcedecor    | Flag opcode. Emitted when `forcedecor=yes`.                                                         |
| 249    | params        | `p1(count)` then per param: `p3(param_id) pbool(is_string) [p4(int_value)                           | pjstr(str_value)]` |
| 250    | debugname     | `pjstr(debugname)` (server only)                                                                    |

Client CRC: 891497087

##### OBJ (Items / Objects)

Source extension: `.obj`

Certificate objects (debugnames starting with `cert_`) receive special handling: they are auto-generated with `certlink`
pointing to the base item and `certtemplate` pointing to `template_for_cert`. If a non-certificate obj has a
corresponding `cert_` entry, the server data also writes a reverse `certlink` (opcode 97) for fast lookup.

When an obj has a `model` but no explicit `name`, the debugname is auto-capitalized (first letter uppercase, underscores
replaced with spaces) and used as the name.

**Wear Positions**

| Name      | ID |
|-----------|----|
| hat       | 0  |
| back      | 1  |
| front     | 2  |
| righthand | 3  |
| torso     | 4  |
| lefthand  | 5  |
| arms      | 6  |
| legs      | 7  |
| head      | 8  |
| hands     | 9  |
| feet      | 10 |
| jaw       | 11 |
| ring      | 12 |
| quiver    | 13 |

**Opcodes**

| Opcode  | Field          | Format                                                                                          |
|---------|----------------|-------------------------------------------------------------------------------------------------|
| 1       | model          | `p2(model_id)`                                                                                  |
| 2       | name           | `pjstr(name)`                                                                                   |
| 3       | desc           | `pjstr(desc)`                                                                                   |
| 4       | 2dzoom         | `p2(value)`                                                                                     |
| 5       | 2dxan          | `p2(value)`                                                                                     |
| 6       | 2dyan          | `p2(value)`                                                                                     |
| 7       | 2dxof          | `p2(value)` (signed)                                                                            |
| 8       | 2dyof          | `p2(value)` (signed)                                                                            |
| 9       | code9          | Flag opcode.                                                                                    |
| 10      | code10         | `p2(seq_id)`                                                                                    |
| 11      | stackable      | Flag opcode.                                                                                    |
| 12      | cost           | `p4(value)`                                                                                     |
| 13      | wearpos        | `p1(wear_pos_id)` (server only)                                                                 |
| 14      | wearpos2       | `p1(wear_pos_id)` (server only)                                                                 |
| 15      | tradeable      | Flag opcode. Emitted when `tradeable=no` (server only).                                         |
| 16      | members        | Flag opcode. Emitted when `members=yes`.                                                        |
| 23      | manwear        | `p2(model_id) p1(offset)`                                                                       |
| 24      | manwear2       | `p2(model_id)`                                                                                  |
| 25      | womanwear      | `p2(model_id) p1(offset)`                                                                       |
| 26      | womanwear2     | `p2(model_id)`                                                                                  |
| 27      | wearpos3       | `p1(wear_pos_id)` (server only)                                                                 |
| 30-34   | op1-op5        | `pjstr(text)` (ground interaction options)                                                      |
| 35-39   | iop1-iop5      | `pjstr(text)` (inventory interaction options)                                                   |
| 40      | recol          | `p1(count)` then per pair: `p2(src) p2(dst)`. Values >= 100 are converted via `rgb15_to_hsl16`. |
| 75      | weight         | `p2(grams)` (server only). Supports `kg`, `g`, `oz`, `lb` suffixes.                             |
| 78      | manwear3       | `p2(model_id)`                                                                                  |
| 79      | womanwear3     | `p2(model_id)`                                                                                  |
| 90      | manhead        | `p2(model_id)`                                                                                  |
| 91      | womanhead      | `p2(model_id)`                                                                                  |
| 92      | manhead2       | `p2(model_id)`                                                                                  |
| 93      | womanhead2     | `p2(model_id)`                                                                                  |
| 94      | category       | `p2(value)` (server only)                                                                       |
| 95      | 2dzan          | `p2(value)`                                                                                     |
| 96      | dummyitem      | `p1(value)`: `graphic_only` = 1, `inv_only` = 2 (server only)                                   |
| 97      | certlink       | `p2(obj_id)`                                                                                    |
| 98      | certtemplate   | `p2(obj_id)`                                                                                    |
| 100-109 | count1-count10 | `p2(obj_id) p2(stack_count)`                                                                    |
| 201     | respawnrate    | `p2(value)` (server only)                                                                       |
| 249     | params         | Same format as loc params.                                                                      |
| 250     | debugname      | `pjstr(debugname)` (server only)                                                                |

Client CRC: -840233510

##### NPC (Non-Player Characters)

Source extension: `.npc`

NPCs always emit a name (opcode 2). If no `name=` key is present, the debugname is used. NPCs also default to
`vislevel=1` if no explicit `vislevel` is set.

**Opcodes**

| Opcode | Field             | Format                                                                                                        |
|--------|-------------------|---------------------------------------------------------------------------------------------------------------|
| 1      | models            | `p1(count)` then `p2(model_id)` per model                                                                     |
| 2      | name              | `pjstr(name)`                                                                                                 |
| 3      | desc              | `pjstr(desc)`                                                                                                 |
| 12     | size              | `p1(value)`                                                                                                   |
| 13     | readyanim         | `p2(seq_id)`                                                                                                  |
| 14     | walkanim (single) | `p2(seq_id)`                                                                                                  |
| 16     | hasalpha          | Flag opcode.                                                                                                  |
| 17     | walkanim (4-way)  | `p2(walk) p2(walk_back) p2(walk_left) p2(walk_right)`                                                         |
| 18     | category          | `p2(value)` (server only)                                                                                     |
| 30-34  | op1-op5           | `pjstr(text)` (interaction options)                                                                           |
| 40     | recol             | `p1(count)` then per pair: `p2(src) p2(dst)`. Values >= 100 use `rgb15_to_hsl16`.                             |
| 60     | head models       | `p1(count)` then `p2(model_id)` per head model                                                                |
| 74-79  | combat stats      | `p2(value)`: 74=attack, 75=defence, 76=strength, 77=hitpoints, 78=ranged, 79=magic (server only)              |
| 90     | resizex           | `p2(value)`                                                                                                   |
| 91     | resizey           | `p2(value)`                                                                                                   |
| 92     | resizez           | `p2(value)`                                                                                                   |
| 93     | minimap           | Flag opcode. Emitted when `minimap=no`.                                                                       |
| 95     | vislevel          | `p2(value)`. `hide` = 0. Defaults to 1 if omitted.                                                            |
| 97     | resizeh           | `p2(value)`                                                                                                   |
| 98     | resizev           | `p2(value)`                                                                                                   |
| 200    | wanderrange       | `p2(value)` (server only)                                                                                     |
| 201    | maxrange          | `p2(value)` (server only)                                                                                     |
| 202    | huntrange         | `p1(value)` (server only)                                                                                     |
| 203    | timer             | `p2(value)` (server only)                                                                                     |
| 204    | respawnrate       | `p2(value)` (server only)                                                                                     |
| 206    | moverestrict      | `p1(value)` (server only): normal=0, blocked=1, blocked+normal=2, indoors=3, outdoors=4, nomove=5, passthru=6 |
| 207    | attackrange       | `p2(value)` (server only)                                                                                     |
| 208    | blockwalk         | `p1(value)` (server only): none=0, all=1, npc=2                                                               |
| 209    | huntmode          | `p1(hunt_id)` (server only)                                                                                   |
| 210    | defaultmode       | `p1(value)` (server only): none=0, wander=1, patrol=2                                                         |
| 211    | members           | Flag opcode (server only).                                                                                    |
| 212    | patrol            | `p1(count)` then per waypoint: `p4(coord) p1(delay)` (server only)                                            |
| 213    | givechase         | Flag opcode. Emitted when `givechase=no` (server only).                                                       |
| 214    | regenrate         | `p2(value)` (server only)                                                                                     |
| 249    | params            | Same format as loc params.                                                                                    |
| 250    | debugname         | `pjstr(debugname)` (server only)                                                                              |

Client CRC: -2140681882

##### FLO (Floor Overlays)

Source extension: `.flo`

If the debugname does not start with `flo_`, opcode 6 is emitted with the debugname as a transmitted name.

| Opcode | Field   | Format                                                          |
|--------|---------|-----------------------------------------------------------------|
| 1      | colour  | `p3(rgb_hex)`                                                   |
| 2      | texture | `p1(texture_id)`                                                |
| 3      | overlay | Flag opcode. Emitted when `overlay=yes`.                        |
| 5      | occlude | Flag opcode. Emitted when `occlude=no`.                         |
| 6      | name    | `pjstr(debugname)` (auto-emitted for non-`flo_` prefixed names) |

Client CRC: **1976597026**

##### IDK (Identity Kit / Player Appearance)

Source extension: `.idk`

**Body Part Types**

| Name        | ID |
|-------------|----|
| man_hair    | 0  |
| man_jaw     | 1  |
| man_torso   | 2  |
| man_arms    | 3  |
| man_hands   | 4  |
| man_legs    | 5  |
| man_feet    | 6  |
| woman_hair  | 7  |
| woman_jaw   | 8  |
| woman_torso | 9  |
| woman_arms  | 10 |
| woman_hands | 11 |
| woman_legs  | 12 |
| woman_feet  | 13 |

**Opcodes**

| Opcode | Field        | Format                                                                                  |
|--------|--------------|-----------------------------------------------------------------------------------------|
| 1      | type         | `p1(body_part_id)`                                                                      |
| 2      | models       | `p1(count)` then `p2(model_id)` per model                                               |
| 3      | disable      | Flag opcode. Emitted when `disable=yes`.                                                |
| 40-49  | recol source | `p2(hsl16)` per index (index = opcode - 39). Values are converted via `rgb15_to_hsl16`. |
| 50-59  | recol dest   | `p2(hsl16)` per index (index = opcode - 49). Values are converted via `rgb15_to_hsl16`. |
| 60-69  | head models  | `p2(model_id)` per index (index = opcode - 59)                                          |
| 250    | debugname    | `pjstr(debugname)` (server only)                                                        |

Client CRC: -359342366

##### VARP (Player Variables)

Source extension: `.varp`

| Opcode | Field      | Format                                                  |
|--------|------------|---------------------------------------------------------|
| 1      | scope      | `p1(value)`: temp=0 (not emitted), perm=1               |
| 2      | type       | `p1(type_char)` (see Script Variable Types below)       |
| 4      | protect    | Flag opcode. Emitted when `protect=no`.                 |
| 5      | clientcode | `p2(value)`                                             |
| 6      | transmit   | Flag opcode. Emitted when `transmit=yes` (server only). |
| 250    | debugname  | `pjstr(debugname)` (server only)                        |

Client CRC: 705633567

##### SPOTANIM (Spot Animations)

Source extension: `.spotanim`

| Opcode | Field        | Format                                                 |
|--------|--------------|--------------------------------------------------------|
| 1      | model        | `p2(model_id)`                                         |
| 2      | anim         | `p2(seq_id)`                                           |
| 3      | hasalpha     | Flag opcode. Emitted when `hasalpha=yes`.              |
| 4      | resizeh      | `p2(value)`                                            |
| 5      | resizev      | `p2(value)`                                            |
| 6      | angle        | `p2(value)`                                            |
| 7      | ambient      | `p1(value)`                                            |
| 8      | contrast     | `p1(value)`                                            |
| 40-49  | recol source | `p2(hsl16)` per index. Converted via `rgb15_to_hsl16`. |
| 50-59  | recol dest   | `p2(hsl16)` per index. Converted via `rgb15_to_hsl16`. |
| 250    | debugname    | `pjstr(debugname)` (server only)                       |

Client CRC: -1279835623

##### PARAM (Server-Only)

Source extension: `.param`

Parameters define typed key-value metadata that can be attached to other config types via opcode 249.

| Opcode | Field            | Format                                                                    |
|--------|------------------|---------------------------------------------------------------------------|
| 1      | type             | `p1(type_char)`                                                           |
| 2      | default (int)    | `p4(value)`                                                               |
| 4      | autodisable      | Flag opcode. Emitted when `autodisable=no`. Autodisable defaults to true. |
| 5      | default (string) | `pjstr(value)`                                                            |
| 250    | debugname        | `pjstr(debugname)`                                                        |

##### INV (Server-Only)

Source extension: `.inv`

| Opcode | Field     | Format                                                              |
|--------|-----------|---------------------------------------------------------------------|
| 1      | scope     | `p1(value)`: temp=0, perm=1, shared=2                               |
| 2      | size      | `p2(value)`                                                         |
| 3      | stackall  | Flag opcode. Emitted when `stackall=yes`.                           |
| 4      | stock     | `p1(count)` then per stock: `p2(obj_id) p2(count) p4(restock_rate)` |
| 5      | restock   | Flag opcode. Emitted when `restock=yes`.                            |
| 6      | allstock  | Flag opcode. Emitted when `allstock=yes`.                           |
| 7      | protect   | Flag opcode. Emitted when `protect=no`.                             |
| 8      | runweight | Flag opcode. Emitted when `runweight=yes`.                          |
| 9      | dummyinv  | Flag opcode. Emitted when `dummyinv=yes`.                           |
| 250    | debugname | `pjstr(debugname)`                                                  |

##### ENUM (Server-Only)

Source extension: `.enum`

Enums map keys to values. Both the input type and output type can be `autoint`, which means keys are auto-assigned from
0, 1, 2, ... in order.

| Opcode | Field            | Format                                                     |
|--------|------------------|------------------------------------------------------------|
| 1      | inputtype        | `p1(type_char)`. If `autoint`, encodes as `int` type char. |
| 2      | outputtype       | `p1(type_char)`                                            |
| 3      | default (string) | `pjstr(value)`                                             |
| 4      | default (int)    | `p4(value)`                                                |
| 5      | string values    | `p2(count)` then per entry: `p4(key) pjstr(value)`         |
| 6      | int values       | `p2(count)` then per entry: `p4(key) p4(value)`            |
| 250    | debugname        | `pjstr(debugname)`                                         |

##### STRUCT (Server-Only)

Source extension: `.struct`

Structs are simple containers of param key-value pairs.

| Opcode | Field     | Format                                                                                          |
|--------|-----------|-------------------------------------------------------------------------------------------------|
| 249    | params    | `p1(count)` then per param: `p3(param_id) pbool(is_string) [p4(int_value) \| pjstr(str_value)]` |
| 250    | debugname | `pjstr(debugname)`                                                                              |

##### HUNT (Server-Only)

Source extension: `.hunt`

Hunt configs define NPC hunting behavior.

| Opcode | Field                | Format                                                     |
|--------|----------------------|------------------------------------------------------------|
| 1      | type                 | `p1(value)`: off=0, player=1, npc=2, obj=3, scenery=4      |
| 2      | check_vis            | `p1(value)`: off=0, lineofsight=1, lineofwalk=2            |
| 3      | check_nottoostrong   | `p1(value)`: off=0, outside_wilderness=1                   |
| 4      | check_notbusy        | Flag opcode. Emitted when `on`.                            |
| 5      | find_keephunting     | Flag opcode. Emitted when `on`.                            |
| 6      | find_newmode         | `p1(npc_mode)`                                             |
| 7      | nobodynear           | `p1(value)`: keephunting=0, pausehunt=1                    |
| 8      | check_notcombat      | `p2(varp_id)`                                              |
| 9      | check_notcombat_self | `p2(varn_id)`                                              |
| 10     | check_afk            | Flag opcode.                                               |
| 11     | rate                 | `p2(value)`. Not emitted if value is 1 (default).          |
| 12     | check_category       | `p2(category_id)`                                          |
| 13     | check_npc            | `p2(npc_id)`                                               |
| 14     | check_obj            | `p2(obj_id)`                                               |
| 15     | check_loc            | `p2(loc_id)`                                               |
| 16     | check_inv            | `p2(inv_id) p2(obj_id) pjstr(condition) p4(value)`         |
| 17     | check_invparam       | `p2(inv_id) p2(param_id) pjstr(condition) p4(value)`       |
| 18-20  | extracheck_var       | `p2(varp_id) pjstr(condition) p4(value)`. Up to 3 entries. |
| 250    | debugname            | `pjstr(debugname)`                                         |

Check requirements are mutually exclusive: only one of `check_category`, `check_npc`, `check_obj`, `check_loc`,
`check_inv`, or `check_invparam` may be specified per hunt config.

##### DBTABLE (Server-Only)

Source extension: `.dbtable`

Database table configs define column schemas.

Column definition format in source: `column=name,type[,type]...[,INDEXED][,REQUIRED][,LIST][,CLIENTSIDE]`

Column property flags:

| Flag       | Bit | Value                                   |
|------------|-----|-----------------------------------------|
| INDEXED    | 0x1 | Column is indexed for lookups           |
| REQUIRED   | 0x2 | Column must have data in every row      |
| LIST       | 0x4 | Column can have multiple values per row |
| CLIENTSIDE | 0x8 | Column is sent to the client            |

| Opcode | Field              | Format                                                                                                                                                                                                            |
|--------|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1      | column definitions | `p1(column_count)` then per column: `p1(flags)` where flags = `col_index \| 0x80` if has defaults, `p1(type_count)` then `p1(type_char)` per type. If has defaults: `p1(1)` then values. Terminated by `p1(255)`. |
| 250    | debugname          | `pjstr(debugname)`                                                                                                                                                                                                |
| 251    | column names       | `p1(count)` then `pjstr(name)` per column                                                                                                                                                                         |
| 252    | column properties  | `p1(count)` then `p1(props)` per column                                                                                                                                                                           |

##### DBROW (Server-Only)

Source extension: `.dbrow`

Database row configs reference a table and contain data for its columns.

Source format:

```
[row_name]
table=table_name
data=column_name,value[,value]...
```

| Opcode | Field           | Format                                                                                                                                                    |
|--------|-----------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
| 3      | column data     | `p1(column_count)` then per column: `p1(col_index) p1(type_count) [p1(type_char)]... p1(field_count)` then values per field set. Terminated by `p1(255)`. |
| 4      | table reference | `p2(table_id)`                                                                                                                                            |
| 250    | debugname       | `pjstr(debugname)`                                                                                                                                        |

Columns marked REQUIRED in the table schema must have at least one data entry. Columns not marked LIST cannot have
multiple data entries.

#### Colour Handling

Recolouring uses a conversion from 15-bit RGB to 16-bit HSL format (`rgb15_to_hsl16`).

The 15-bit RGB value encodes 5 bits per channel:

- Red: bits 14-10
- Green: bits 9-5
- Blue: bits 4-0

These are normalized to 0.0-1.0 and converted to HSL. The HSL is then quantized to a 16-bit packed format via
`hsl24to16`:

```
hsl16 = ((hue >> 2) << 10) | ((saturation >> 5) << 7) | (lightness >> 1)
```

Saturation is reduced at high lightness values (above 179, 192, 217, or 243) to avoid oversaturation in bright colours.

For obj and npc recol values, raw values less than 100 are treated as pre-encoded HSL and passed through directly;
values >= 100 are treated as 15-bit RGB and converted via `rgb15_to_hsl16`. For loc recol, all values go through
`rgb15_to_hsl16`. For idk and spotanim, all recol values are converted via `rgb15_to_hsl16`.

#### Script Variable Types

Parameters, enums, and varps use a type character system to identify data types:

| Type Name             | Char   | Description                   |
|-----------------------|--------|-------------------------------|
| int                   | `i`    | Integer                       |
| autoint               | `\xFF` | Auto-incrementing integer     |
| string                | `s`    | String                        |
| coord                 | `c`    | Coordinate                    |
| obj / namedobj        | `o`    | Object reference              |
| npc                   | `n`    | NPC reference                 |
| loc                   | `l`    | Location reference            |
| component / interface | `I`    | Interface component reference |
| boolean               | `1`    | Boolean                       |
| enum                  | `g`    | Enum reference                |
| struct                | `J`    | Struct reference              |
| stat / npc_stat       | `S`    | Stat/skill reference          |
| seq                   | `A`    | Animation sequence reference  |
| synth                 | `P`    | Sound synth reference         |
| inv                   | `v`    | Inventory reference           |
| spotanim              | `t`    | Spot animation reference      |
| varp                  | `V`    | Player variable reference     |
| model                 | `m`    | Model reference               |
| category              | `y`    | Category reference            |
| idkit                 | `K`    | Identity kit reference        |
| player_uid            | `p`    | Player UID                    |
| npc_uid               | `N`    | NPC UID                       |
| dbrow                 | `\xD0` | Database row reference        |

#### CRC Verification

Client-side packed data can be verified against known CRC values to ensure binary-exact output:

| Config Type            | Expected Client CRC |
|------------------------|---------------------|
| seq                    | 1638136604          |
| loc                    | 891497087           |
| npc                    | -2140681882         |
| obj                    | -840233510          |
| flo                    | 1976597026          |
| idk                    | -359342366          |
| varp                   | 705633567           |
| spotanim               | -1279835623         |
| config (whole archive) | 511217062           |

#### Shared Config Enums

String-to-integer mappings in pack configs (e.g. `"normal" => 0` for moverestrict) are defined as shared enums in
`rs-pack/src/types.rs`. Both the packers and cache decoders reference these enums, eliminating magic numbers.

Key enums used in config packing:

| Enum                  | Config key                  | Config type |
|-----------------------|-----------------------------|-------------|
| MoveRestrict          | moverestrict                | npc         |
| BlockWalk             | blockwalk                   | npc         |
| NpcMode               | defaultmode, find_newmode   | npc, hunt   |
| InvScope              | scope                       | inv         |
| VarPlayerScope        | scope                       | varp        |
| WearPos               | wearpos, wearpos2, wearpos3 | obj         |
| DummyItem             | dummyitem                   | obj         |
| BodyType              | type                        | idk         |
| HuntModeType          | type                        | hunt        |
| HuntCheckVis          | check_vis                   | hunt        |
| HuntCheckNotTooStrong | check_nottoostrong          | hunt        |
| HuntNobodyNear        | nobodynear                  | hunt        |
| HuntCheckNotBusy      | check_notbusy               | hunt        |
| HuntCheckAfk          | check_afk                   | hunt        |
| ForceApproach         | forceapproach               | loc         |
| LocShape              | model shapes                | loc         |

#### Config Packing Source Code Reference

The config packing implementation is in the `rs-pack` crate:

- `rs-pack/src/pack/pack.rs` -- Main packing orchestration, text parsing, constant substitution
- `rs-pack/src/pack/packed_data.rs` -- Binary dat/idx builder
- `rs-pack/src/pack/pack_registry.rs` -- Pack file registry (debugname-to-id mapping)
- `rs-pack/src/pack/config/` -- Per-type packer modules (one file per config type)
- `rs-pack/src/pack/util/colour.rs` -- RGB-to-HSL colour conversion
- `rs-pack/src/types.rs` -- Shared enums for all config value mappings
- `rs-pack/src/cache/` -- Cache decoders that read the packed binary back into typed structs
- `rs-io/src/jag.rs` -- Jag archive reader/writer with per-entry bzip2 compression

### Config Unpacking

Extracts all game configuration data from the config Jag archive into human-readable text files and generates pack
registry files.

#### Input

The `config` Jag archive file (CRC: `511217062`), using per-entry bzip2 compression. Contains 8 paired `.dat`/`.idx`
entries:

| Type     | Files                          | Entries |
|----------|--------------------------------|---------|
| flo      | `flo.dat`, `flo.idx`           | 78      |
| seq      | `seq.dat`, `seq.idx`           | 1,027   |
| loc      | `loc.dat`, `loc.idx`           | 3,387   |
| spotanim | `spotanim.dat`, `spotanim.idx` | 244     |
| obj      | `obj.dat`, `obj.idx`           | 2,886   |
| npc      | `npc.dat`, `npc.idx`           | 1,017   |
| idk      | `idk.dat`, `idk.idx`           | 82      |
| varp     | `varp.dat`, `varp.idx`         | 295     |

#### Output

```
content_unpack/
├── all.flo              -- floor overlay configs
├── all.seq              -- animation sequence configs
├── all.loc              -- location/scenery configs
├── all.spotanim         -- spot animation configs
├── all.obj              -- item configs (excludes cert entries)
├── all.npc              -- NPC configs
├── all.idk              -- player appearance configs
├── all.varp             -- player variable configs
└── pack/
    ├── flo.pack         -- 0=cliff, 1=cliff2, ... (debug names from code 6)
    ├── seq.pack
    ├── loc.pack
    ├── spotanim.pack
    ├── obj.pack         -- includes cert_obj_N and template_for_cert entries
    ├── npc.pack
    ├── idk.pack
    ├── varp.pack
    ├── model.pack       -- context-aware model names
    ├── anim.pack        -- animation frame references
    ├── texture.pack     -- texture references
    └── category.pack    -- category references
```

#### Binary Entry Decoding

Each config type stores entries in a `.dat`/`.idx` pair:

- **idx**: `p2(count)`, then `p2(entry_length)` for each entry
- **dat**: `p2(count)`, then entry data concatenated sequentially

Each entry is a sequence of code-data pairs terminated by code 0:

```
[p1(code) data...]* p1(0)
```

The decoder reads each code using a `while buf.remaining() > 0` loop with `0 => break` as the first match arm. Any
unrecognized code value triggers a panic (e.g., `panic!("Unrecognized obj config code: {code}")`), ensuring all binary
data is fully accounted for.

#### Processing Order

Config types are decoded in a specific priority order that determines model naming:

1. **IDK** -- sets `_idk` / `_idk_head` model names (highest priority)
2. **OBJ** -- sets `_obj` / `_obj_wear` model names
3. **NPC** -- sets `_npc` / `_npc_head` model names
4. **SPOTANIM** -- sets `_spotanim` model names
5. **FLO** -- no model references
6. **SEQ** -- references anim.pack (frame IDs)
7. **LOC** -- sets `model_loc_{id}_{shape}` model names (lowest priority)
8. **VARP** -- no model references

The first config type to reference a model ID determines its name in `model.pack`.

#### Text File Format

Each text file contains sections for every entry in sequential ID order. Section headers use the entry name from the
corresponding `.pack` file:

```
[obj_0]
model=model_2141_obj
name=Cannon base
desc=A heavy metal cannon base.
2dzoom=1000
2dxan=123
op1=Pick-up

[obj_1]
model=model_2144_obj
name=Cannon stand
...
```

FLO entries use their debug name from code 6 as the section header instead of a generic ID:

```
[cliff]
overlay=yes
colour=0xAAAAAA

[water]
overlay=yes
texture=water
```

##### Cross-References

Values that reference other types use the generated names from their respective `.pack` files:

- `model=model_28_obj_wear` -> resolves via `model.pack`
- `readyanim=seq_808` -> resolves via `seq.pack`
- `certlink=obj_6` -> resolves via `obj.pack`
- `texture=door` -> resolves via `texture.pack`
- `walkanim=seq_819,seq_820,seq_821,seq_822` -> comma-separated seq refs

#### Cert Object Detection

OBJ entries containing only `certlink` + `certtemplate` codes (97 + 98) are identified as certificate objects:

1. The entry is named `cert_obj_{linked_id}` in `obj.pack` (e.g., `7=cert_obj_6`)
2. The entry is **excluded** from `all.obj` -- the packer auto-generates cert entries from the `cert_` prefix
3. The cert template entry (the obj referenced by all certtemplate codes) is named `template_for_cert`
4. The template entry is also excluded from `all.obj`

#### Config Unpacking Code Tables

All config decoders use the naming convention `code` for the byte value. Unrecognized codes panic with
`"Unrecognized {type} config code: {code}"`.

##### FLO (client codes)

| Code | Key     | Data  | Notes                                                                      |
|------|---------|-------|----------------------------------------------------------------------------|
| 1    | colour  | p3    | Hex RGB color (written as `0xRRGGBB`)                                      |
| 2    | texture | p1    | Texture ID -> real name from hardcoded table (e.g., `planks`, `water`)     |
| 3    | overlay | flag  | Written only if true                                                       |
| 5    | occlude | flag  | Inverted: written only if false                                            |
| 6    | --      | pjstr | Debug name -- used as entry name in flo.pack and section header in all.flo |

##### VARP (client codes)

| Code | Key        | Data |
|------|------------|------|
| 5    | clientcode | p2   |

All other varp codes (scope, type, protect, transmit) are server-only.

##### IDK (client codes)

| Code  | Key       | Data                  | Notes                                       |
|-------|-----------|-----------------------|---------------------------------------------|
| 1     | type      | p1                    | Body part enum (man_hair..woman_feet, 0-13) |
| 2     | model1..N | p1(count), p2 x count | Model IDs -> `_idk` names                   |
| 3     | disable   | flag                  |                                             |
| 40-49 | recol{N}s | p2                    | HSL16 -> reverse to RGB15                   |
| 50-59 | recol{N}d | p2                    | HSL16 -> reverse to RGB15                   |
| 60-69 | head{N}   | p2                    | Model IDs -> `_idk_head` names              |

IDK type determines gender: 0-6 = man, 7-13 = woman. This controls model directory placement.

##### SPOTANIM (client codes)

| Code  | Key       | Data | Notes                     |
|-------|-----------|------|---------------------------|
| 1     | model     | p2   | -> `_spotanim` name       |
| 2     | anim      | p2   | -> `seq.pack` name        |
| 3     | hasalpha  | flag |                           |
| 4     | resizeh   | p2   |                           |
| 5     | resizev   | p2   |                           |
| 6     | angle     | p2   |                           |
| 7     | ambient   | p1   |                           |
| 8     | contrast  | p1   |                           |
| 40-49 | recol{N}s | p2   | HSL16 -> reverse to RGB15 |
| 50-59 | recol{N}d | p2   | HSL16 -> reverse to RGB15 |

##### SEQ (client codes)

| Code | Key                           | Data                                | Notes                                                            |
|------|-------------------------------|-------------------------------------|------------------------------------------------------------------|
| 1    | frame{N}, iframe{N}, delay{N} | p1(count), then per frame: p2+p2+p2 | Frame from `anim.pack`, iframe 0xFFFF = absent, delay 0 = absent |
| 2    | loops                         | p2                                  |                                                                  |
| 3    | walkmerge                     | p1(count), p1 x count               | Values formatted as `label_{N}`                                  |
| 4    | stretches                     | flag                                |                                                                  |
| 5    | priority                      | p1                                  |                                                                  |
| 6    | replaceheldleft               | p2                                  | 0 = `hide`, otherwise `obj.pack` name (value - 512)              |
| 7    | replaceheldright              | p2                                  | Same as code 6                                                   |
| 8    | maxloops                      | p1                                  |                                                                  |

##### LOC (client codes)

| Code  | Key                  | Data                        | Notes                                                          |
|-------|----------------------|-----------------------------|----------------------------------------------------------------|
| 1     | model                | p1(count), then p2+p1 pairs | (model_id, shape) pairs -> `model_loc_{loc_id}_{shape_suffix}` |
| 2     | name                 | pjstr                       |                                                                |
| 3     | desc                 | pjstr                       |                                                                |
| 14    | width                | p1                          |                                                                |
| 15    | length               | p1                          |                                                                |
| 17    | blockwalk            | flag                        | Inverted                                                       |
| 18    | blockrange           | flag                        | Inverted                                                       |
| 19    | active               | p1                          | Boolean                                                        |
| 21    | hillskew             | flag                        |                                                                |
| 22    | sharelight           | flag                        |                                                                |
| 23    | occlude              | flag                        |                                                                |
| 24    | anim                 | p2                          | -> `seq.pack` name                                             |
| 25    | hasalpha             | flag                        |                                                                |
| 28    | wallwidth            | p1                          |                                                                |
| 29    | ambient              | p1                          | Signed i8                                                      |
| 30-34 | op1-op5              | pjstr                       |                                                                |
| 39    | contrast             | p1                          | Signed i8                                                      |
| 40    | recol{N}s, recol{N}d | p1(count), then p2+p2 pairs | Already HSL16, written as-is                                   |
| 60    | mapfunction          | p2                          |                                                                |
| 62    | mirror               | flag                        |                                                                |
| 64    | shadow               | flag                        | Inverted                                                       |
| 65-67 | resizex/y/z          | p2                          |                                                                |
| 68    | mapscene             | p2                          |                                                                |
| 69    | forceapproach        | p1                          | Flags -> direction name (north/east/south/west)                |
| 70-72 | offsetx/y/z          | p2                          |                                                                |
| 73    | forcedecor           | flag                        |                                                                |

**LOC Model Shape Suffixes**

Each (model_id, shape) pair produces a model name with a shape suffix:

| Shape | Suffix | Shape | Suffix | Shape | Suffix |
|-------|--------|-------|--------|-------|--------|
| 0     | `_1`   | 8     | `_t`   | 16    | `_g`   |
| 1     | `_2`   | 9     | `_5`   | 17    | `_h`   |
| 2     | `_3`   | 10    | `_8`   | 18    | `_z`   |
| 3     | `_4`   | 11    | `_9`   | 19    | `_x`   |
| 4     | `_q`   | 12    | `_a`   | 20    | `_c`   |
| 5     | `_w`   | 13    | `_s`   | 21    | `_v`   |
| 6     | `_r`   | 14    | `_d`   | 22    | `_0`   |
| 7     | `_e`   | 15    | `_f`   |       |        |

The loc config text references the base name `model_loc_{loc_id}` without suffix. The packer checks for `{base}_8` (
centrepiece), then `{base}_1` through `{base}_0` for all shapes.

##### NPC (client codes)

| Code  | Key                  | Data                   | Notes                                     |
|-------|----------------------|------------------------|-------------------------------------------|
| 1     | model1..N            | p1(count), p2 x count  | -> `_npc` names                           |
| 2     | name                 | pjstr                  |                                           |
| 3     | desc                 | pjstr                  |                                           |
| 12    | size                 | p1                     |                                           |
| 13    | readyanim            | p2                     | -> `seq.pack`                             |
| 14    | walkanim             | p2                     | Single direction -> `seq.pack`            |
| 16    | hasalpha             | flag                   |                                           |
| 17    | walkanim             | p2 x 4                 | Four directions, comma-separated seq refs |
| 30-34 | op1-op5              | pjstr                  |                                           |
| 40    | recol{N}s, recol{N}d | p1(count), p2+p2 pairs | Conditional RGB15->HSL16 reverse          |
| 60    | head1..N             | p1(count), p2 x count  | -> `_npc_head` names                      |
| 90-92 | resizex/y/z          | p2                     |                                           |
| 93    | minimap              | flag                   | Inverted                                  |
| 95    | vislevel             | p2                     | 0 = `hide`                                |
| 97-98 | resizeh/v            | p2                     |                                           |

##### OBJ (client codes)

| Code   | Key                                      | Data                   | Notes                                         |
|--------|------------------------------------------|------------------------|-----------------------------------------------|
| 1      | model                                    | p2                     | -> `_obj` name                                |
| 2      | name                                     | pjstr                  |                                               |
| 3      | desc                                     | pjstr                  |                                               |
| 4-6    | 2dzoom, 2dxan, 2dyan                     | p2                     |                                               |
| 7-8    | 2dxof, 2dyof                             | p2                     | Signed i16                                    |
| 9      | code9                                    | flag                   |                                               |
| 10     | code10                                   | p2                     | -> `seq.pack`                                 |
| 11     | stackable                                | flag                   |                                               |
| 12     | cost                                     | p4                     |                                               |
| 16     | members                                  | flag                   |                                               |
| 23     | manwear                                  | p2, p1                 | Model + index, comma-separated -> `_obj_wear` |
| 24     | manwear2                                 | p2                     | -> `_obj_wear`                                |
| 25     | womanwear                                | p2, p1                 | Same as manwear                               |
| 26     | womanwear2                               | p2                     | -> `_obj_wear`                                |
| 30-34  | op1-op5                                  | pjstr                  |                                               |
| 35-39  | iop1-iop5                                | pjstr                  |                                               |
| 40     | recol{N}s, recol{N}d                     | p1(count), p2+p2 pairs | Conditional RGB15->HSL16 reverse              |
| 78-79  | manwear3, womanwear3                     | p2                     | -> `_obj_wear`                                |
| 90-93  | manhead, womanhead, manhead2, womanhead2 | p2                     | -> `_obj_wear`                                |
| 95     | 2dzan                                    | p2                     |                                               |
| 97     | certlink                                 | p2                     | -> `obj.pack` name                            |
| 98     | certtemplate                             | p2                     | -> `obj.pack` name                            |
| 99-108 | count1-count10                           | p2, p2                 | Obj ref + count, comma-separated              |

#### Raw Binary Extraction

In addition to text config files, the raw `.dat`/`.idx` pairs are extracted to `_raw/config/` for CRC verification. This
enables exact Jag reassembly without going through the text->binary re-encoding pipeline.

---

## Interface

### Interface Packing

#### Overview

The interface Jag archive contains UI component definitions for the RuneScape rev-225 client. Every game panel,
inventory, dialogue box, skill tab, and interactive overlay is defined as a tree of interface components packed into
this archive.

Original CRC: `1614084464`. Client data CRC: `-2146838800`.

#### Jag Archive Structure

The interface Jag is unique among all archives: it uses whole-archive bzip2 compression, meaning the entire archive
payload is compressed as one block rather than compressing individual entries separately. The packing tool automatically
selects the optimal compression mode by trying both per-entry and whole-archive and picking the smaller output -- for
the interface archive, whole-archive consistently wins by ~38 bytes.

The archive contains a single entry:

```
interface (Jag archive, whole-archive compression)
└── data         -- all interface component data (written as "data", not "data.dat")
```

The `data` entry is the client `PackedData.dat` buffer written directly into the Jag via `jag.write_file("data", ...)`.

#### Source Format

Interface definitions are authored as `.if` text files in the content directory, collected by `collect_if_files`. Each
`.if` file defines one top-level interface and its child components.

##### File Structure

```
// Comments start with // or #
// Lines without = are skipped
// Blank lines are ignored

// Properties before any [section] apply to the root interface component
width=512
height=334

[com_0]
type=text
font=p12
text=Hello World
x=10
y=20

[com_1]
type=graphic
graphic=backbase1
layer=com_0
```

The root component is implicitly a `layer` type with dimensions 512x334. Each `[section]` declares a child component.
The `layer` property reassigns a child from the root to another component, establishing the parent-child hierarchy.

##### Pack Registry

The file `interface.pack` maps interface names and component names to numeric IDs:

```
0=player_kit_tailor_legs_man
1=player_kit_tailor_legs_man:com_0
2=player_kit_tailor_legs_man:com_1
```

Top-level interfaces use bare names (e.g., `player_kit_tailor_legs_man`). Child components use `interface:component`
notation. The `interface.order` file controls the serialization order of all component IDs.

#### Component Types

| Name            | ID | Description                                                                                                            |
|-----------------|----|------------------------------------------------------------------------------------------------------------------------|
| layer / overlay | 0  | Container that holds child components. `overlay` is functionally the same type but flagged differently in server data. |
| inv             | 2  | Inventory grid with item slots, drag/swap, and interaction options.                                                    |
| rect            | 3  | Colored rectangle, optionally filled.                                                                                  |
| text            | 4  | Text label with font, color, shadow, and active-state text.                                                            |
| graphic         | 5  | Sprite image with optional active-state graphic.                                                                       |
| model           | 6  | 3D model display with rotation, zoom, and animation.                                                                   |
| invtext         | 7  | Text-based inventory display with item names and interaction options.                                                  |

#### Button Types

| Name   | ID | Description                                                                                 |
|--------|----|---------------------------------------------------------------------------------------------|
| (none) | 0  | No button behavior.                                                                         |
| normal | 1  | Standard clickable button. Writes `option` string.                                          |
| target | 2  | Targeting button (use item on...). Writes `actionverb`, `action`, and `actiontarget` flags. |
| close  | 3  | Close button that dismisses the interface.                                                  |
| toggle | 4  | Toggle button with on/off state. Writes `option` string.                                    |
| select | 5  | Selection button. Writes `option` string.                                                   |
| pause  | 6  | Pause/continue button. Writes `option` string.                                              |

#### Script System

Components support conditional scripts that control visibility and state based on game variables. Up to 5 scripts can be
attached per component, each with a comparator and a sequence of opcodes.

##### Comparators

Each script has a comparator that defines how the computed value is tested:

| Name | ID |
|------|----|
| eq   | 1  |
| lt   | 2  |
| gt   | 3  |
| neq  | 4  |

A script definition in the `.if` source looks like: `script1=eq,100` (comparator, value).

##### Script Opcodes

Opcodes define what value to compute for comparison. Each opcode may consume additional operands:

| Opcode            | ID | Operands                      | Description                                      |
|-------------------|----|-------------------------------|--------------------------------------------------|
| stat_level        | 1  | stat name                     | Current boosted level of a stat.                 |
| stat_base_level   | 2  | stat name                     | Base (unboosted) level of a stat.                |
| stat_xp           | 3  | stat name                     | Current XP in a stat.                            |
| inv_count         | 4  | interface:component, obj name | Count of an item in an inventory component.      |
| pushvar           | 5  | varp name                     | Value of a player variable (varp).               |
| stat_xp_remaining | 6  | stat name                     | XP remaining to next level.                      |
| op7               | 7  | (none)                        | Unknown/unused.                                  |
| op8               | 8  | (none)                        | Unknown/unused.                                  |
| op9               | 9  | (none)                        | Unknown/unused.                                  |
| inv_contains      | 10 | interface:component, obj name | Whether an inventory contains an item (boolean). |
| runenergy         | 11 | (none)                        | Current run energy.                              |
| runweight         | 12 | (none)                        | Current carried weight.                          |
| testbit           | 13 | varp name, bit index          | Test a specific bit of a varp value.             |

Script opcodes are written in `.if` files as: `script1op1=stat_level,attack` or
`script1op2=inv_count,inventory:com_0,bronze_arrow`.

##### Operand Encoding

Each opcode writes a different number of words to the binary:

- **No extra operands** (op7, op8, op9, runenergy, runweight): just the opcode ID.
- **1 extra operand** (stat_level, stat_base_level, stat_xp, stat_xp_remaining, pushvar): opcode + stat/varp ID.
- **2 extra operands** (inv_count, inv_contains, testbit): opcode + component/varp ID + obj/bit ID.

The `count_script_ops` function pre-calculates the total number of words needed for all opcodes in a script, which is
written as a length prefix.

#### Stats

| Name        | ID |
|-------------|----|
| attack      | 0  |
| defence     | 1  |
| strength    | 2  |
| hitpoints   | 3  |
| ranged      | 4  |
| prayer      | 5  |
| magic       | 6  |
| cooking     | 7  |
| woodcutting | 8  |
| fletching   | 9  |
| fishing     | 10 |
| firemaking  | 11 |
| crafting    | 12 |
| smithing    | 13 |
| mining      | 14 |
| herblore    | 15 |
| agility     | 16 |
| thieving    | 17 |
| runecraft   | 20 |

Note the gap: IDs 18 and 19 are unused.

#### Fonts

| Name | ID | Description       |
|------|----|-------------------|
| p11  | 0  | Proportional 11px |
| p12  | 1  | Proportional 12px |
| b12  | 2  | Bold 12px         |
| q8   | 3  | Quill 8px         |

#### Action Target Flags

When `buttontype=target` or `type=inv`, the `actiontarget` property defines what entity types the action can target,
encoded as a bitmask:

| Target  | Flag |
|---------|------|
| obj     | 0x01 |
| npc     | 0x02 |
| loc     | 0x04 |
| player  | 0x08 |
| heldobj | 0x10 |

Multiple targets are comma-separated: `actiontarget=obj,npc`.

#### Binary Format

##### PackedData (dat + idx)

The interface pack produces a `PackedData` structure with two buffers:

- **dat**: `p2(count)` followed by opcode-terminated component entries.
- **idx**: `p2(count)` followed by `p2(length)` per entry.

Both server and client `PackedData` are produced. The client data is what gets assembled into the interface Jag archive.

##### Component Stream Layout

Components are serialized in `interface.order` sequence. The stream uses sentinel-delimited groups:

```
p2(0xFFFF)           -- group separator (signals new root interface)
p2(root_id)          -- ID of the root interface component

p2(component_id)     -- ID of this component
```

A new group header (`0xFFFF` + root ID) is emitted whenever the root interface changes. Components within the same root
interface share the group header.

##### Server-Only Fields

The server stream includes two additional fields per component that the client stream omits:

```
pjstr(debug_name)    -- human-readable component name (e.g., "inventory:com_5")
pbool(is_overlay)    -- true if type is "overlay" rather than "layer"
```

##### Common Fields (Both Client and Server)

Every component writes the following fields:

```
p1(type)             -- component type (0-7)
p1(buttontype)       -- button behavior (0-6)
p2(clientcode)       -- client-side handler code
p2(width)            -- component width in pixels
p2(height)           -- component height in pixels
```

**Overlayer**

```
if overlayer exists:
    p2(layer_id + 0x100)   -- overlay layer reference (offset by 256)
else:
    p1(0)                  -- no overlayer
```

**Comparators**

```
p1(comparator_count)       -- number of conditional scripts (0-5)
for each comparator:
    p1(comparator)         -- eq/lt/gt/neq (1-4)
    p2(value)              -- comparison target value
```

**Script Arrays**

```
p1(script_count)           -- number of script opcode arrays (0-5)
for each script:
    p2(op_count)           -- total word count including opcodes and operands
    for each op:
        p2(opcode)         -- script opcode ID
        [p2(operand)]...   -- 0, 1, or 2 additional words depending on opcode
    p2(0)                  -- terminator (if any ops were written)
```

##### Type-Specific Fields

**Layer (type 0)**

```
p2(scroll)                 -- scroll height in pixels
pbool(hide)                -- initially hidden
p1(child_count)            -- number of child components
for each child:
    [client only] p2(child_id)
    p2(x)                  -- child X position (signed, cast to u16)
    p2(y)                  -- child Y position (signed, cast to u16)
```

Note: child IDs are only written to the client stream. The server stream omits them, writing only x/y positions.

**Inv (type 2)**

```
pbool(draggable)           -- items can be dragged
pbool(interactable)        -- items can be interacted with
pbool(usable)              -- items can be used on targets
p1(margin_x)               -- horizontal slot margin
p1(margin_y)               -- vertical slot margin
for slot 1..20:
    pbool(has_slot)
    if has_slot:
        p2(offset_x)       -- sprite X offset within slot
        p2(offset_y)       -- sprite Y offset within slot
        pjstr(sprite)      -- slot background sprite name
for option 1..5:
    pjstr(option_text)     -- right-click menu option text
```

Slot definitions use colon-separated format: `slot1=sprite_name:x,y`.

**Rect (type 3)**

```
pbool(fill)                -- filled or outline only
```

**Text (type 4)**

```
pbool(center)              -- center-aligned text
p1(font)                   -- font ID (0-3)
pbool(shadowed)            -- text shadow enabled
pjstr(text)                -- default text content
pjstr(activetext)          -- text when component is active/selected
```

**Rect and Text shared (types 3 and 4)**

```
p4(colour)                 -- default color (hex RGB, e.g., 0xFF0000)
p4(activecolour)           -- color when active
p4(overcolour)             -- color when hovered
```

**Graphic (type 5)**

```
pjstr(graphic)             -- sprite name
pjstr(activegraphic)       -- sprite when active/selected
```

**Model (type 6)**

Model and animation references use a `+0x100` offset encoding to distinguish "has value" from "no value":

```
if model exists:
    p2(model_id + 0x100)
else:
    p1(0)

if activemodel exists:
    p2(activemodel_id + 0x100)
else:
    p1(0)

if anim exists:
    p2(seq_id + 0x100)
else:
    p1(0)

if activeanim exists:
    p2(seq_id + 0x100)
else:
    p1(0)

p2(zoom)                   -- camera zoom level
p2(xan)                    -- X-axis rotation angle
p2(yan)                    -- Y-axis rotation angle
```

**Invtext (type 7)**

```
pbool(center)              -- center-aligned text
p1(font)                   -- font ID (0-3)
pbool(shadowed)            -- text shadow enabled
p4(colour)                 -- text color (hex RGB)
p2(margin_x)               -- horizontal margin
p2(margin_y)               -- vertical margin
pbool(interactable)        -- items can be interacted with
for option 1..5:
    pjstr(option_text)     -- right-click menu option text
```

##### Button-Specific Fields

**Target buttons (buttontype 2) and Inv components (type 2)**

```
pjstr(actionverb)          -- verb displayed during targeting (e.g., "Use", "Cast")
pjstr(action)              -- action description
p2(flags)                  -- action target bitmask (obj=0x01, npc=0x02, loc=0x04, player=0x08, heldobj=0x10)
```

**Buttons with option text (buttontypes 1, 4, 5, 6)**

```
pjstr(option)              -- button label / menu option text
```

#### Component Properties Reference

| Property                | Types               | Description                                                                    |
|-------------------------|---------------------|--------------------------------------------------------------------------------|
| type                    | all                 | Component type name (layer, overlay, inv, rect, text, graphic, model, invtext) |
| buttontype              | all                 | Button behavior (normal, target, close, toggle, select, pause)                 |
| clientcode              | all                 | Client-side handler identifier                                                 |
| width                   | all                 | Width in pixels                                                                |
| height                  | all                 | Height in pixels                                                               |
| x                       | children            | X position relative to parent                                                  |
| y                       | children            | Y position relative to parent                                                  |
| layer                   | children            | Parent component name (reassigns from root)                                    |
| overlayer               | all                 | Overlay layer reference                                                        |
| scroll                  | layer               | Scroll content height                                                          |
| hide                    | layer               | Initially hidden (yes/no)                                                      |
| draggable               | inv                 | Allow item dragging (yes/no)                                                   |
| interactable            | inv, invtext        | Allow item interaction (yes/no)                                                |
| usable                  | inv                 | Allow use-on targeting (yes/no)                                                |
| margin                  | inv, invtext        | Slot spacing as "x,y"                                                          |
| slot1..slot20           | inv                 | Slot sprite and offset as "sprite:x,y"                                         |
| option1..option5        | inv, invtext        | Right-click menu options                                                       |
| fill                    | rect                | Filled rectangle (yes/no)                                                      |
| center                  | text, invtext       | Center text alignment (yes/no)                                                 |
| font                    | text, invtext       | Font name (p11, p12, b12, q8)                                                  |
| shadowed                | text, invtext       | Text shadow (yes/no)                                                           |
| text                    | text                | Default display text                                                           |
| activetext              | text                | Text when active                                                               |
| colour                  | rect, text, invtext | Default color as hex                                                           |
| activecolour            | rect, text          | Color when active                                                              |
| overcolour              | rect, text          | Color on hover                                                                 |
| graphic                 | graphic             | Sprite name                                                                    |
| activegraphic           | graphic             | Sprite when active                                                             |
| model                   | model               | 3D model name                                                                  |
| activemodel             | model               | Model when active                                                              |
| anim                    | model               | Animation sequence name                                                        |
| activeanim              | model               | Animation when active                                                          |
| zoom                    | model               | Camera zoom level                                                              |
| xan                     | model               | X-axis rotation                                                                |
| yan                     | model               | Y-axis rotation                                                                |
| actionverb              | target/inv          | Targeting verb                                                                 |
| action                  | target/inv          | Action description                                                             |
| actiontarget            | target/inv          | Target flags as comma list (obj, npc, loc, player, heldobj)                    |
| option                  | buttons             | Button label text                                                              |
| script1..script5        | all                 | Comparator as "comparator,value"                                               |
| script1op1..script5op20 | all                 | Script opcode as "opcode,args..."                                              |

#### Interface CRC Verification

Two CRC checks are performed:

1. **Client data CRC** (`-2146838800`): Verified against the raw `PackedData.dat` bytes before Jag assembly. This
   ensures the component serialization is byte-identical to the original.

2. **Jag archive CRC** (`1614084464`): Verified against the final Jag archive bytes after whole-archive bzip2
   compression. This ensures the Jag assembly and compression produce identical output.

#### Interface Shared Enums

Component types, button types, fonts, stats, script ops, and comparators are defined as shared enums in
`rs-pack/src/types.rs`:

- `IfComponentType` -- Layer=0, Inv=2, Rect=3, Text=4, Graphic=5, Model=6, InvText=7
- `IfButtonType` -- None=0, Normal=1, Target=2, Close=3, Toggle=4, Select=5, Pause=6
- `FontId` -- P11=0, P12=1, B12=2, Q8=3
- `StatId` -- Attack=0 through Runecraft=20
- `IfScriptOp` -- StatLevel=1 through TestBit=13
- `IfComparator` -- Eq=1, Lt=2, Gt=3, Neq=4

#### Interface Packing Implementation

- **Source**: `rs-pack/src/pack/interface/mod.rs` -- `pack_interfaces` function
- **Jag assembly**: `rs-pack/src/lib.rs` -- `assemble_interface_jag` function
- **File collection**: `rs-pack/src/pack/pack.rs` -- `collect_if_files` function
- **Pack registry**: `content/pack/interface.pack` -- name-to-ID mapping
- **Component order**: `content/pack/interface.order` -- serialization order
- **Cache decoder**: `rs-pack/src/cache/if.rs` -- `IfTypeProvider`
- **Shared enums**: `rs-pack/src/types.rs`

### Interface Unpacking

Extracts the raw interface data entry from the interface Jag archive for CRC verification.

#### Input

The `interface` Jag archive file (CRC: `1614084464`), using **whole-archive** bzip2 compression. Contains a single
entry: `data`.

#### Output

```
content_unpack/_raw/interface/
├── _jag_order.txt       -- entry name order (just "data")
└── data                 -- raw decompressed interface binary
```

#### Current Approach: Raw Extraction

The interface Jag uses whole-archive compression -- the entire payload (file table + all data) is compressed as a single
bzip2 block. The single `data` entry contains all UI component definitions in a complex binary format (see
the [interface packing](#interface-packing) section for the full specification).

Currently, the interface is extracted as a raw binary blob rather than decoded into `.if` text files. This is because:

1. The interface binary format is highly complex -- component trees with type-specific fields, script arrays, and
   cross-references to other registries (model, seq, obj, varp, interface)
2. Full text roundtrip would require generating `interface.pack` (7,386 entries) and `interface.order` files
3. The raw extraction is sufficient for CRC verification

#### CRC Verification

The raw `data` entry is reassembled into a Jag via `JagFile::new()` + `build()`, which automatically selects
whole-archive compression (since it produces a smaller output for a single entry). The CRC is compared against the
expected value. The whole-archive compression means the Jag header has `packed_size != unpacked_size`, and the entire
payload is compressed as one block.

#### Future: Full Text Decode

A complete interface unpacker would need to:

1. Parse the component stream (sentinel-delimited groups with `0xFFFF` separators)
2. Decode all component types (layer, inv, rect, text, graphic, model, invtext)
3. Decode script arrays with variable-length opcode chains
4. Resolve cross-references (model IDs, seq IDs, varp IDs, interface component IDs)
5. Generate `interface.pack` with `interface_name:component_name` notation
6. Generate `interface.order` for serialization ordering
7. Reconstruct the parent-child hierarchy (layer assignments)
8. Write `.if` text files with section headers and key=value properties

---

## Map

### Map Packing

Maps are NOT a Jag archive -- they are individual compressed files keyed by (type, map_x, map_z).

#### Source

Content is read from the `content/maps/` directory containing `.jm2` text files named `m{mx}_{mz}.jm2`.

Each `.jm2` file can contain up to 4 sections, marked by header lines:

- `==== MAP ====` -- terrain tile data
- `==== LOC ====` -- location/scenery placements
- `==== NPC ====` -- NPC spawn points
- `==== OBJ ====` -- ground item placements

#### Section Formats

All sections use the format: `level local_x local_z: data`

- level: 0-3 (height plane)
- local_x: 0-63 (tile X within the map square)
- local_z: 0-63 (tile Z within the map square)

##### MAP Section

Data contains space-separated tile properties:

- `h{value}` -- height
- `o{id}` or `o{id};{shape}` or `o{id};{shape};{rotation}` -- overlay
- `f{value}` -- flags
- `u{value}` -- underlay

##### LOC Section

Data format: `loc_id [shape] [angle]`

- loc_id: location/scenery type ID
- shape: placement shape (default 10 = centrepiece_straight)
- angle: rotation 0-3 (default 0)

##### NPC Section

Data format: `npc_id`

- npc_id: NPC type ID to spawn at this position

##### OBJ Section

Data format: `obj_id [count]`

- obj_id: item/object type ID
- count: stack count (default 1)

#### Binary Encoding

Each section type produces a separate compressed binary file.

##### Terrain ('m' prefix)

Encodes all 4 * 64 * 64 = 16,384 tiles sequentially (level, then x, then z):

- If tile is completely default (height=0, no overlay, no flags, no underlay): `p1(0)`
- If overlay set: `p1(opcode)` where opcode = 2 + (shape << 2) + rotation, then `p1(overlay_id)`
- If flags set: `p1(flags + 49)`
- If underlay set: `p1(underlay + 81)`
- If height != 0: `p1(1) p1(height)`, else `p1(0)`

##### Locations ('l' prefix)

Delta-encoded by loc_id using psmart (variable-length 1-2 byte encoding):

```
for each loc_id (sorted, delta from previous):
    psmart(id_delta)
    for each placement:
        psmart(position_delta + 1)   // position = (level << 12) | (x << 6) | z
        p1(shape << 2 | angle & 0x3)
    psmart(0)  // end placements
psmart(0)  // end all locs
```

##### NPCs ('n' prefix)

Same delta encoding as locs but without shape/angle:

```
for each npc_id (sorted, delta from previous):
    psmart(id_delta)
    for each placement:
        psmart(position_delta + 1)
    psmart(0)
psmart(0)
```

##### Objects ('o' prefix)

Same delta encoding with an additional count field:

```
for each obj_id (sorted, delta from previous):
    psmart(id_delta)
    for each placement:
        psmart(position_delta + 1)
        p2(count)
    psmart(0)
psmart(0)
```

#### Map Compression

Each encoded binary is compressed with bzip2 (block size 1) and prepended with a 4-byte uncompressed size header:

```
[p4(uncompressed_size)] [bzip2_data_without_BZh1_header]
```

CRC32 is computed on the compressed output for each file.

#### Output Keys

Maps are stored in a `HashMap<MapSquare, Vec<u8>>` where:

- char: `'m'` (terrain), `'l'` (locs), `'n'` (npcs), `'o'` (objs)
- u8: map_x coordinate
- u8: map_z coordinate

#### psmart Encoding

Variable-length encoding:

- Values 0-127: written as `p1(value)`
- Values 128-32767: written as `p2(value + 32768)`

#### Map Packing Performance

Map packing reads all files as raw bytes (no UTF-8 validation), reuses tile and encoder buffers across files, and calls
`rs_bzip2::compress` directly to avoid unnecessary copies.

### Map Unpacking

Decodes individual compressed map files back into human-readable `.jm2` text files.

#### Input

Individual compressed files in `maps/`, prefixed by type character:

- `m{x}_{z}` -- terrain data
- `l{x}_{z}` -- location placements

Each file has a 4-byte uncompressed size header followed by bzip2-compressed data (BZh1 header stripped).

#### Output

```
content_unpack/maps/
├── m29_75.jm2
├── m30_75.jm2
├── m50_50.jm2
└── ...
```

Each `.jm2` file combines all available data for a map square into sections.

#### File Format

```
==== MAP ====
0 0 0: u48
0 0 1: u48
0 0 2: h30 u48
...

==== LOC ====
0 0 0: 1247 22 3
0 0 4: 1911 3 2
...

==== NPC ====

==== OBJ ====
```

All four section headers are always present, even if the section has no data. Sections are separated by a blank line
before the header.

#### Terrain Decoding

The terrain binary encodes all 4 x 64 x 64 = 16,384 tiles sequentially (level outer, x middle, z inner).

##### Binary Format Per Tile

Each tile is a sequence of opcodes read until a terminator:

| Opcode | Meaning                   | Data                                                               |
|--------|---------------------------|--------------------------------------------------------------------|
| 0      | Terminator (default tile) | --                                                                 |
| 1      | Height + terminator       | p1(height)                                                         |
| 2-49   | Overlay                   | p1(overlay_id); shape = (opcode-2) >> 2; rotation = (opcode-2) & 3 |
| 50-81  | Flags                     | flags = opcode - 49                                                |
| 82+    | Underlay                  | underlay = opcode - 81                                             |

Opcodes 2-49 and 50+ are not terminators -- multiple can appear per tile. Opcodes 0 and 1 terminate the tile.

##### Text Output Per Tile

Only tiles with non-default data are written. Format: `{level} {x} {z}: {properties}`

Properties are space-separated:

- `h{value}` -- height (only if non-zero from opcode 1)
- `o{id}` or `o{id};{shape}` or `o{id};{shape};{rotation}` -- overlay (shape and rotation omitted if both zero)
- `f{value}` -- flags
- `u{value}` -- underlay

#### Location Decoding

Locations use delta-encoded IDs with smart values (psmart: 1 byte for 0-127, 2 bytes for 128-32767).

##### Binary Format

```
loop:
    id_delta = gsmart()           -- if 0, end all locs
    loc_id += id_delta
    
    last_pos = 0
    loop:
        pos_delta = gsmart()      -- if 0, end this loc's placements
        last_pos += pos_delta - 1
        level = (last_pos >> 12) & 3
        x = (last_pos >> 6) & 63
        z = last_pos & 63
        loc_info = g1()
        shape = loc_info >> 2
        angle = loc_info & 3
```

##### Text Output Per Placement

Format: `{level} {x} {z}: {loc_id} {shape}` or `{level} {x} {z}: {loc_id} {shape} {angle}` (angle omitted if 0).

Shape is always written (unlike the packing side where shape 10 is the default for centrepiece). This ensures all
placements roundtrip correctly regardless of model resolution logic.

#### NPC and OBJ Sections

NPC and OBJ data is **not** present in the expected map files (only `m` terrain and `l` location files exist). Empty
section headers are written to maintain the `.jm2` format:

```
==== NPC ====

==== OBJ ====
```

If NPC/OBJ expected files were present (`n{x}_{z}`, `o{x}_{z}`), their binary format would use the same delta-encoding
as locations:

- NPC: `gsmart(id_delta)`, positions, no extra data per placement
- OBJ: `gsmart(id_delta)`, positions, `p2(count)` per placement

---

## Media (Sprites)

### Media Packing

The media Jag archive contains all in-game sprite graphics -- interface elements, icons, map markers, cursors, and other
2D assets.

Original CRC: `-343404987`

The archive uses per-entry bzip2 compression.

#### Jag Structure

```
media (Jag archive)
├── index.dat          -- shared metadata for all sprites
├── backbase1.dat      -- pixel data for backbase1
├── chatback.dat       -- pixel data for chatback
├── combaticons.dat    -- pixel data for combaticons
└── ...                -- one .dat per sprite group
```

All sprite groups share a single `index.dat` that contains tile dimensions, palettes, and per-sub-sprite crop metadata.
Each sprite group has its own `{name}.dat` containing pixel indices.

#### Source Directory

```
content/sprites/
├── backbase1.tga     -- one indexed sprite-sheet TGA per group
├── combaticons.tga
├── ...
└── meta/
    ├── index.order   -- group keys in index.dat order
    └── sprite.order  -- order entries appear in the Jag file table
```

Each sprite group is one **indexed (color-mapped) TGA sprite sheet**, `<key>.tga`, that carries everything needed to
repack it byte-exact inside native TGA fields:

- the **color map** carries the palette -- entry 0 magenta (`0xFF00FF` transparent), entries 1..N the real colors in
  cache order, including any entries no pixel uses. A multi-frame sheet appends one extra **decorative gridline entry**
  (neutral grey `0x808080`) at the end; it is used only in the grid gaps, and pack strips it, so it never reaches the
  cache;
- the **image ID** field carries the grid dimensions -- `tileW`, `tileH`, `frameCount` as three big-endian u16s (6
  bytes), so the sheet is self-describing;
- the **pixels** are the frames laid out in a grid whose column count is chosen so the sheet is as close to square as
  possible in pixels (given the tile aspect ratio), rather than one long row. A single-frame group is a plain
  `tileW x tileH` tile; a multi-frame group is a mini atlas with a visible 1px gridline (the extra color-map entry)
  around every cell. Cell interiors and empty cells are index 0 (magenta).

The palette (order + unused entries) is the one thing not derivable from the frame pixels (see
[Source Encoding](#source-encoding)); the TGA color map carries it natively, so there is no separate metadata file.

#### Ordering Files

**`index.order`** -- one group key per line, in `index.dat` order. Drives packing: the group's `.dat` metadata is
written in this order (setting each `indexPos`). This order is derived from the original Jagex rev-225 cache and cannot
be algorithmically reproduced.

**`sprite.order`** -- one entry name per line, including `index`. Controls the order entries appear in the Jag file
table (independent of `index.dat` order).

#### Binary Format

##### index.dat (shared, sequential)

For each sprite group (in index order):

```
p2(tileWidth)
p2(tileHeight)
p1(paletteCount)               -- includes transparent index 0
[p3(rgb24)] x (paletteCount-1) -- palette colors for indices 1..N
```

Then for each sub-sprite within the group:

```
p1(cropX)          -- X offset of content within tile
p1(cropY)          -- Y offset of content within tile
p2(contentWidth)   -- width of content region
p2(contentHeight)  -- height of content region
p1(pixelOrder)     -- 0 = row-major, 1 = column-major
```

##### {name}.dat (per sprite group)

```
p2(indexPos)       -- byte offset into index.dat where this sprite's metadata starts
[p1(paletteIndex)] x (contentWidth x contentHeight)  -- per sub-sprite, sequentially
```

Pixel data contains only the cropped content region. Palette index 0 is always transparent (`0xFF00FF`).

For `pixelOrder = 0` (row-major): pixels stored left-to-right, top-to-bottom.
For `pixelOrder = 1` (column-major): pixels stored top-to-bottom, left-to-right (`for x { for y { ... } }`).

#### Source Encoding

Each `<key>.tga` is an uncompressed 8-bit color-mapped TGA (type 1, bottom-left origin -- the classic Targa defaults).
The reader also accepts RLE-compressed (type 9), 32-bit color maps, and top-left origin so sheets survive a round trip
through common image editors. At pack time each sheet's grid dimensions are read from the image ID, its palette from
the color map, and each frame's cell sliced straight out (the pixels are already palette indices -- no inversion
needed). Palette index 0 is magenta `0xFF00FF` (transparent).

The palette lives in the TGA color map (rather than derived from the frame pixels) because it **cannot** be recovered
from those pixels:

- **Order** is not recoverable. A first-encounter pixel scan reproduces the original palette order for only about a
  quarter of groups; the rest match no scan order. Order matters because the colors are written to `index.dat` in that
  order and every `{name}.dat` pixel is an index into it -- reorder and both byte streams change.
- **Unused entries** are not recoverable. Several groups (e.g. the `back*` steel border pieces) carry palette colors no
  pixel references. Nothing in the pixels can bring them back.

Both are artifacts of the color maps in Jagex's original indexed source images, so the TGA color map preserves them
natively. Edit sheets only in a tool that preserves indexed TGA (the color map and its order) and the image ID field;
a tool that flattens to truecolor or drops the image ID breaks byte-exactness.

##### Parameter Derivation

Per-group parameters come from native TGA fields; per-frame parameters are re-derived from the cell pixels at pack
time:

| Parameter        | Derivation                                                     |
|------------------|----------------------------------------------------------------|
| tileW x tileH    | Image ID field                                                |
| Sub-sprite count | Image ID field (`frameCount`)                                 |
| Palette          | TGA color map (colors + order, magenta entry 0)               |
| Crop region      | Bounding box of non-zero palette indices, per frame           |
| Pixel order      | Run-length scoring: row-major vs column-major, per frame      |

##### Crop Detection

The content region is the tight bounding box of all non-zero (non-transparent) pixels:

- The bounding box exactly reproduces `cropX, cropY, contentWidth, contentHeight`
- Fully transparent sub-sprites are stored as a full tile of index 0 (`cropX=0, cropY=0, contentWidth=tileWidth,
  contentHeight=tileHeight`), matching the original Jagex packer. These occur as placeholder frames (e.g. unused
  `combaticons3` slots) and empty font glyphs.

##### Pixel Order Determination

The `pixelOrder` byte controls whether pixel data is stored row-major or column-major. The order that produces more
consecutive identical palette indices is chosen for better bzip2 compression:

```
row_runs = consecutive identical palette indices in row-major order
col_runs = consecutive identical palette indices in column-major order
pixelOrder = if col_runs > row_runs { 1 } else { 0 }
```

#### Black Color Handling

The client decoder replaces palette color `0x000000` (black) with `0x000001` (almost-black) during rendering. Source
frames should contain true black -- the encoder writes `0x000000` to the palette, and the client adjusts at decode
time.

### Media Unpacking

Extracts all in-game sprite graphics from the media Jag archive into one indexed sprite-sheet TGA per group.

#### Input

The `media` Jag archive file (CRC: `-343404987`), using per-entry bzip2 compression.

#### Output

```
content_unpack/sprites/
├── backbase1.tga     -- one indexed sprite-sheet TGA per group
├── combaticons.tga
├── ...
└── meta/
    ├── index.order   -- group keys in index.dat order
    └── sprite.order  -- Jag file table order (derived from Jag entry sequence)
```

#### Hash-to-Name Resolution

The Jag file table stores filename hashes, not names. A hardcoded table of known sprite names is used to reverse the
hash:

```
backbase1, backbase2, backhmid1, backhmid2, backleft1, backleft2,
backright1, backright2, backtop1, backtop2, backvmid1, backvmid2,
backvmid3, chatback, combatboxes, combaticons, combaticons2,
combaticons3, compass, cross, gnomeball_buttons, headicons, hitmarks,
index, invback, leftarrow, magicoff, magicoff2, magicon, magicon2,
mapback, mapdots, mapflag, mapfunction, mapscene, miscgraphics,
miscgraphics2, miscgraphics3, prayerglow, prayeroff, prayeron,
redstone1, redstone2, redstone3, rightarrow, scrollbar, sideicons,
staticons, staticons2, steelborder, steelborder2, sworddecor,
tradebacking, wornicons
```

Each candidate name is hashed using the Jag hash function (`hash = hash * 61 + (uppercase_char - 32)`) and compared
against the file table entry.

#### Index Order Derivation

`index.order` lists the group keys in `index.dat` order (which also determines the `indexPos` offsets). This order
cannot be derived from the Jag file table order -- it is recovered from the binary data.

Each sprite's `.dat` entry begins with `p2(indexPos)` -- the byte offset into `index.dat` where that sprite's metadata
starts. The order is recovered by sorting sprites by their `indexPos` value (ascending), then written as `index.order`.

#### Sprite Decoding

Each sprite group is decoded from its binary `index.dat` metadata and `.dat` pixel data, then written as one indexed
sprite-sheet TGA (color map = palette, image ID = grid dims, pixels = frame grid).

##### Reading index.dat

Starting at the byte offset specified by the sprite's `indexPos`:

```
p2(tileWidth)
p2(tileHeight)
p1(paletteCount)
[p3(rgb24)] x (paletteCount - 1)    -- palette colors for indices 1..N
```

Palette index 0 is always `0xFF00FF` (transparent magenta) and is not stored in the binary.

##### Reading per-sub-sprite metadata

For each sub-sprite (read sequentially until `.dat` data is exhausted):

```
p1(cropX)
p1(cropY)
p2(contentWidth)
p2(contentHeight)
p1(pixelOrder)       -- 0 = row-major, 1 = column-major
```

##### Reading pixel data

From the `.dat` file (after the 2-byte `indexPos` header), pixel indices are read according to `pixelOrder`:

- **Row-major (0)**: `contentWidth x contentHeight` bytes in left-to-right, top-to-bottom order
- **Column-major (1)**: same count, but read top-to-bottom then left-to-right (`for x { for y { ... } }`)

##### Sheet Reconstruction

Each group is written as one indexed sprite-sheet TGA:

1. Decode every sub-sprite into a `tileWidth x tileHeight` index buffer (index 0 background, pixel indices placed at
   `(cropX, cropY)` unchanged)
2. Write the color map: magenta index 0 + palette colors 1..N from `index.dat`, in order (including unused entries); for
   a multi-frame group append one extra grey `0x808080` gridline entry (index N+1)
3. Write the image ID field: `tileWidth`, `tileHeight`, `frameCount` as three big-endian u16s
4. Lay the frames into a grid whose column count makes the sheet as close to square as possible in pixels (not one long
   row): single-frame groups are a plain tile, multi-frame groups get a visible 1px gridline (the extra entry) around
   every cell; cell interiors and empty cells stay index 0. On repack the gridline entry is dropped, so `index.dat`
   keeps only the real palette

##### Sub-sprite Count

The number of sub-sprites is not stored explicitly in the binary. It is determined by consuming pixel data from the
`.dat` file until no bytes remain (after the 2-byte `indexPos` header). In the source tree it is the `frameCount` in
the sheet's image ID field.

#### Roundtrip Guarantee

The indexed sprite sheet preserves all parameters needed for exact re-packing:

| Parameter                | Preserved By                                                         |
|--------------------------|----------------------------------------------------------------------|
| Palette colors and order | TGA color map (including entries unused by any pixel)               |
| Pixel indices            | Sheet cell pixels (already palette indices)                        |
| Tile dimensions          | Image ID field                                                    |
| Sub-sprite count         | Image ID field (`frameCount`)                                      |
| Crop bounds              | Bounding box of non-zero pixels per frame (full tile when empty)     |
| Pixel order              | Re-derived from run-length heuristic (deterministic from pixel data) |

The `examples/sprite_roundtrip.rs` example verifies this: it packs the content sprites into media and title Jags,
unpacks them, re-packs the result, and asserts byte-identical output.

---

## Model

### Model Packing

Packs 3D models (.ob2), animation frames (.frame), and skeleton bases (.base) into the models Jag archive.

Original CRC: `-2000991154`

The archive uses per-entry bzip2 compression.

#### Data Streams

The Jag contains 21 data streams, assembled in the following order:

| #  | Stream Name       |
|----|-------------------|
| 1  | `base_label.dat`  |
| 2  | `ob_point1.dat`   |
| 3  | `ob_point2.dat`   |
| 4  | `ob_point3.dat`   |
| 5  | `ob_point4.dat`   |
| 6  | `ob_point5.dat`   |
| 7  | `ob_head.dat`     |
| 8  | `base_head.dat`   |
| 9  | `frame_head.dat`  |
| 10 | `frame_tran1.dat` |
| 11 | `frame_tran2.dat` |
| 12 | `ob_vertex1.dat`  |
| 13 | `ob_vertex2.dat`  |
| 14 | `frame_del.dat`   |
| 15 | `base_type.dat`   |
| 16 | `ob_face1.dat`    |
| 17 | `ob_face2.dat`    |
| 18 | `ob_face3.dat`    |
| 19 | `ob_face4.dat`    |
| 20 | `ob_face5.dat`    |
| 21 | `ob_axis.dat`     |

#### Source Files

Source files are located in the `content/models/` directory:

- `.ob2` files -- 3D mesh data (vertices, faces, textures), binary format
- `.frame` files -- animation frame data, human-readable text format
- `.base` files -- skeleton base definitions, human-readable text format

#### Ordering Files

Ordering files are located in `content/pack/` and control the packing order via numeric IDs:

- `model.order` -- numeric IDs controlling ob2 packing order
- `anim.order` -- numeric IDs controlling frame packing order
- `base.order` -- numeric IDs controlling base packing order

#### Registry Files

Registry files are also located in `content/pack/` and map IDs to debug names:

- `model.pack` -- maps model ID to debugname (e.g. `0=model_0_npc_head`)
- `anim.pack` -- maps anim ID to debugname (e.g. `0=anim_0`)
- `base.pack` -- maps base ID to debugname (e.g. `0=base_0`)

#### Base Format (.base)

Human-readable text files defining animation skeletons. Each bone line combines a transform type with the vertex group
labels that bone controls.

##### Bone Types

Defined in the `BoneType` enum (`types.rs`):

| Value | Name        | Description                    |
|-------|-------------|--------------------------------|
| 0     | `translate` | Move position (X, Y, Z offset) |
| 1     | `rotate`    | Orientation change             |
| 2     | `scale`     | Resize                         |
| 3     | `alpha`     | Transparency                   |
| 5     | `origin`    | Reference point for transforms |

##### File Format

```
[base_0]
bone0=translate,2
bone1=scale,8,9,10,11,0,5,1,7
bone2=translate
bone3=translate,19,24
bone4=rotate,25,18
```

- Each `boneN` line starts with the transform type name.
- Remaining comma-separated values are vertex group IDs this bone controls.
- A bone with no vertex groups (e.g. `bone2=translate`) is a root/identity transform.
- Bones are numbered sequentially starting from 0.

##### Base Packing Process

Each `.base` file is parsed and split across three binary streams:

1. **`base_head`** receives: `p2(id)`, `p1(bone_count)`
2. **`base_type`** receives: one byte per bone (the `BoneType` enum value)
3. **`base_label`** receives: per bone, `p1(label_count)` then `label_count` x `p1(label_id)`

The `base_head` stream is prefixed with `p2(total_count)`, `p2(highest_id)` before any per-base entries.

#### Frame Format (.frame)

Human-readable text files defining a single animation pose. Each frame references a base skeleton and provides per-bone
transform deltas.

##### File Format

```
[anim_0]
delay=8
base=base_0
bone0=z,48
bone1=none
bone4=y,59
bone5=xy,10,20
bone6=xyz,1,2,3
bone17=x,62
```

- **`delay`** -- frame duration in client ticks (1 tick = 20ms). A delay of 8 = 160ms.
- **`base`** -- reference to the skeleton by its pack name (e.g. `base_0`). Must exist in `base.pack` when packing; the
  packer will panic if the reference is invalid.
- **`boneN`** -- transform for bone N of the referenced base:
    - `none` -- no transform applied to this bone
    - `x,48` -- X axis delta of 48
    - `y,59` -- Y axis delta of 59
    - `z,48` -- Z axis delta of 48
    - `xy,10,20` -- X delta=10, Y delta=20
    - `xz,5,30` -- X delta=5, Z delta=30
    - `yz,10,20` -- Y delta=10, Z delta=20
    - `xyz,1,2,3` -- X=1, Y=2, Z=3

##### Axis Flags

The axis name maps to a bitmask (bit 0=X, bit 1=Y, bit 2=Z):

| Name   | Flag value | Axes  |
|--------|------------|-------|
| `none` | 0          | --    |
| `x`    | 1          | X     |
| `y`    | 2          | Y     |
| `xy`   | 3          | X+Y   |
| `z`    | 4          | Z     |
| `xz`   | 5          | X+Z   |
| `yz`   | 6          | Y+Z   |
| `xyz`  | 7          | X+Y+Z |

##### Delta Encoding

Delta values are encoded using `gsmart` (unsigned smart) format:

- Values 0-127: encoded as 1 byte
- Values 128-32767: encoded as 2 bytes (value + 32768, high bit set)

The packer uses `Packet::psmart()` to write deltas. The unpacker uses `Packet::gsmart()` to read them.

##### Frame Packing Process

Each `.frame` file is parsed and split across four binary streams:

1. **`frame_head`** receives: `p2(id)`, `p2(base_id)`, `p1(bone_count)`
2. **`frame_del`** receives: `p1(delay)`
3. **`frame_tran1`** receives: per bone, `p1(flags)` (the axis bitmask)
4. **`frame_tran2`** receives: per bone where flags > 0, one `psmart(delta)` per flagged axis (X first, then Y, then Z)

The `frame_head` stream is prefixed with `p2(total_count)`, `p2(highest_id)` before any per-frame entries.

The `base` field is resolved from name to numeric ID via `base.pack`. If the referenced base does not exist, the packer
panics with an error identifying the frame and the invalid base name.

##### Trailing Bone Omission

A frame may have fewer bones than its base skeleton. The original encoder omits trailing `none` bones to save space.
When packing, only the bones present in the file are written -- missing trailing bones are implicitly `none`.

#### Model Format (.ob2)

Binary files containing 3D mesh data. The last 18 bytes form a metadata trailer:

```
p2(vertex_count), p2(face_count), p1(textured_face_count),
p1(has_info), p1(has_priorities), p1(has_alpha),
p1(has_face_labels), p1(has_vertex_labels),
p2(vertex_x_length), p2(vertex_y_length),
p2(vertex_z_length), p2(face_vertex_length)
```

##### Stream Splitting

The `ob_head` stream receives `p2(count)` followed by one entry per model: `p2(id)` + 10 bytes of metadata (vertex_count
through has_vertex_labels).

The model data preceding the trailer is split across multiple streams in this exact order:

| #  | Stream       | Size                            | Condition                |
|----|--------------|---------------------------------|--------------------------|
| 1  | `ob_point1`  | `vertex_count` bytes            | always                   |
| 2  | `ob_vertex2` | `face_count` bytes              | always                   |
| 3  | `ob_face3`   | `face_count` bytes              | `has_priorities == 255`  |
| 4  | `ob_face5`   | `face_count` bytes              | `has_face_labels == 1`   |
| 5  | `ob_face2`   | `face_count` bytes              | `has_info == 1`          |
| 6  | `ob_point5`  | `vertex_count` bytes            | `has_vertex_labels == 1` |
| 7  | `ob_face4`   | `face_count` bytes              | `has_alpha == 1`         |
| 8  | `ob_vertex1` | `face_vertex_length` bytes      | always                   |
| 9  | `ob_face1`   | `face_count * 2` bytes          | always                   |
| 10 | `ob_axis`    | `textured_face_count * 6` bytes | always                   |
| 11 | `ob_point2`  | `vertex_x_length` bytes         | always                   |
| 12 | `ob_point3`  | `vertex_y_length` bytes         | always                   |
| 13 | `ob_point4`  | `vertex_z_length` bytes         | always                   |

##### Stream Data Descriptions

| Stream       | Content                                                              |
|--------------|----------------------------------------------------------------------|
| `ob_point1`  | Per-vertex flags byte controlling delta encoding for X/Y/Z axes      |
| `ob_point2`  | Vertex X coordinate deltas (variable encoding based on point1 flags) |
| `ob_point3`  | Vertex Y coordinate deltas (variable encoding based on point1 flags) |
| `ob_point4`  | Vertex Z coordinate deltas (variable encoding based on point1 flags) |
| `ob_point5`  | Per-vertex label IDs (for skeletal animation bone binding)           |
| `ob_vertex1` | Face vertex indices (triangle definitions)                           |
| `ob_vertex2` | Per-face encoding type (1=strip, 2=fan, other=new triangle)          |
| `ob_face1`   | Per-face colour/texture reference (2 bytes each)                     |
| `ob_face2`   | Per-face render info flags                                           |
| `ob_face3`   | Per-face render priority                                             |
| `ob_face4`   | Per-face alpha transparency values                                   |
| `ob_face5`   | Per-face label IDs (for skeletal animation bone binding)             |
| `ob_axis`    | Texture axis mapping data (6 bytes per textured face)                |

##### Vertex Delta Encoding

The `ob_point1` flags byte determines how each vertex's X, Y, Z deltas are encoded in `ob_point2`, `ob_point3`,
`ob_point4`:

- If bit N is set (bits 0/1/2 for X/Y/Z): 2 bytes (p2 delta)
- If bit N+3 is set (bits 3/4/5 for X/Y/Z): 1 byte (smart delta)
- Otherwise: 0 bytes (no delta, position unchanged)

##### Face Vertex Encoding

The `ob_vertex2` encoding type determines how many bytes each face consumes in `ob_vertex1`:

- Type 1 (triangle strip continuation): 4 bytes (2 vertex refs, reuses 1 from previous face)
- Type 2 (triangle fan continuation): 4 bytes (2 vertex refs, reuses 1 from previous face)
- All other types (new triangle): 6 bytes (3 vertex refs)

### Model Unpacking

Extracts 3D models (.ob2), animation frames (.frame), and skeleton bases (.base) from the models Jag archive.

#### Input

The `models` Jag archive file (CRC: `-2000991154`), using per-entry bzip2 compression. Contains 21 data streams.

#### Output

```
content_unpack/models/
├── npc/                     -- NPC body and head models
│   ├── model_0_npc_head.ob2
│   └── ...
├── obj/                     -- item and wearable models
│   ├── model_28_obj_wear.ob2
│   └── ...
├── loc/                     -- location models with shape suffixes
│   ├── model_loc_3_3.ob2
│   └── ...
├── spotanim/                -- spot animation models
│   ├── model_411_spotanim.ob2
│   └── ...
├── human/
│   ├── man/                 -- male IDK body and head models
│   │   ├── model_151_idk.ob2
│   │   └── ...
│   └── woman/               -- female IDK body and head models
│       ├── model_103_idk_head.ob2
│       └── ...
├── _unpack/                 -- uncategorized ob2 + bases + frames
│   ├── model_42.ob2         -- models without config references
│   ├── base/
│   │   ├── base_0.base
│   │   └── ...
│   └── frame/
│       ├── anim_0.frame
│       └── ...
└── _raw/                    -- raw Jag entries for CRC verification
    ├── _jag_order.txt
    ├── ob_head.dat
    ├── ob_point1.dat
    └── ...
```

#### OB2 Directory Routing

Each model is placed in a subdirectory based on its config usage (determined during config unpacking):

| Category                | Directory      | Source                                   |
|-------------------------|----------------|------------------------------------------|
| NPC body/head           | `npc/`         | NPC opcode 1, 60                         |
| OBJ inventory/wear/head | `obj/`         | OBJ opcodes 1, 23-26, 78-79, 90-93       |
| LOC shapes              | `loc/`         | LOC opcode 1                             |
| Spot animation          | `spotanim/`    | SPOTANIM opcode 1                        |
| IDK male body/head      | `human/man/`   | IDK opcode 2, 60-69 (type 0-6)           |
| IDK female body/head    | `human/woman/` | IDK opcode 2, 60-69 (type 7-13)          |
| No config reference     | `_unpack/`     | ~365 models not referenced by any config |

#### Base Extraction

Bases define animation skeletons. Each base contains a list of bones, where each bone has a transform type and a set of
vertex group labels it controls. Extracted from three streams: `base_head.dat`, `base_type.dat`, `base_label.dat`.

##### Stream Parsing

```
base_head: p2(count), p2(highest_id)
           then per base: p2(id), p1(bone_count)

base_type: per base: bone_count x p1(transform_type)

base_label: per base, per bone: p1(group_count), group_count x p1(label)
```

The `bone_count` from `base_head` determines how many bytes to read from `base_type`. For `base_label`, each bone reads
a variable-length group: `p1(count)` then `count` label bytes.

##### Bone Types

Each bone has a transform type defined in the `BoneType` enum (`types.rs`):

| Value | Name        | Description                    |
|-------|-------------|--------------------------------|
| 0     | `translate` | Move position (X, Y, Z offset) |
| 1     | `rotate`    | Orientation change             |
| 2     | `scale`     | Resize                         |
| 3     | `alpha`     | Transparency                   |
| 5     | `origin`    | Reference point for transforms |

##### Text File Format

Each `.base` file is written as human-readable text. Each `boneN` line combines the transform type with the vertex group
labels that bone controls:

```
[base_0]
bone0=translate,2
bone1=scale,8,9,10,11,0,5,1,7,2,16,14,19,27,24,28,25,18,12,4,13,6,3,22,20,21,23,26
bone2=translate
bone3=translate,19,24
bone4=rotate,25,18
```

- The first value after `=` is the bone's transform type name.
- The remaining comma-separated values are the vertex group IDs this bone controls.
- A bone with no vertex groups (e.g. `bone2=translate`) is a root/identity transform.

#### Frame Extraction

Frames define a single pose in an animation. Each frame references a base skeleton and provides per-bone transform
deltas. Extracted from four streams: `frame_head.dat`, `frame_tran1.dat`, `frame_tran2.dat`, `frame_del.dat`.

##### Stream Parsing

```
frame_head:  p2(total_count), p2(highest_id)
             then per frame: p2(id), p2(base_id), p1(bone_count)

frame_del:   per frame: p1(delay)

frame_tran1: per frame, per bone: p1(flags)

frame_tran2: per frame, per bone (where flags > 0):
             if flags & 0x1: gsmart(x_delta)
             if flags & 0x2: gsmart(y_delta)
             if flags & 0x4: gsmart(z_delta)
```

The `gsmart` encoding reads 1 byte (value 0-127) or 2 bytes (value >= 128, read as u16 minus 32768), producing unsigned
values in the range 0-32767.

##### Frame Fields

- **delay**: Duration of this frame in client ticks (each tick = 20ms). A delay of 8 means the frame displays for 160ms.
- **base**: Reference to the skeleton this frame animates (e.g. `base_0`). The base must exist in `base.pack` when
  packing.
- **bones**: Each bone specifies which axes are transformed and the delta values.

##### Axis Flags

The flags byte is a bitmask indicating which axes have deltas:

| Flag value | Axes  | Name   |
|------------|-------|--------|
| 0          | --    | `none` |
| 1          | X     | `x`    |
| 2          | Y     | `y`    |
| 3          | X+Y   | `xy`   |
| 4          | Z     | `z`    |
| 5          | X+Z   | `xz`   |
| 6          | Y+Z   | `yz`   |
| 7          | X+Y+Z | `xyz`  |

##### Text File Format

Each `.frame` file is written as human-readable text:

```
[anim_0]
delay=8
base=base_0
bone0=z,48
bone1=none
bone2=none
bone3=none
bone4=y,59
bone10=y,68
bone11=z,48
bone17=x,62
bone18=x,66
```

- `delay` is the frame duration in client ticks.
- `base` references a skeleton by its pack name (e.g. `base_0`).
- Each `boneN` line starts with the axis name, followed by the delta value(s) for each flagged axis.
- `bone1=none` means no transform is applied to that bone in this frame.
- `bone4=y,59` means only the Y axis is transformed, with a delta of 59.
- `bone5=xy,10,20` would mean X delta=10, Y delta=20.
- `bone6=xyz,1,2,3` would mean X=1, Y=2, Z=3.
- A frame may have fewer bones than its base skeleton. Trailing bones with no transform are omitted by the encoder to
  save space.

#### OB2 Extraction

Models are extracted from 14 streams. The `ob_head.dat` stream contains per-model metadata that determines how much data
to read from each stream.

##### Stream Parsing

```
ob_head: p2(count)
         then per model: p2(id), p2(vertex_count), p2(face_count),
         p1(textured_face_count), p1(has_info), p1(has_priorities),
         p1(has_alpha), p1(has_face_labels), p1(has_vertex_labels)
```

##### Stream Consumption Per Model

Data is read from streams in this exact order:

| #  | Stream     | Size                    | Condition              |
|----|------------|-------------------------|------------------------|
| 1  | ob_point1  | vertex_count            | always                 |
| 2  | ob_vertex2 | face_count              | always                 |
| 3  | ob_face3   | face_count              | has_priorities == 255  |
| 4  | ob_face5   | face_count              | has_face_labels == 1   |
| 5  | ob_face2   | face_count              | has_info == 1          |
| 6  | ob_point5  | vertex_count            | has_vertex_labels == 1 |
| 7  | ob_face4   | face_count              | has_alpha == 1         |
| 8  | ob_vertex1 | computed                | always                 |
| 9  | ob_face1   | face_count x 2          | always                 |
| 10 | ob_axis    | textured_face_count x 6 | always                 |
| 11 | ob_point2  | computed                | always                 |
| 12 | ob_point3  | computed                | always                 |
| 13 | ob_point4  | computed                | always                 |

##### Computed Stream Sizes

**ob_vertex1 (face vertex data)**: Size depends on face encoding types in ob_vertex2. For each face, the encoding byte
determines vertex data size:

- Types 1 or 2: 4 bytes per face (triangle strip/fan continuation -- 2 vertex refs)
- All other types: 6 bytes per face (new triangle -- 3 vertex refs)

**ob_point2/3/4 (vertex delta X/Y/Z)**: Size depends on per-vertex flags in ob_point1. For each vertex, the flag byte
determines delta encoding per axis (X=bit 0, Y=bit 1, Z=bit 2):

- If bit N is set: 2 bytes (p2 delta)
- If bit N+3 is set: 1 byte (smart delta)
- Otherwise: 0 bytes (no delta)

##### File Reconstruction

Each `.ob2` file is written as a binary blob:

```
[all stream data concatenated in order][18-byte trailer]
```

The 18-byte trailer contains all the metadata fields from `ob_head`:

```
p2(vertex_count), p2(face_count), p1(textured_face_count),
p1(has_info), p1(has_priorities), p1(has_alpha),
p1(has_face_labels), p1(has_vertex_labels),
p2(vertex_x_length), p2(vertex_y_length),
p2(vertex_z_length), p2(face_vertex_length)
```

#### Generated Pack Files

All `.pack` files are generated with entries in ascending key order (0 through max_id):

| File          | Content                                                                               |
|---------------|---------------------------------------------------------------------------------------|
| `model.pack`  | Context-aware names from config (merged with config-generated names), ascending by ID |
| `model.order` | Model IDs in stream encounter order (used for packing)                                |
| `anim.pack`   | `{id}=anim_{id}` for all IDs 0..max, ascending                                        |
| `anim.order`  | Frame IDs in stream encounter order (used for packing)                                |
| `base.pack`   | `{id}=base_{id}` for all IDs 0..max, ascending                                        |
| `base.order`  | Base IDs in stream encounter order (used for packing)                                 |

The `.order` files preserve the original stream encounter sequence (which may not be sorted), while `.pack` files always
use ascending ID order for readability and consistency.

#### Model CRC Verification

The models Jag roundtrip uses raw stream reassembly (not re-extraction from individual files) for CRC verification. The
`_raw/` directory contains the 21 decompressed Jag entries and their original file table order. These are reassembled
into a new Jag archive and compared against the expected CRC.

---

## Sound, Song, and Jingle

### Sound Packing

This section covers three separate asset types related to audio: synthesized sounds, songs (MIDI music), and jingles (
short MIDI). Each has distinct source formats, packing procedures, and output structures.

#### Sounds Jag Archive

**Original CRC:** -1532605973

The sounds archive contains synthesizer instrument definitions. It is packed into a single Jag archive with one entry:
`sounds.dat`.

##### Source Files

- **Source directory:** `content/synth/` (contains `.synth` files)
- **Registry:** `content/pack/synth.pack` (maps numeric IDs to debug names)
- **Ordering:** `content/pack/synth.order` (numeric IDs, one per line)

##### Binary Format of sounds.dat

The `sounds.dat` file is a flat binary stream consisting of sequential synth entries followed by a terminator:

```
For each synth (in order defined by synth.order):
    p2(id)              -- 2-byte big-endian synth ID
    [raw file bytes]    -- the raw .synth file contents

Terminator:
    p2(0xFFFF)          -- 2-byte sentinel value (65535) marking end of data
```

##### Compression

The sounds Jag archive uses per-entry bzip2 compression.

#### Songs (MIDI Music)

Songs are **not** packed into a Jag archive. They are stored as individual compressed files and served to the client on
demand.

##### Source Files

- **Source directory:** `content/songs/` (contains `.mid` files)

##### Output

A `HashMap<String, Vec<u8>>` where:

- **Key:** truncated filename (see truncation rules below)
- **Value:** bzip2 compressed data with a size header

##### Filename Truncation Rules

The original `.mid` filename is transformed as follows:

1. Remove all dashes (`-`)
2. Replace all dots (`.`) with underscores (`_`)
3. Truncate to a maximum of 12 characters

##### Compression Format

Each file is individually bzip2 compressed. The output format prepends a 4-byte uncompressed size header:

```
[p4(uncompressed_size)]         -- 4-byte big-endian original file size
[bzip2_data_without_BZh1_header] -- bzip2 compressed data with the BZh1 magic header stripped
```

#### Jingles (Short MIDI)

Jingles follow the exact same format as songs but use a different source directory.

##### Source Files

- **Source directory:** `content/jingles/` (contains `.mid` files)

##### Output

Same as songs: a `HashMap<String, Vec<u8>>` with truncated filename keys and individually compressed values.

##### Filename Truncation Rules

Identical to songs:

1. Remove all dashes (`-`)
2. Replace all dots (`.`) with underscores (`_`)
3. Truncate to a maximum of 12 characters

##### Compression Format

Identical to songs:

```
[p4(uncompressed_size)]         -- 4-byte big-endian original file size
[bzip2_data_without_BZh1_header] -- bzip2 compressed data with the BZh1 magic header stripped
```

#### MIDI Duration Parsing

At load time, the cache layer (`rs-pack/src/cache/midi.rs`) decompresses each song and jingle and parses the MIDI file
to extract its duration in milliseconds. This is stored alongside the compressed data in `MidiProvider`, replacing the
need for separate song/jingle HashMaps in `CacheStore`.

The parser handles:

- **RIFF-wrapped MIDIs** (e.g. `attack1.mid`) -- unwraps the RIFF/RMID container to find the inner MIDI data
- **Multi-track formats** 0, 1, and 2 -- tracks max tick across all tracks
- **Tempo changes** (meta event 0x51) -- accumulates microseconds per tempo region
- **SMPTE timing** -- converts frames-per-second + ticks-per-frame directly
- **Variable-length delta encoding** -- standard MIDI VLQ parsing

The duration is converted to game ticks via `ceil(ms / 600) + 1`, exposed as `MidiType::tick_length()`.

The `MidiType` struct retains the original compressed bytes in its `data` field, so the HTTP server can serve them
directly to clients without a separate data store.

#### Sound Packing Summary

| Asset Type | Source Directory   | Output Format            | Archive Type      | Compression            |
|------------|--------------------|--------------------------|-------------------|------------------------|
| Sounds     | `content/synth/`   | Single `sounds.dat` file | Jag archive       | Per-entry bzip2        |
| Songs      | `content/songs/`   | Individual files         | None (standalone) | bzip2 with size header |
| Jingles    | `content/jingles/` | Individual files         | None (standalone) | bzip2 with size header |

### Sound Unpacking

Extracts synthesizer instruments from the sounds Jag archive, and decompresses MIDI music files.

#### Sounds Jag Archive Unpacking

##### Input

The `sounds` Jag archive file (CRC: `-1532605973`), using per-entry bzip2 compression. Contains a single entry:
`sounds.dat`.

##### Output

```
content_unpack/synth/
├── synth_0.synth
├── synth_1.synth
├── synth_2.synth
└── ...

content_unpack/pack/
├── synth.pack           -- ID-to-name mapping
└── synth.order          -- packing order
```

##### sounds.dat Format

The decompressed `sounds.dat` is a flat binary stream:

```
[p2(id)][synth_data]  xN
[p2(0xFFFF)]              -- terminator
```

Each synth entry consists of a 2-byte ID followed by the raw `.synth` file bytes. The terminator `0xFFFF` marks the end.

##### Synth Format Parsing (JagFX / SoundEffect)

Unlike most binary formats, individual synth entries have **no length prefix**. The boundary between entries is
determined by parsing the internal SoundEffect structure.

Each SoundEffect consists of up to 10 tone slots followed by a 4-byte footer:

```
for slot 0..10:
    check = g1()
    if check == 0: empty slot (1 byte consumed)
    else: pos--, Tone.load() (variable bytes consumed)
p2(loopBegin)
p2(loopEnd)
```

##### Tone Format (Instrument)

Each tone reads from the data sequentially:

```
Envelope: frequencyBase     -- required
Envelope: amplitudeBase     -- required

check = g1()
if check != 0: pos--
    Envelope: frequencyModRate
    Envelope: frequencyModRange

check = g1()
if check != 0: pos--
    Envelope: amplitudeModRate
    Envelope: amplitudeModRange

check = g1()
if check != 0: pos--
    Envelope: release
    Envelope: attack

for harmonic 0..10:
    volume = gsmart()
    if volume == 0: break
    semitone = gsmarts()
    delay = gsmart()

reverbDelay = gsmart()
reverbVolume = gsmart()
length = g2()
start = g2()
```

##### Envelope Format

Each envelope reads:

```
form = g1()
start = g4()              -- signed 32-bit
end = g4()                -- signed 32-bit
segmentCount = g1()
for each segment:
    shapeDelta = g2()
    shapePeak = g2()
```

Total: `10 + segmentCount x 4` bytes per envelope.

##### Smart Value Encoding

- `gsmart()`: if byte < 128, read 1 byte (value = byte); else read 2 bytes (value = u16 - 32768)
- `gsmarts()`: if byte < 128, read 1 byte (value = byte - 64); else read 2 bytes (value = u16 - 49152)

##### Implementation

The parser uses `Packet` from `rs-io` directly -- `buf.g1()`, `buf.g2()`, `buf.g4s()`, `buf.gsmart()`, `buf.gsmarts()`,
and `buf.remaining()` handle all reading and position tracking. No manual byte-level position arithmetic is needed.

##### Ordering

Synth IDs in `sounds.dat` are **not monotonically increasing** -- they follow the order defined by `synth.order`, which
can jump (e.g., 0, 1, 2, 259, 260, 261, 3, 4, 5, ...). The parser extracts IDs as they appear in the stream and
generates `synth.order` from the encountered sequence.

##### Pack File

`synth.pack` is generated with entries for all IDs from 0 through the maximum encountered ID, in ascending order:

```
0=synth_0
1=synth_1
2=synth_2
...
434=synth_434
```

#### Songs Unpacking

##### Input

Individual compressed files in `expected/songs/`, named by truncated filename (e.g., `harmony_mid`, `scape_main_m`).

##### Output

```
content_unpack/songs/
├── harmony.mid
├── scape_main.mid
└── ...
```

##### Decompression

Each file has a 4-byte header followed by bzip2-compressed data (BZh1 header stripped):

```
[p4(uncompressed_size)][bzip2_data_without_BZh1]
```

The BZh1 header (`42 5A 68 31`) is prepended before decompression.

##### Filename Recovery

The truncated key is converted back to a `.mid` filename:

- Keys shorter than 12 characters: strip trailing `_mid` suffix and add `.mid` extension (e.g., `harmony_mid` ->
  `harmony.mid`)
- Keys exactly 12 characters: append `.mid` directly (e.g., `scape_main_m` -> `scape_main_m.mid`)

This produces filenames that roundtrip through the packer's truncation rules: remove dashes, replace dots with
underscores, truncate to 12 characters.

#### Jingles Unpacking

Same format and decompression as songs. Source directory: `expected/jingles/` (if present). Currently no expected jingle
files are provided.

---

## Texture

### Texture Packing

The textures Jag archive contains all 3D surface textures used by the game world renderer.

Original CRC: `1703545114`

The archive uses per-entry bzip2 compression.

#### Jag Structure

```
textures (Jag archive)
├── index.dat     -- shared metadata for all textures
├── 0.dat         -- pixel data for texture ID 0 (door)
├── 1.dat         -- pixel data for texture ID 1 (water)
├── 2.dat         -- pixel data for texture ID 2 (wall)
└── ...           -- one .dat per texture, named by numeric ID
```

Unlike the media Jag which uses sprite names, texture entries are named by their **numeric ID** from `texture.pack` (
e.g., `0.dat`, `1.dat`, `49.dat`).

#### Source Directory

```
content/textures/
├── door.tga           -- one indexed sprite-sheet TGA per texture, named by texture name
├── water.tga
├── ...
└── meta/
    ├── index.order    -- numeric IDs in index.dat order
    └── texture.order  -- numeric IDs controlling Jag entry order
```

The textures archive is one indexed sprite sheet per texture, exactly like media. Files are named by texture **name**
(via `texture.pack`); `index.order` lists the numeric **IDs** (each texture's `index.dat` identity) in `index.dat`
order.

#### ID-to-Name Mapping

The pack registry (`content/pack/texture.pack`) maps numeric IDs to texture names, used to resolve each `index.order`
id to its `<name>.tga` sheet:

```
0=door
1=water
2=wall
3=planks
4=elfdoor
...
49=canvas
```

#### Ordering Files

**`index.order`** -- numeric texture IDs in `index.dat` order.

**`texture.order`** -- numeric texture IDs plus `index`, one per line. Controls the order entries appear in the Jag file
table (independent of `index.dat` order).

#### Binary Format

Textures use the same indexed-color binary format as media sprites. See the [media packing](#media-packing) section for
full details on:

- `index.dat` structure (tile dimensions, palette, per-sub-sprite crop metadata)
- `{id}.dat` structure (indexPos + palette indices)
- Pixel order encoding (row-major vs column-major)

#### Source Encoding

Textures use the same indexed sprite sheets as media sprites: each `<name>.tga` carries its grid dims in the image ID,
its palette in the color map, and index 0 is magenta transparency. All per-frame parameters (crop region, pixel order)
are re-derived at pack time. See the [media packing](#media-packing) section for the full rules.

Most textures are single 128x128 images with no cropping, though some are 64x64.

### Texture Unpacking

Extracts all 3D surface textures from the textures Jag archive into one indexed sprite-sheet TGA per texture.

#### Input

The `textures` Jag archive file (CRC: `1703545114`), using per-entry bzip2 compression.

#### Output

```
content_unpack/textures/
├── door.tga             -- one indexed sprite-sheet TGA per texture, named by texture name
├── water.tga
├── ...
└── meta/
    ├── index.order      -- numeric IDs in index.dat order
    └── texture.order    -- Jag file table order (numeric IDs + "index")

content_unpack/pack/
└── texture.pack         -- ID-to-name mapping (e.g., 0=door)
```

#### ID-to-Name Resolution

Texture Jag entries are named by numeric ID (`0.dat`, `1.dat`, ...). A hardcoded name table maps IDs to human-readable
texture names:

| ID | Name      | ID | Name           | ID | Name        |
|----|-----------|----|----------------|----|-------------|
| 0  | door      | 17 | fountain       | 34 | yewtree     |
| 1  | water     | 18 | thatched       | 35 | elfbrick    |
| 2  | wall      | 19 | cargonet       | 36 | elfwall     |
| 3  | planks    | 20 | books          | 37 | chainmail   |
| 4  | elfdoor   | 21 | elfroof2       | 38 | mummy       |
| 5  | darkwood  | 22 | elfwood        | 39 | elfpainting |
| 6  | roof      | 23 | mossybricks    | 40 | jungleleaf4 |
| 7  | damage    | 24 | water_animated | 41 | plant       |
| 8  | leafytree | 25 | gungywater     | 42 | jungleleaf2 |
| 9  | treestump | 26 | web            | 43 | plant2      |
| 10 | leafybase | 27 | elfroof        | 44 | roof2       |
| 11 | mossy     | 28 | mossydamage    | 45 | door2       |
| 12 | railings  | 29 | bamboo         | 46 | pebblefloor |
| 13 | painting1 | 30 | willowtex3     | 47 | rockwall    |
| 14 | painting2 | 31 | lava           | 48 | glyphs      |
| 15 | marble    | 32 | bark           | 49 | canvas      |
| 16 | wood2     | 33 | mapletree      |    |             |

IDs beyond this table fall back to `texture_{id}`. The mapping is written to `pack/texture.pack` and used to name each
sheet file; `index.order` keys stay numeric IDs.

#### Sprite Decoding

Textures use the identical indexed-color binary format as media sprites. See the [media unpacking](#media-unpacking)
section for the full sprite decoding process:

- Index order derived from `indexPos` values in each `.dat` entry (written as `index.order`)
- Palette, crop bounds, and pixel order read from shared `index.dat`
- Written as one `<name>.tga` indexed sprite sheet (palette in the color map) for exact roundtrip

Most textures are single 128x128 images (some 64x64) with no cropping and a single sub-sprite.

---

## Title

### Title Packing

The title Jag archive contains the login screen sprites, font sprites, and the title background image.

Original CRC: `-430779560`

The archive uses per-entry bzip2 compression.

#### Jag Structure

```
title (Jag archive)
├── index.dat        -- shared metadata for all sprites
├── p11.dat          -- font: 11pt proportional
├── p12.dat          -- font: 12pt proportional
├── b12.dat          -- font: 12pt bold
├── q8.dat           -- font: 8pt quill
├── logo.dat         -- RuneScape logo
├── titlebox.dat     -- login dialog box
├── titlebutton.dat  -- login buttons
├── runes.dat        -- animated rune sprites
└── title.dat        -- background image (JPEG, not a sprite)
```

The `title.dat` entry is a raw JPEG file (`title.jpg`), not an indexed-color sprite. All other entries use the same
sprite format as the media archive.

#### Source Directory

The title Jag's fonts and title-screen sprites share the JAG's `index.dat`, so one `index.order` lists both, but their
sheets live in separate directories (fonts in `fonts/`, title sprites in `title/`), plus the binary background:

```
content/fonts/
├── p11.tga            -- font glyph sheets
├── p12.tga
├── b12.tga
└── q8.tga

content/title/
├── logo.tga           -- title screen sprite sheets
├── titlebox.tga
├── titlebutton.tga
├── runes.tga
└── meta/
    ├── index.order    -- all title-Jag group keys in index.dat order (fonts + title)
    └── title.order    -- Jag entry order

content/binary/
└── title.jpg          -- background image (optional)
```

`index.order` lists font groups (`p11`, `p12`, `b12`, `q8`) and title sprites (`logo`, `titlebox`, `titlebutton`,
`runes`) together, in `index.dat` order. On pack, each key is resolved to `fonts/<key>.tga` if present, else
`title/<key>.tga`.

#### Ordering Files

**`index.order`** -- one group key per line, in `index.dat` order; fonts and title sprites together.

**`title.order`** -- controls the order entries appear in the Jag file table, including `index` and `title`:

```
p11
p12
titlebox
title
runes
q8
index
titlebutton
logo
b12
```

Both located in `content/title/meta/`.

#### Binary Format

Title sprites use the same indexed-color binary format as media sprites. See the [media packing](#media-packing) section
for full details on:

- `index.dat` structure (tile dimensions, palette, per-sub-sprite crop metadata)
- `{name}.dat` structure (indexPos + palette indices)
- Source encoding (indexed sprite sheet: palette in color map, grid dims in image ID)

#### Font Sprites

Each font group holds 120 glyph frames on 20x20 canvases (the rev-274+ `_full` fonts hold 256). Each glyph is a
cropped character with 2-color palette (text color + transparent); unmapped glyph slots are blank placeholder frames.
Glyph order follows the RuneScape character set:

- Indices 0-25: uppercase A-Z
- Indices 26-51: lowercase a-z
- Indices 52-61: digits 0-9
- Indices 62+: punctuation and special characters

#### Title Background

The `title.jpg` file from `content/binary/` is included as a raw JPEG in the Jag (entry name `title`). It is not
processed through the sprite encoder. If the file does not exist, the `title` entry is omitted from the Jag.

### Title Unpacking

Extracts the login screen sprites, font sprites, and title background image from the title Jag archive.

#### Input

The `title` Jag archive file (CRC: `-430779560`), using per-entry bzip2 compression.

#### Output

```
content_unpack/fonts/
├── p11.tga             -- font glyph sheets
├── p12.tga
├── b12.tga
└── q8.tga

content_unpack/title/
├── logo.tga            -- title screen sprite sheets
├── titlebox.tga
├── titlebutton.tga
├── runes.tga
└── meta/
    ├── index.order     -- all title-Jag group keys in index.dat order
    └── title.order     -- Jag file table order

content_unpack/binary/
└── title.jpg           -- raw JPEG background image
```

#### Hash-to-Name Resolution

A hardcoded table of known title entry names resolves Jag file hashes:

```
index, logo, p11, p12, b12, q8, runes, title, titlebox, titlebutton
```

#### Entry Routing

Entries are handled by name:

| Entry                              | Destination            | Content                       |
|------------------------------------|------------------------|-------------------------------|
| p11, p12, b12, q8                  | `fonts/<name>.tga`     | Font glyph sheets             |
| logo, titlebox, titlebutton, runes | `title/<name>.tga`     | Title screen sprite sheets    |
| title                              | `binary/title.jpg`     | Raw JPEG (not sprite-decoded) |
| index                              | Sheet image IDs        | Shared sprite metadata        |

Fonts and title sprites share one `index.order` because they share the JAG's `index.dat`, but their sheets are written
to separate directories.

#### Title Background

The `title.dat` entry is a raw JPEG file, not an indexed-color sprite. It is extracted directly to `binary/title.jpg`
without any processing.

#### Sprite Decoding

All non-JPEG entries use the identical indexed-color format as media sprites.
See the [media unpacking](#media-unpacking) section for the full sprite decoding process. Index order is derived from
`indexPos` values in each sprite's `.dat` entry (written as `index.order`).

---

## Word Encoding

### WordEnc Packing

The `wordenc` Jag archive contains chat filter data used for profanity and URL filtering.

- **Original CRC:** `1570981179`
- **Source directory:** `content/wordenc/`
- **Compression:** Per-entry bzip2 (not whole-archive)

The archive is built from 4 text files, each converted into a binary-encoded entry within the Jag.

#### badenc.txt

Contains banned words along with character combination substitutions.

##### Source format

Each line follows the pattern:

```
word combo1 combo2 ...
```

Where each combo is an `a:b` pair representing character substitutions.

##### Binary format

```
p4(line_count)
for each line:
    p1(word_length)
    [word bytes]
    p1(combination_count)
    for each combination:
        p1(a)  // from "a:b" split on ':'
        p1(b)
```

#### fragmentsenc.txt

Contains numeric fragment values used in filter matching.

##### Source format

One numeric value per line.

##### Binary format

```
p4(line_count)
for each line:
    p2(fragment_value)
```

#### tldlist.txt

Contains top-level domain entries with their classification type, used for URL filtering.

##### Source format

Each line follows the pattern:

```
tld type
```

For example: `com 0`

##### Binary format

```
p4(line_count)
for each line:
    p1(tld_type)
    p1(tld_length)
    [tld bytes]
```

#### domainenc.txt

Contains domain strings used for URL/domain filtering.

##### Source format

One domain string per line.

##### Binary format

```
p4(line_count)
for each line:
    p1(domain_length)
    [domain bytes]
```

#### Runtime Filter

The cache layer (`rs-pack/src/cache/wordenc.rs`) decodes all 4 files at load time and provides a
`filter(&str) -> String` method implementing the full RuneScape chat filter pipeline:

1. **Format** -- strip disallowed characters, collapse consecutive spaces
2. **Lowercase** -- convert to lowercase for matching
3. **TLD filter** -- detect top-level domain patterns (`.com`, `.org`) with period/slash context
4. **Bad word filter** -- detect banned words with leet-speak substitution (`@`=`a`, `$`=`s`, `0`=`o`, `1`=`l`, etc.),
   fragment validation, and combo exclusion via binary search
5. **Domain filter** -- detect domain patterns with `@` prefix and `.` suffix detection
6. **Fragment filter** -- detect IP address patterns (4 groups of numbers 0-255)
7. **Whitelist** -- restore known safe words (`cook`, `cook's`, `cooks`, `seeks`, `sheet`)
8. **Uppercase** -- restore original casing for unmasked characters, normalize sentence capitalization

Masked characters are replaced with `*`.

### WordEnc Unpacking

Extracts chat filter data from the wordenc Jag archive back into text files.

#### Input

The `wordenc` Jag archive file (CRC: `1570981179`), using per-entry bzip2 compression. Contains 4 entries.

#### Output

```
content_unpack/wordenc/
├── badenc.txt
├── fragmentsenc.txt
├── tldlist.txt
└── domainenc.txt
```

#### Decoding

Each entry is decompressed from the Jag and decoded from its binary format back to the original text format.

##### badenc.txt

Binary format:

```
p4(line_count)
for each line:
    p1(word_length)
    [word_bytes x word_length]
    p1(combo_count)
    for each combo:
        p1(a)
        p1(b)
```

Text output: `word a:b a:b ...` (one line per entry, combos space-separated as `a:b` pairs).

##### fragmentsenc.txt

Binary format:

```
p4(line_count)
for each line:
    p2(fragment_value)
```

Text output: one numeric value per line.

##### tldlist.txt

Binary format:

```
p4(line_count)
for each line:
    p1(tld_type)
    p1(tld_length)
    [tld_bytes x tld_length]
```

Text output: `tld type` (one line per entry, e.g., `com 0`).

##### domainenc.txt

Binary format:

```
p4(line_count)
for each line:
    p1(domain_length)
    [domain_bytes x domain_length]
```

Text output: one domain string per line.

#### Roundtrip

The text format is identical to the packing source format. Re-encoding these text files through the wordenc packer
produces byte-identical binary data and matching Jag CRC.
