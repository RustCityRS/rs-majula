<div align="center">
<pre>
██████╗ ██╗   ██╗███████╗████████╗ ██████╗██╗████████╗██╗   ██╗
██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔════╝██║╚══██╔══╝╚██╗ ██╔╝
██████╔╝██║   ██║███████╗   ██║   ██║     ██║   ██║    ╚████╔╝ 
██╔══██╗██║   ██║╚════██║   ██║   ██║     ██║   ██║     ╚██╔╝  
██║  ██║╚██████╔╝███████║   ██║   ╚██████╗██║   ██║      ██║   
╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝    ╚═════╝╚═╝   ╚═╝      ╚═╝   
</pre>
</div>

----

# rs-majula

> **The first fully feature-complete, multi-revision, RuneScape private server engine written in
> Rust** -- and the first private server to build its game cache from source assets
> to CRCs that perfectly match the original Jagex game cache.

> [!IMPORTANT]  
> `rs-majula` is the project, Cargo workspace, and canonical engine name: a
> from-scratch Rust reimplementation of a **RuneScape 2** game server, with
> byte-identical protocol and content emulation, a single-threaded deterministic game
> loop, and an async `tokio` host. The stock client connects and plays against
> unmodified cache content.

----

## Overview

The workspace is 19 crates organized by responsibility:

- **`rs-engine/`** -- See [`rs-engine/README.md`](rs-engine/README.md) for the full technical whitepaper.
- **`rs-pack/`** -- the content/cache compiler (also invoked in-process at boot).
- **`rs-protocol/`** (+ `macros/`) -- the logic-free wire codec.
- **`rs-server/`** -- the async `tokio` binary: bootstrap, sockets, the HTTP
  service that serves the web client, and the terminal dashboard (TUI).
- **`rs-ether/`** -- an Elixir sidecar for cross-world social features
  (**git submodule**). Login is gated on it -- see [requirements](#prerequisites).
- **`content/`, `.keys/`, `public/`** -- runtime assets (cache sources packed at
  boot, the RSA key pair, and the web client), already included in the repo.

----

## Prerequisites

| Tool                    | Version                                 | Why                                                                                                                                                                                                                                                                                                                     |
|-------------------------|-----------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **Rust** (via `rustup`) | stable, **≥ 1.95** (MSRV), edition 2024 | Pinned by `rust-toolchain.toml`, so `rustup` installs the right toolchain automatically. Builds use `-C target-cpu=native` (the binary is tuned to the building machine's CPU).                                                                                                                                         |
| **C linker**            | --                                      | Required by any Rust build: MSVC Build Tools on Windows, `cc`/`clang` on Linux/macOS.                                                                                                                                                                                                                                   |
| **Docker** + Compose    | --                                      | **Required to log in.** Runs Postgres 17 (`docker-compose.yml`); the server authenticates/persists accounts against it. Until Postgres is connected, logins are rejected with *login server offline*.                                                                                                                   |
| **Elixir** + `mix`      | `~> 1.15`                               | **Required to log in.** Runs the `rs-ether` sidecar, which the login flow depends on (cross-world login checks). The server boots and serves the web client without it, but every login attempt returns *login server offline*. Auto-spawned from source with `mix`; production runs a built release instead (see [Deploying](#deploying)). |
| **Git**                 | --                                      | `rs-ether` is a submodule -- clone with `--recursive`.                                                                                                                                                                                                                                                                  |

> [!NOTE]
> **To actually log in you need both Postgres (Docker) up and the ether sidecar
> connected.** If Postgres is down or the sidecar fails to start (e.g. Elixir
> isn't installed), the server still serves the web client, but the client will
> report **"login server offline"** on every login.

----

## Get the source

```bash
git clone --recursive https://github.com/RustCityRS/rs-majula.git
cd rs-majula
# already cloned without submodules?
git submodule update --init --recursive
```

----

## Quick start (single world)

Needs **Rust + Docker + Elixir**. The RSA keys and cache content ship in the
repo, so there is nothing else to generate.

```bash
# 1. Start Postgres (background; data persists in a named volume)
docker compose up -d

# 2. Build & run the server (node 10 = world 1).
#    Debug builds apply schema migrations on boot (`rs-db/migrations/`, and
#    ether's social tables), so Docker must be up first. They also
#    auto-register any new name on first login.
cargo run -p rs-server

# 3. Open the web client and log in
#    http://localhost:8080/rs2.cgi
```

> [!TIP]
> First build compiles the whole workspace and takes a while. For running a
> populated world locally, prefer the `dev-opt` profile (near-release speed,
> still has debuginfo): `cargo run --profile dev-opt -p rs-server`.

> [!TIP]
> For **maximum performance** (production or benchmarking), run in release mode --
> `cargo run --release -p rs-server` -- for full optimizations and fat LTO, at the
> cost of the longest compile. Release builds have no development defaults:
> set `RS_DB_PASS`, `RS_PEPPER` and `RS_COOKIE` first, and run
> `rs-server migrate` once (see [Deploying](#deploying)).

Stop Postgres when you're done with `docker compose down` (add `-v` to also wipe
the database volume).

----

## Running a second world (cluster)

The `cargo world1` / `cargo world2` aliases bring up a two-node cluster on the
`dev-opt` profile (each auto-spawns its own ether sidecar; they mesh via the
shared `--cluster` list):

```bash
docker compose up -d
cargo world1   # --node-id 10, cluster world10+world11
cargo world2   # --node-id 11, in a second terminal -- the cluster meshes
```

See [`rs-ether/README.md`](rs-ether/README.md) for sidecar details.

----

## Running the website

`rs-web` is the game's website: player registration (`/create`), news, the
world select that frames each world's web client, and the archived 2004
pages. It shares the game database (same `--db-*` flags, `RS_DB_*` variables
and defaults) and must use the same `RS_PEPPER` as the game server, or
passwords registered on the site won't verify in game. Like the game server,
debug builds migrate on startup; release builds only check the schema
(`rs-web migrate` applies it).

```bash
docker compose up -d
cargo run -p rs-web   # http://localhost:8000
```

- `rs-web/worlds.json` lists the worlds shown on the world select; each
  `address` is that world's HTTP port (`8070 + node_id`, so `8080` for world 1).
- Player counts are live: each world writes a heartbeat to the `world_status`
  table every ~10 seconds, and the site shows a world as OFFLINE once its
  heartbeat is 30 seconds old (world `N` is node `N + 9`).
- Templates are [minijinja](https://docs.rs/minijinja): `templates/pages/` is
  served by URL (`/faq/billing` -> `pages/faq/billing.html`), `templates/views/`
  is rendered by handlers, `templates/layout/` holds shared partials. Debug
  builds reload edited templates without a restart.
- Flags: `--port` (`8000`), `--root` (`rs-web`), `--public-host`, `--https`,
  and `--real-ip-header` (e.g. `cf-connecting-ip`) when behind a proxy, so the
  per-IP registration limit sees the real client.
- Prometheus metrics (requests by route and status, response times,
  registrations) are on `http://127.0.0.1:8160/metrics`, apart from the public
  port; `--metrics-host`/`--metrics-port` move it and `--metrics-port 0` turns
  it off. `--log-format json` writes JSON lines for journald and Loki;
  `deploy/grafana/web.alloy` ships both from the website's host.
- The staff console posts news through the site's private admin API
  (`127.0.0.1:8190`; `--admin-host`/`--admin-port`, `--admin-port 0` turns it
  off). Its requests are signed with `RS_ADMIN_SECRET`, like the worlds';
  release builds without the secret leave the admin API off.
- News is written in Markdown and stored with the HTML rendered from it.
  Images, GIFs and videos (PNG, JPEG, GIF, WebP, MP4, WebM, up to 50 MB,
  checked by their content) are uploaded through the admin API, kept under
  `--media-dir` (`RS_WEB_MEDIA_DIR`; `rs-web/media` in debug builds, and
  release builds without it take no uploads) named by their content's hash,
  and served at `/news/media/` with a year-long cache and range requests.
  Keep that folder outside the release directory and back it up: it isn't in
  the database.
- Ctrl+C or SIGTERM stops taking connections and finishes open requests
  before exiting.

----

## Running the staff console

`rs-admin` is a web console for running every world from one place, phones
included. It talks to each world's private admin API, never the database, and
only game staff get accounts.

### First-time setup (local)

```bash
cargo world1                                                # a world; its admin API is on 8280
cargo run -p rs-admin -- user add <name> --role maintainer   # asks for a password (12+ characters)
cargo run -p rs-admin                                       # http://127.0.0.1:8150
```

`<name>` must be a player with staff level 1 or more (`user add` asks a world
to check). Open http://127.0.0.1:8150, sign in, and scan the QR code with an
authenticator app. Every sign-in after that asks for its six-digit code. Debug
builds share the secret `rs_admin` with debug worlds, so nothing else needs
setting.

### Roles

Roles match the game's staff levels, and each one adds to the one above it.

| Role | Level | Can |
|---|---|---|
| `trial` | 1 | Kick, and mute for up to 2 days. No bans, lifting sanctions, deciding appeals, bulk dismissing or sanctioning linked accounts. |
| `moderator` | 1 | Kick, mute and ban, look players up, read chat logs, free stuck sessions. Changes nobody's account. |
| `admin` | 2 | Reboot, broadcast, maintenance mode, send players home, reset passwords, restore items, roll back saves, change the hiscores and what the economy tracks, post news. |
| `maintainer` | 3 | Manage staff accounts, change players' stats and items, reload scripts, shut down and start worlds. |

Nobody can moderate staff at or above their own level, or see the recordings,
chat, timeline, addresses or other logs of staff above them.

Manage accounts on the Staff page, or with `rs-admin user
list|add|passwd|role|reset-2fa|disable|enable|remove`. Changing a password,
role or authenticator signs that account out everywhere.

### What's in it

**Worlds**

- Reboot with a countdown (optionally staggered, with a reason players see),
  cancel a reboot, broadcast.
- Maintenance mode closes a world to new logins. Players already on stay,
  staff can still log in, and a restart reopens it.
- Maintainers can shut a world down (it stays off) and start it again.
- **Schedule:** reboots and broadcasts for later, once or repeating. A job
  missed by over 5 minutes while the console was down is skipped, and the
  audit log says so.
- **Controls:** kill switches for trading, the duel arena, dropping, picking
  up, shops, banking or scripts by name, and a login lockdown for accounts
  younger than a set age (staff exempt). They stay on until turned off and are
  set again on any world that restarts.

**Players**

- Look anyone up, then kick, mute or ban. **Sanction templates** pick the
  length from a ladder (e.g. Macroing: 2 days, then 2 weeks, then permanent).
- **Timeline:** logins, activity and wealth logs, chat, moderation, abuse
  reports and sessions, newest first.
- **Linked accounts:** every address they logged in from (kept 180 days) and
  the accounts that share one, with a button to ban them all. Moderators see
  addresses as labels (`addr-3f9a1c`); admins see the real ones, audited.
- **Devices:** each browser or client install they used, numbered D1, D2...,
  with the addresses it was used from.
- **Link flags** warn of things like logins from a banned account's browser,
  headless browsers, or one browser shared by 4+ accounts. The **ban chain**
  follows shared browsers and addresses up to 3 links out and bans them
  together.
- **Staff notes** (which can put a player on the watchlist), **progress
  charts** (XP, wealth and when they play) and **chat logs** (public and
  private, with a Live mode; viewing them is audited).

**Player support**

- **Session:** why they can't log in, and Free session to release a stuck
  one. It never lets go of a player who is online or whose newest save isn't
  in the database.
- **Send home** (admins): to Lumbridge, or Tutorial Island's start.
- **Reset password** (admins): a one-time code the player uses at `/recover`
  on the website within 3 days. Only its hash is stored.
- **Restore items** and **earlier saves** (admins): saves are kept at logout,
  hourly, and before every staff change, for 30 days. Compare, restore or
  roll back. Both need an open case, and a rollback can be undone.

**Moderation center**

- **Cases:** one per player being looked into, worked on by any number of
  staff. A case holds its evidence (copied in so it outlives pruning),
  comments with images and videos, a status and a priority, and shows who
  else has it open.
- **Triage:** abuse reports and flagged sessions that aren't in a case yet.
  Open a case, add to one, or dismiss.
- **Detections:** the console opens cases by itself for anti-cheat scores of
  50+, possible ban evasion, players reported 3 times in a day, and staff
  lifting sanctions on accounts linked to theirs. Staff hear of new ones from
  a toast and a count in the nav and the tab title.
- **Appeals:** players appeal a ban or mute on the website (`/appeal`), and
  each one becomes a case within 30 seconds (needs `--web-url`). Whoever
  issued the sanction can't decide it.
- **Staff activity** (admins): each staff member's actions and response times
  over 7, 30 or 90 days, for coaching trial moderators.
- **Conflicts:** cases and notes about a staff member are hidden from them
  and from staff at or below their level. Staff can't look up their own
  account, and on accounts linked to theirs they can only comment.
  Maintainers aren't held to this, but are detected and audited like anyone.

**Replays and Investigate**

- **Replays:** every session is recorded and scored by anti-cheat checks.
  Watch any of them in the game's own client with the player's mouse, at up
  to 8x, with a timeline of what they did. A session still going can be
  watched live, a few seconds behind. Watching is audited. See [Session
  replays and anti-cheat](#session-replays-and-anti-cheat).
- **Investigate:** a live map of every online player (click one to watch,
  kick, mute or ban), the most-flagged players, ban evasion, and likely bot
  farms grouped by shared addresses, sign-up times, names, click rhythm and
  hours.

**Economy**

- Worlds log every item passing between players (trades, stakes, drops,
  pickups), valued at the cache's prices.
- **Circulation:** coins and tracked rares counted across every save every
  15 minutes and charted. A jump past the item's threshold (5% by default)
  raises an alert, often the first sign of a dupe.
- **Who owns**, **item trace** (where a stack came from, up to 6 holders
  back) and **trade graph** (shows rings and mules).
- **Ring rollback** (admins): rolls a whole dupe ring back to before one
  moment, in one action.
- **Hiscores:** staff level 2+ and banned players are never ranked. Admins
  can hide an account or remove some of its entries.

**Maintainer tools**

- **Script reload:** recompiles one world's scripts and swaps them in without
  a restart. Check first: if they don't compile, nothing changes. Scripts
  only; configs, maps and interfaces need a restart.
- **Staff:** add accounts, change roles and passwords, reset 2FA, sign them
  out everywhere. Each change needs your password and a fresh code. A name
  opens their profile: sessions, numbers, audit entries, sanctions, cases and
  sign-ins.
- **Stats and items:** set stats, give or take items, move players, online or
  offline. Audited.
- **News** (admins too): the website's news in Markdown, with images and
  videos, through its admin API (`--web-url`, `RS_ADMIN_WEB_URL`).

**Finding your way:** Ctrl+K (or `/`) searches players, cases, recordings,
audit entries, pages and actions. Pins, saved filters, CSV export (audited)
and your time zone follow your account.

### How it works

- **Admin API:** each world serves one on `8270 + node_id` (`--admin-port`).
  Requests are HMAC-signed with `RS_ADMIN_SECRET`, which the console and every
  world share. Mutes and bans go through a world exactly like `::mute` and
  `::ban`. Lookups are answered by any online world from the game database.
- **`rs-admin/worlds.json`** lists the worlds and their admin API `url`s
  (private, unlike `rs-web/worlds.json`). A world's `client` is its public web
  address, which replays need. Its `unit` is its systemd unit, which Start
  needs.
- **Data** lives in `rs-admin/data/` (`RS_ADMIN_DATA` or `--data-dir` moves
  it). Back up the whole folder.

  | File | Holds |
  |---|---|
  | `users.json` | Accounts (argon2 password hashes, TOTP secrets) |
  | `audit.jsonl` | Everything done, sign-ins included. Only ever appended to. |
  | `cases.jsonl` | Every change to a case. Only ever appended to; cases are rebuilt from it at start. |
  | `media/` | Images and videos posted on cases, served only to signed-in staff |
  | `notes.jsonl`, `prefs.json`, `controls.json`, `schedule.json`, `economy.json`, `economy-samples.jsonl` | Staff notes, per-account preferences, kill switches, scheduled jobs, economy tracking |

- **Audit log:** the Audit log page, its export, search and Staff activity
  hide what staff above you did, and show addresses only for your own entries
  and those of staff below you. Maintainers see everything.
- **Security:** sessions are in memory (a restart signs everyone out), 30
  minutes idle and 12 hours at most. `HttpOnly` and `SameSite=Strict` cookies,
  a CSRF token and an Origin check on every change, a strict CSP. 5 failed
  sign-ins lock an account for 15 minutes.
- **Keep it private:** it binds to `127.0.0.1` by default. On a tailnet, serve
  it with `tailscale serve --bg --https=8150 http://127.0.0.1:8150` and start
  it with `--https true --origin https://<host>.<tailnet>.ts.net:8150`. Keep
  each world's admin API private too (`--admin-host`).

### Session replays and anti-cheat

Every world records each player's session (`--replay-days`, 7 by default;
`0` records nothing): every packet it sends them, minus the ISAAC cipher,
and every packet their client sends back, in chunks compressed with zstd
by a task with its own database connection, so saves never wait on it
(chunks stored before zstd are deflate, and still play). Recordings go to
`replays` and `replay_chunks` and are pruned once older than the setting.
They're a few kilobytes per minute of play.

**Replaying.** A world serves `/replay`: its own `client.js` with
`replay.js` loaded first. The console frames it and hands it the recording.
The script stands in for the world: it fakes the game connection and answers
the login handshake, pinning `Math.random` while the client picks its ISAAC
seeds so it can cipher the recorded packets the same way, and it runs the
page on a virtual clock, so pause, speed and fast-forward drive the client's
own game loop. Maps and models load from the world as usual. A recording
plays on a world of the revision it was recorded at, with no changes to the
clients: what the script needs from one (its login, scene state, camera and
draw step, connection timeout) it finds in its code by pattern.

**Seeking.** Fast-forwarding runs the client's game loop one tick (20ms) a
step, many steps a task (a few ms of them at a time for the scout, so what's
being watched stays smooth), and skips its drawing except just before a
recorded click (its menus are built as it draws). One tick a step matters:
the client works out how many ticks to run from how long its last steps
took, and any other step leaves what it simulates (walking, the camera)
behind the recording. Seeks match playing the same stretch normally. Every
minute the world records a keyframe: what a client that has just logged in
needs to show what the player's showed (the map and what's on it,
inventories, the players and NPCs in view, and the last of each packet that
sets lasting state, like interface text, varps and stats, plus recent chat).
Going back, or more than 15 seconds forward, logs the client out with the
game's own logout packet and in again from the nearest keyframe, then
fast-forwards from there; no reload. Replays play no music: the client
renders a whole song up front, a heavy load for seconds, run again on the
fast clock. Sound effects still play.

**Buffering.** While it plays, the page keeps a frame every half second of
what the game showed, and a scout (a second, hidden, silent copy of the
replay, `/replay?scout=1`, under the replay and covered by it; out of view,
Chrome would run it once a second) sweeps the whole recording, coarse first
and then down to a frame every 5 seconds (wider on long sessions, at most
~600). Frames are kept up to 160 MB; past that every other one goes, so hours
of play stay covered. Seeking somewhere buffered shows that frame at once
while the client catches up behind it, hovering the timeline previews it (a
spot not buffered yet jumps the scout's queue), and a bar under the timeline
shows what's buffered.

**The dock.** Loading the client takes seconds, so the console keeps one (and
its scout) loaded between replays: started ahead of time on the replays
list and a player's page, and only logged in to a recording when one opens
(switching sessions is a logout and a login). Moving a frame in the page
would reload it, so the dock is fixed on top of the page, laid over the
replay page's stage, and waits out of sight (still in view) elsewhere; it's
let go five minutes after the last replay page. The console also keeps
finished recordings it has downloaded in memory (up to 256 MB) for the rest
of the visit.

**Input.** The player's input comes from the client's own reports: from 254,
`EVENT_MOUSE_MOVE` (the cursor every 50ms), `EVENT_MOUSE_CLICK` and
`EVENT_APPLET_FOCUS`; before 274, `EVENT_TRACKING` (mouse, keys, focus),
which the world turns on at login and asks for every ~10 seconds (input from
the last few seconds before a disconnect is lost). The 225 web client doesn't
report mouse moves, so its replays show the cursor only where it clicked.
The camera follows the client's `EVENT_CAMERA_POSITION` reports (225, 254+);
244 and 245.2 have none, so their replays press the recorded arrow keys.
A minimap click also carries the minimap's tilt and zoom (random, drifting
a step every 10 seconds or so, against screen-reading bots) and the camera's yaw, and so
does 225's camera report. The replay sets the client's to match, so its
minimap looks as the player's did and a replayed minimap click puts the flag
where theirs went; on 244 and 245.2 the yaw also corrects the camera the arrow
keys turn.
Chat, private messages and `::` commands are typed into the client again from
the packets that sent them (274+ report no keys at all), a key at a time and
ending as they were sent, so the client shows them as it did; the timeline
lists them too.

**Free look.** The console's Free look button pauses and gives the viewer
the client: their mouse works its menus, tabs and tooltips and the arrow keys
turn the camera, while the player's cursor is still drawn. The recording
stands still but the client keeps running, so it answers. What the client
sends goes nowhere, and the viewer can't type into it. Turning it off (or
playing) puts the client back as it was: if the viewer clicked or turned
anything, it starts again from the nearest keyframe to the same spot.

**The checks** (flags for review, never acted on):

| Flag | Means |
|---|---|
| Modified client | An `ANTICHEAT_*` packet had the wrong contents (they're fixed per revision family: 225-245.2, 254-289), or a packet was shorter than any the client sends of its kind. |
| Rejected requests | 10+ requests (and a tenth of the actions) the world turned down as ones the client wouldn't make: buttons on interfaces that aren't open or have none, objs in slots that hold something else. Mistyped cheat commands aren't counted. |
| Auto-banned | The world banned it (48 hours) for a packet only a modified client sends: an abuse report with a reason the screen doesn't offer or an offender name that can't exist, or a private message to such a name. The stats name which. |
| No game loop | 20+ actions and none of the heartbeats the client's game loop sends (`ANTICHEAT_CYCLELOGIC3/4`, from 254 `4/7`): it isn't running the client. |
| Forged heartbeats | The game loop sends its two heartbeats a fixed number of cycles apart (`CYCLELOGIC4` every 111 and `3` every 128, on 244-245.2 also `5` every 86; from 254 `7` every 63 and `4` every 193), so however fast the loop runs, their counts keep to that ratio. More than 3 (and 6%) off the count the rarer one's makes the other's, once there are 15 of the rarer: something else is sending them. A real client's are within a count or two. |
| Never drew | 254+: 20+ actions and 30 `CYCLELOGIC4`s (about 2 minutes) without one `CYCLELOGIC1`, which the client sends from drawing the game every 1175 frames. |
| Missing op checks | The client sends `ANTICHEAT_OPLOGIC*` with certain actions once a hidden count passes a threshold; the world mirrors the counts and expects them. |
| Actions without clicks | 254+: over a quarter of 20+ actions came without the click the client reports alongside each. |
| Actions while unfocused | 3+ actions while the client said its window had lost focus (not on touch screens, whose keyboard takes focus). A real click gives the window focus first; a script's made-up click doesn't. Before 254 the click's press is judged in the input reports. |
| Clicked what it wasn't shown | Actions in 3+ separate stretches (including "use" and spells) on an NPC or player the world wasn't sending the client, a loc outside its map, or an obj in a zone it wasn't sent since the map was built. Something that left view or a map that moved on still counts as shown for 10 ticks, for clicks made before the update arrived, and the actions within 10 ticks of the first count once: one late update (a stalled connection) makes several. |
| Menu options without right-click | Most of 5+ actions on options a left click doesn't pick (worked out from the types as the client builds its menu) came with no right press in the input reports to open the menu. Not for players' options, one-button mouse mode or touch screens. |
| No mouse | 20+ actions and no mouse reports. |
| Cursor jumps | 244+: most of 20+ clicks landed over 40 pixels from the cursor's last report. |
| Clockwork clicks | 30+ clicks at very even intervals. |
| Forged minimap clicks | A minimap click ends in 14 bytes: the click's spot on the minimap, the camera's yaw, the minimap's random tilt and zoom, where the client drew the player, and the fixed values 57, 89 and 63. Two or more with wrong fixed values, values out of range, or a spot that (worked out as the client does) doesn't lead to the tile walked to. |
| Minimap position off | Most of 5+ minimap clicks drew the player over 8 tiles from where the world has them. |
| Instant reactions | Half of 20+ reactions came faster than 180ms, network and all. A reaction is the first action after what the player was acting on went (a rock ran out, a tree fell, an NPC died) or after an obj they then took appeared, timed from when the world sent it to when the action reached it. Only counted after a second with no actions, so clicking away all the while isn't a reaction. |
| Even reactions | 20+ reactions that hardly vary (standard deviation under 0.15 of the mean). People have a long tail. |
| Long days | 16+ hours played on 3+ of the last 7 days (UTC), from the account's recorded sessions. |
| No long breaks | Played 3 days back and never stopped for 4 hours since. |
| Instant relogs | 4 in 5 of 5+ returns this week came within 15 seconds. A return is a login after a session the player didn't end: a lost connection, an idle logout, a kick or ban, or a reboot. It's timed from when the world let go of them (a lost connection keeps them in a while, and the client reconnects itself meanwhile), or from the world coming back after a reboot. Logouts are left out, as world hops are back in seconds. |
| Repeated laps | Of 150+ recent actions, a fifth of them walks, 85%+ of the runs of 4 were seen before: the same tiles, booth and order every lap. People's walks land a tile or two off. |
| Same pixel clicks | Half of 20+ aimed clicks on inventory slots hit the same pixel of the slot. A click only counts after the cursor moved: clicking a slot again where it already is (withdrawing one at a time) isn't aiming. Slots in a layer that scrolls (the bank) aren't judged. |
| Even slot clicks | 50+ clicks on inventory slots spread over the whole slot (8.3+ pixels from their mean; evenly over a slot is 9.2) instead of clustering on the obj in the middle. Not for touch screens. |
| Straight mouse moves | 254+: 60% of 30+ moves (5+ samples, 40+ pixels) went in a dead straight line. |
| Steady mouse moves | 254+: 60% of 30+ moves no faster through their middle third than 1.3 times their ends. People speed up and slow down, about twice as fast midway. |
| Repeated mouse curves | 254+: 40% of 20+ curved moves followed the shape of an earlier one. |
| Anti-AFK | An hour in all of stretches of 15+ minutes with no actions, clicks or chat, no idle report from the client, and nothing but tiny mouse nudges or one or two keys at even intervals (or, from 274, where keys aren't reported, nothing at all). |
| Camera never turned | 225, 254+: 3+ hours and 300+ actions without the camera turning once. |

Each session's score is the sum of its flags' weights; the measurements
behind them are kept with it. The checks on behaviour over time (from
Instant reactions on) weigh 15 to 30 each, so none opens a case alone. A real client scores 0; a packet bot that only
sends walks scores 110 (no game loop, actions without clicks, no mouse).

----

## Command-line arguments

All configuration is via CLI flags (clap); the database, secret and cluster
settings can also come from the environment variable shown. This is the
complete set -- run `cargo run -p rs-server -- --help` for the canonical
output. `rs-server migrate` applies pending migrations and exits (flags go
before `migrate`).

| Argument                      | Default                     | Description                                                                                                    |
|-------------------------------|-----------------------------|----------------------------------------------------------------------------------------------------------------|
| `--host <HOST>`               | `0.0.0.0`                   | Bind address for the TCP game + HTTP listeners.                                                                |
| `--http-port <HTTP_PORT>`     | `8070 + node_id` (`8080`)   | HTTP port -- web client + cache archives.                                                                      |
| `--tcp-port <TCP_PORT>`       | `43584 + node_id` (`43594`) | TCP game-protocol port.                                                                                        |
| `--private-key <PRIVATE_KEY>` | `.keys/private.pem`         | RSA private key (PEM) for the login handshake.                                                                 |
| `--members <BOOL>`            | `true`                      | Members world (vs free-to-play) content rules. Takes a value: `--members false` runs a free-to-play world.     |
| `--client-pathfinder`         | `true`                      | Trust client-computed movement paths.                                                                          |
| `--no-tui`                    | `false`                     | Disable the TUI dashboard; use stdout logging (auto-falls back when stdout isn't a TTY).                       |
| `--verify`                    | `true`                      | Verify packed-cache byte-identity at boot (`pack_all`).                                                        |
| `--node-id <NODE_ID>`         | `10`                        | World node ID (10 = world 1, 11 = world 2, …); drives the derived port scheme.                                 |
| `--real-ip-header <NAME>`     | unset                       | Header a reverse proxy puts the client IP in (e.g. `x-real-ip`). Per-IP connection limits and login rate limiting then use it instead of the proxy's address. Only set it when the HTTP port is reachable through the proxy alone, or clients can forge their address. |
| `--site-origin <ORIGINS>`     | unset (`RS_SITE_ORIGIN`)    | The website's origins, space-separated (e.g. `"https://rustcity.org https://www.rustcity.org"`): the only other sites allowed to frame the client page, and the only ones its `/uid` page tells the browser's game id (for the website's account recovery form). Unset, any site may frame it, and `/uid` tells nobody. WebSockets always have to come from a page this world served. |
| `--ether-port <ETHER_PORT>`   | `5000 + node_id` (`5010`)   | Ether sidecar TCP port.                                                                                        |
| `--db-host <DB_HOST>`         | `localhost` (`RS_DB_HOST`)  | Postgres hostname.                                                                                             |
| `--db-port <DB_PORT>`         | `5432` (`RS_DB_PORT`)       | Postgres port.                                                                                                 |
| `--db-name <DB_NAME>`         | `postgres` (`RS_DB_NAME`)   | Postgres database name.                                                                                        |
| `--db-user <DB_USER>`         | `postgres` (`RS_DB_USER`)   | Postgres username.                                                                                             |
| `--db-pass <DB_PASS>`         | `RS_DB_PASS`; `password` in debug only | Postgres password. Required in release builds.                                                      |
| `--pepper <PEPPER>`           | `RS_PEPPER`; `localhost` in debug only | Server-side pepper for password hashing. Required in release builds.                                |
| `--cookie <COOKIE>`           | `RS_COOKIE`; `rs_secret` in debug only | Erlang cookie shared by every ether node in the cluster. Required in release builds.                |
| `--admin-port <ADMIN_PORT>`   | `8270 + node_id` (`8280`)   | Private admin API the staff console (`rs-admin`) calls. `0` turns it off.                                     |
| `--admin-host <ADMIN_HOST>`   | `127.0.0.1`                 | Bind address for the admin API. Keep it private (loopback or the Tailscale address).                          |
| `--admin-secret <SECRET>`     | `RS_ADMIN_SECRET`; `rs_admin` in debug only | Shared with `rs-admin`, which signs every request with it. Required in release builds.        |
| `--node-host <NODE_HOST>`     | `127.0.0.1` (`RS_NODE_HOST`) | Host part of this world's ether node name (`world<id>@<host>`); other worlds must be able to reach it.       |
| `--cluster <CLUSTER>`         | `""` (`RS_CLUSTER_HOSTS`)   | Comma-separated ether node list (e.g. `world10@10.0.0.5,world11@10.0.0.6`). Empty means `world10`..`world20` on `--node-host`. |
| `--ether-release <PATH>`      | unset (`RS_ETHER_RELEASE`)  | A built ether release (`rs-ether/_build/prod/rel/rs_ether`) to run instead of `mix` from source.               |
| `--auto-register <BOOL>`      | `true` (debug) / `false` (release) | Register an unknown name on first login (debug builds also grant staff level 3). Off in release: players register on the website. Takes a value, e.g. `--auto-register false`. |
| `--auto-migrate <BOOL>`       | `true` (debug) / `false` (release) | Apply pending migrations on startup. Off, the server only checks the schema is current and keeps logins offline until `rs-server migrate` has run. |
| `--replay-days <DAYS>`        | `7` (`RS_REPLAY_DAYS`)      | Days to keep session recordings for the staff console's replays. `0` records nothing. |

> [!NOTE]
> `--client-pathfinder`, `--no-tui`, and `--verify` are boolean flags
(defaults shown). The `--db-*` defaults match `docker-compose.yml`, so the server
> connects with no extra flags.

----

## Deploying

One website, many worlds, one shared Postgres. Each world runs its own ether
node; the nodes mesh into one cluster for cross-world social features.

1. **Secrets come from the environment.** Set `RS_DB_PASS`, `RS_PEPPER`,
   `RS_COOKIE` and `RS_ADMIN_SECRET` (the website needs the first two, the
   staff console only the last). Release builds refuse to
   start without them. Flags would also work, but anyone who can list
   processes can read them.
2. **Build ether once.** In `rs-ether/`: `MIX_ENV=prod mix deps.get` then
   `MIX_ENV=prod mix release`, and point each world at the result with
   `RS_ETHER_RELEASE=rs-ether/_build/prod/rel/rs_ether`. The release needs no
   Elixir install on the world host, only a matching OS.
3. **Migrate as its own deploy step**, before rolling out new binaries:
   `rs-server migrate` (game tables, then ether's). Release servers never
   migrate on their own; they check the schema and keep logins offline until
   it's current. They tolerate migrations from a *newer* build, so old worlds
   keep running while you roll out, as long as each migration is backward
   compatible.
4. **Patch every release you deploy.** The repo's `.keys/private.pem`
   is public. Copy `deploy/patch.py` to the server, and in each extracted
   release, before starting the world, run `python3 /path/to/patch.py`
   (needs `openssl`). It writes a new key and patches
   `public/<rev>/client.js` to match. Run it on the server only: the key must
   never be committed or uploaded. The Java client keeps the old key and can't
   log in afterwards. It also patches the web client's login uid (a constant
   1337 out of the build) into a random id per browser, which the worlds log
   with each login so the staff console links accounts played from the same
   browser, as it does by address.
5. **Name nodes by reachable hosts.** Give each world `--node-host` (its
   private IP or hostname) and every world the same `--cluster` list, e.g.
   `world10@10.0.0.5,world11@10.0.0.6`.
6. **Keep the cluster private.** Anyone holding the cookie who can reach a
   node can run code on it. Firewall EPMD (`4369`) and the distribution ports
   to the private network. To pin those ports, set
   `ERL_AFLAGS="-kernel inet_dist_listen_min 9100 inet_dist_listen_max 9110"`
   for the world process; the sidecar inherits it.
7. **TLS at a reverse proxy.** The web client connects back to the page's own
   host over `ws`/`wss`, so terminate TLS for the website and for each world's
   HTTP port. Behind the proxy every connection comes from the proxy's own
   address, so have it set the client IP in a header (nginx:
   `proxy_set_header X-Real-IP $remote_addr;`), start the world with
   `--real-ip-header x-real-ip`, and bind it where only the proxy can reach it
   (e.g. `--host 127.0.0.1`).
8. **Run one staff console, privately.** Put `rs-admin` on a host that can
   reach every world's admin API over the private network (on the worlds'
   host, they can stay on `127.0.0.1`), with the same `RS_ADMIN_SECRET` and
   `RS_ADMIN_DATA` on persistent disk. List the worlds' private admin URLs in
   a worlds file passed with `--worlds`. Serve it over HTTPS on the private
   network only (e.g. `tailscale serve --bg 8150`), start it with
   `--https true --origin https://<that host>`, and create the first account
   with `rs-admin user add <name> --role maintainer` (in the console's
   environment and with its `--worlds`, since it asks a world whether they're
   game staff, so with the worlds up). Never expose it or an
   admin API to the internet. Worlds restart after a console reboot only if
   their service manager restarts them (systemd `Restart=always`). For news,
   give `rs-web` the same `RS_ADMIN_SECRET` and an admin port the console can
   reach privately (`--admin-host`, `--admin-port`), point the console at it
   with `RS_ADMIN_WEB_URL`, give it a `--media-dir` outside the release
   folder for news images and videos, and let the website's database login
   insert, update and delete `news_posts`. For replays, give each world in
   the console's worlds file its public `client` address (e.g.
   `https://w1.example.com`); the console's browser loads `/replay` from it.
9. **Keep recordings out of routine backups.** Session recordings are bulky
   and short-lived: dump with `pg_dump --exclude-table-data=replay_chunks`.
10. **Grant the website's login what appeals and recovery need.** If the
    website runs as its own restricted database role, `/appeal`,
    `/recover` and `/recover/request` also need, on top of news, hiscores
    and registration:

    ```sql
    GRANT SELECT (id, display_name, password_hash, muted_until, banned_until, reset_code_hash, reset_code_until) ON players TO rs_web;
    GRANT UPDATE (password_hash, reset_code_hash, reset_code_until) ON players TO rs_web;
    GRANT SELECT, INSERT, UPDATE ON appeals TO rs_web;
    GRANT SELECT, INSERT, UPDATE ON recovery_requests TO rs_web;
    GRANT SELECT (id, moderator_id, target_id, action, hours, reason, created_at) ON moderation_logs TO rs_web;
    GRANT SELECT, INSERT ON login_failures TO rs_web;
    ```

----

## Build profiles

Defined in `.cargo/config.toml`:

- **`dev`** (default) -- unoptimized, full debuginfo; fast compiles, enables the
  debug-only hot-reload.
- **`dev-opt`** -- `inherits = dev` with `opt-level = 2`; the practical profile for
  running a real world locally. `cargo run --profile dev-opt -p rs-server`.
- **`release`** -- **maximum performance**: `opt-level=3`, fat LTO, `panic = "unwind"`
  (load-bearing for the engine's `catch_unwind` recovery). Slowest to compile; use it
  for production and benchmarking. `cargo run --release -p rs-server`.

----

## How to Play

There are two ways to play the server.

### 1. The original shipped Java client

Depending on your targeted revision, navigate to the respective `/public/{REV}/client.jar` file.
Run the following command:

```bash
java -cp client.jar client 10 0 highmem members 10
```

> [!NOTE]
> Usage: node-id, port-offset, [lowmem/highmem], [free/members], storeid

### 2. The ported JavaScript browser client

Navigate to the following address on any modern web browser:

http://localhost:8080/

> [!NOTE]
> Any browser should be supported as long as it supports WebAssembly. This includes any mobile browser as well.

----

## Multi-Revision

The engine supports multiple revision targets. The ones listed below can be targeted to your choice.
Simply change the `REV` located in `/.cargo/config.toml` and rebuild.

```toml
[env]
REV = "289"
```

### 225 (2004-05-18)

- https://runescape.wiki/w/Update:Big_Chompy_Bird_Hunting

### 244 (2004-06-28)

- https://runescape.wiki/w/Update:New_game_update_system
- https://runescape.wiki/w/Update:Agility_improved_and_bug_fixes
- https://runescape.wiki/w/Update:New_In-Game_Player_Moderators
- https://runescape.wiki/w/Update:Special_Attacks
- https://runescape.wiki/w/Update:Various_tweaks_to_the_game
- https://runescape.wiki/w/Update:Lots_more_improvements

### 245.2 (2004-07-13)

- https://runescape.wiki/w/Update:Easier_to_rearrange_bank
- https://runescape.wiki/w/Update:Priest_In_Peril_Quest
- https://runescape.wiki/w/Update:Nature_Spirit_Quest

### 254 (2004-09-07)

- https://runescape.wiki/w/Update:Quest_Journals_and_Chompy_Hats
- https://runescape.wiki/w/Update:Agility,_Potions_and_Parties!
- https://runescape.wiki/w/Update:New_Random_Events!
- https://runescape.wiki/w/Update:Death_Plateau_Quest
- https://runescape.wiki/w/Update:Troll_Stronghold_Quest
- https://runescape.wiki/w/Update:Herblore_Additions
- https://runescape.wiki/w/Update:Situation_in_the_Sands

### 274 (2004-11-24)

- https://runescape.wiki/w/Update:Tai_Bwo_Wannai_Trio
- https://runescape.wiki/w/Update:Plague_City_Part_4_Released
- https://runescape.wiki/w/Update:Skills,_Duels_and_the_Kalphite
- https://runescape.wiki/w/Update:Eadgars_Ruse
- https://runescape.wiki/w/Update:Morytania_Expansion
- https://runescape.wiki/w/Update:Mortton_Shades_and_Mage_Armour
- https://runescape.wiki/w/Update:Mage_Armour_Updated
- https://runescape.wiki/w/Update:Treasure_Trails_and_Changes
- https://runescape.wiki/w/Update:The_Fremennik_Trials
- https://runescape.wiki/w/Update:Horror_From_The_Deep
- https://runescape.wiki/w/Update:Burthorpe_Games_Room

### 289 (2005-01-17)

- https://runescape.wiki/w/Update:Throne_Of_Miscellania
- https://runescape.wiki/w/Update:Monkey_Madness
- https://runescape.wiki/w/Update:Various_Small_Changes.
- https://runescape.wiki/w/Update:More_Small_Changes
- https://runescape.wiki/w/Update:Castle_Wars
- https://runescape.wiki/w/Update:Changes_to_Castle_Wars
- https://runescape.wiki/w/Update:Santa,_Flax_and_Castlewars
- https://runescape.wiki/w/Update:The_Haunted_Mine
- https://runescape.wiki/w/Update:Troll_Romance,_Banks_and_Chat
- https://runescape.wiki/w/Update:In_Search_Of_The_Myreque
- https://runescape.wiki/w/Update:Trawler_Game_Update
- https://runescape.wiki/w/Update:Karamja_Dungeon

----

## Contributing

All are welcome to help contribute to the project. I especially am always looking for performance opportunities
that could be had, whether CPU or RAM optimizations.

- Please provide benchmarks of a before & after if you open a PR for any performance improvements.
- Please provide sources of any additional engine or content changes you open a PR for.
- I am using a Windows machine and am unable to validate the project under Linux or MacOS myself, relying on Github
  actions.

> [!TIP]
> When you open or push to a PR, a Github action will run jobs to validate that all revisions build,
> compile content and test. You do not have to manually validate every supported revision yourself.

----

*Bearer of the curse, seek misery.*