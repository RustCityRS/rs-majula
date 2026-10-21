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

`rs-admin` is a web console for running every world from one place. It works
on phones too.

- **Worlds:** reboot (with a countdown, optionally staggered across worlds,
  and an optional reason players see with it), cancel a reboot, broadcast,
  close a world to new logins (maintenance mode: players already on stay,
  staff can still log in, a restart reopens it; the website's world list
  shows it as MAINTENANCE within about 20 seconds), and each world's tick time,
  memory and connections.
- **Players:** find anyone online, look anyone up (last seen, registered,
  mutes, bans), and kick, mute or ban. A player's page has their
  **timeline**: logins, activity and wealth logs (`session_log`,
  `wealth_event`), chat, moderation, abuse reports against them and by them,
  and recorded sessions, merged newest first and filterable by kind. And
  their **linked accounts**: every address they logged in from (kept 180
  days) and the other accounts that logged in or registered from one, with
  a button to ban them all together. Moderators see addresses as labels
  (`addr-3f9a1c`: the same label is the same address); admins see the
  addresses, and that is audited.
- **Controls:** kill switches that switch a feature off on every world
  (trading, the duel arena, dropping, picking up, shops, banking, or any
  scripts by name pattern): players trying it are told it's switched off.
  And a login lockdown that turns away accounts younger than a set age (for
  a bot wave; staff exempt). Both stay on until turned off: the console
  keeps them (`controls.json`) and sets them again on any world that
  restarts or missed the change.
- **Schedule:** reboots and broadcasts set for later, once or repeating
  (daily, weekly, every N hours), on every world or picked ones. A job due
  while the console was down is skipped (not run late) if it's over 5
  minutes overdue, and the audit log says so.
- **Appeals:** players appeal a ban or mute on the website (`/appeal`,
  linked from Support), signing in with their game username and password
  (wrong passwords count toward the same per-IP lockout as game logins),
  and follow it on `/appeal/status`. The console takes each appeal into a
  case of its own within 30 seconds (it needs `--web-url`), with who issued
  the sanction and why. Whoever issued it can't decide it: the case log
  refuses them. Deciding is Uphold or Overturn with a note the player sees;
  overturning lifts the sanction; either closes the case, which can't be
  closed any other way while the appeal is undecided. One appeal per
  sanction: a decided one can't be appealed again.
- **Staff notes:** private notes on a player's page, apart from cases. A
  note can put them on the watchlist, which the moderation center shows
  with who's online. Kept in `rs-admin/data/notes.jsonl`; their author or
  an admin can remove one, and adding or removing is audited.
- **Sanction templates:** named offences with a ladder of mutes and bans
  (e.g. Macroing: 2-day ban, then 2 weeks, then permanent). Pick one when
  muting or banning: the length comes from how many times the player has had
  it within its window, and the template is logged as the reason. Admins
  edit them in the moderation center.
- **Moderation center:** one place to moderate from. Cases: one per player
  being looked into, which any number of staff work on together (join one,
  or put others on it) until someone closes it as actioned, no action or a
  duplicate. A case holds its evidence (abuse reports, recorded sessions,
  chat messages, copied in so it outlives pruning), a timeline of comments
  and everything done, a status (open, investigating, waiting) and a
  priority. Comments and closing notes are Markdown, with images, GIFs and
  videos attached, pasted or dropped in; kicks, mutes and bans sent from it are noted in it, and it
  refreshes every few seconds and shows who else has it open. The triage
  queue lists abuse reports from the in-game Report Abuse screen and the
  sessions the anti-cheat checks flagged that aren't in a case yet: open a
  case, add to one, or dismiss what needs nothing; a report's "Their
  session" plays the offender's recording from a minute before it. Also who
  is muted or banned now, and every moderation action. Players' pages and
  replays link into it.
- **Finding your way:** Ctrl+K (or `/`) searches players (online, or
  registered by exact name), cases, recordings and recent audit entries, as
  well as pages and actions. Pin players from their page; pinned and
  recently viewed players are on the Players page and open the search.
  "Yours" and "Changed" in the moderation center list your cases and those
  changed by someone else since you last looked (the nav shows a dot). The
  timeline, chat logs and audit log save filters by name, and export what
  they show as CSV (audited; formulas are defused for spreadsheets). Times
  show in your zone or UTC (the button by your name, or Account). Pins,
  recents, saved filters, the zone and what you've read follow your account,
  in `rs-admin/data/prefs.json`.
- **Progress charts**, on a player's page: total XP and wealth (every item,
  bank included, at its value in the cache) at each save kept for
  rollbacks, and when they play: what their scripts logged (`session_log`)
  by weekday and hour over the last 90 days.
- **Staff activity** (admins, in the moderation center): for each staff
  member over 7, 30 or 90 days, their kicks, mutes, bans and lifts, cases
  opened and closed, comments, dismissals and appeals decided; how many
  appeals against their sanctions were overturned (linking the cases); and
  their median pickup (a case opening to their first change on it), time
  to close and time to decide an appeal. For coaching trial moderators.
- **Chat logs:** public chat and private messages, by player or world.
  Turn on Live to see new messages at the top as they're sent. Viewing them
  is audited.
- **Replays:** every session is recorded, and anti-cheat checks score it.
  Watch any session in the game's own client, with the player's mouse drawn
  on top and replayed into the client (menus, hovers and tabs show as they
  did), at up to 8x, with a timeline of what they did ("Attack Goblin",
  "Walk to 3222, 3218"). List only the sessions the checks flagged. A
  session still going on can be watched live: its world then stores it
  every 2 seconds, and the replay plays a few seconds behind it (about 14
  before revision 274, whose client reports input in batches). Watching is
  audited. See [Session replays and anti-cheat](#session-replays-and-anti-cheat).
- **Investigate:** everyone at once.
  - *Live map:* every online player as a dot on the world map (a world's
    own `/livemap` page, framed), coloured for flagged, staff and muted.
    Click a dot to watch live, kick, mute or ban; find a player by name; and
    a list of hotspots, where players stand bunched together.
  - *Anti-cheat:* the players the checks flagged most over the last day,
    week or month, with their highest score in each stretch of it.
  - *Bot farms:* accounts active in the window grouped where they share an
    address, or several of: registering within 15 minutes of each other,
    alike names, the same even click rhythm, the same spot and the same
    hours. Moderators see addresses as labels; admins see them, audited.
- **Player support**, on a player's page:
  - *Session:* why they can't log in, if a world still holds their session
    (they're logging out and their save hasn't reached the database, the
    world kept their save in a local file while its database was down, the
    claim is left over, or the world holding it has stopped), and Free
    session to finish or release it. It never lets go of a player who is
    online or whose newest save isn't in the database; a world the console
    can't reach is only released from once its heartbeat is a minute old,
    as a login elsewhere would take it over.
  - *Send home:* to Lumbridge (or Tutorial Island's start, for a player
    still on it), now or where they next log in (admins). Moderators
    (staff level 1) change nobody's account: no edits, restorations,
    rollbacks or send-home.
  - *Reset password* (admins): a one-time code to give the player, which
    sets a new password at `/recover` on the website within 3 days (wrong
    codes count toward the per-IP login lockout). It can also lock the old
    password at once, logging them out. Only the code's hash is stored, and
    the code is never logged.
  - *Restore items* (admins): give back items lost to a bug, all of them or
    none. Needs an open case and a reason, and is noted on the case. A
    wealth log row in the timeline, or an earlier save, can be the proof:
    the console checks it covers what's restored.
  - *Earlier saves* (admins): a save is kept at logout and every hour
    through a session (at most one per 15 minutes; within that the older
    stays, so there's always one from before a dupe), and just before every
    staff change to a logged-out player, kept 30 days (each player's newest
    5 always). Compare one with now, restore what's gone since, or roll
    back to it (needs an open case; a player online is logged out first,
    and the save replaced is kept, so a rollback can be undone).
- **Economy:** the tools for a dupe or real-world trading. Worlds log
  items passing between players themselves, whatever the content does:
  trades, duel stakes and other handovers (who to), drops and pickups, as
  wealth events of their own beside the scripts' `wealth_event`s, valued at
  the cache's prices.
  - *Circulation:* coins and tracked rares (the holiday rares to start
    with) counted across every save, bank, worn and noted ones included,
    every 15 minutes, and charted over a week, month, quarter or year. A
    count that rises more than its item's threshold (5% unless set) over the
    one before raises an alert, often the first sign of a dupe: it's in the
    sidebar until someone marks it seen, and in the audit log. Admins choose
    what's tracked and the thresholds. Counts are kept 400 days in
    `rs-admin/data/economy-samples.jsonl`; items and alerts in
    `economy.json`.
  - *Who owns:* every account holding an item (noted ones counted in), the
    most first, where they keep it, as of their last save.
  - *Item trace:* where a player's stack came from, back through the
    players it passed: who traded, staked or handed it to them, whose drop
    they picked up (matched to the latest drop of the item on that tile),
    and where it came into the game (a shop, a spawn, alchemy for coins).
    Up to 6 holders back; each player is followed once, and never back
    through staff above you. Timeline rows link to it.
  - *Trade graph:* who a player hands items to and gets them from, with what
    passed between their partners too (rings show as amber arrows), and the
    accounts many players feed that give little back (mules).
  - *Ring rollback* (admins, from a trace or a graph): every account of a
    dupe ring back to its newest save from before one moment, in one action
    with one audit entry, noted on the case for each account. Check shows
    the save each goes back to; nothing is rolled back unless every account
    has one.
  - *Hiscores:* Jagex moderators, maintainers (staff level 2 and up) and
    banned players are never ranked; player moderators are, like players.
    Admins can hide an account from every table, or remove a cheater's
    entries from some (their saves keep the rows current, so putting them
    back is instant); a player's page shows their ranks.
- **Script reload** (maintainers only, on the Worlds page with one world
  picked): recompiles that world's scripts from its content folder and
  swaps them in without a restart, for an emergency fix. Check first: if
  they don't compile, nothing changes and the compiler's errors show.
  Scripts only (configs, maps and interfaces need a restart); scripts
  already running finish on the old code. A reason is required, and checks
  and reloads are audited.
- **Staff** (maintainers only): add accounts, change roles and passwords,
  reset 2FA, sign them out everywhere, disable or remove. Each change needs
  the maintainer's password and a fresh authenticator code. A name opens
  their profile: where they're signed in, their numbers over 7, 30 or 90
  days, and tabs for their audit entries (searchable, exportable), every
  kick, mute and ban by them in game or here, the cases they opened,
  closed or are on, and their sign-ins and failed attempts under their
  name.
- **Stats and items** (maintainers only, on a player's page): set stats, give
  or take items in the backpack, bank or any saved inventory, and move them.
  A logged-in player is changed live by the world they're on; a logged-out
  one has their save changed by any world, which claims their session for
  the moment it takes, exactly as a login would, so they can't log in half
  way through. Every change is audited.
- **News** (maintainers only): write the website's news in Markdown, with
  images, GIFs and videos (add them with the button, or paste or drop them
  into the text), preview it as the site shows it, and post, edit or delete
  it. It goes through the website's admin API (`--web-url`,
  `RS_ADMIN_WEB_URL`; `http://127.0.0.1:8190` in debug builds).

First-time setup, locally:

```bash
cargo world1                                                # a world; its admin API is on 8280
cargo run -p rs-admin -- user add <name> --role maintainer   # a game staff account; asks for a password (12+ characters)
cargo run -p rs-admin                                       # http://127.0.0.1:8150
```

Open http://127.0.0.1:8150, sign in, and scan the QR code with an
authenticator app (Google Authenticator, 1Password, Aegis, ...); the console
asks for its six-digit code at every sign-in after that. Debug builds share
the secret `rs_admin` with debug worlds, so nothing else needs setting.

- It talks to each world's private admin API (`--admin-port`, `8270 +
  node_id`), never the database. Requests are HMAC-signed with
  `RS_ADMIN_SECRET`, which the console and every world must share; worlds
  refuse unsigned, stale or replayed requests. Mutes and bans go through a
  world exactly as the in-game `::mute`/`::ban` do. Lookups (players,
  moderation, chat, abuse reports, timelines, linked accounts) are answered by whichever world is online, from the
  game database; each world keeps 2 database connections for them, apart
  from the game's own.
- `rs-admin/worlds.json` lists the worlds and where their admin APIs are
  (`url`). It isn't `rs-web/worlds.json`: those addresses are public, these
  are private. A world's `client` is its public web address
  (`http://127.0.0.1:8080` locally): replays play on its `/replay` page, so
  without one that world can't show them.
- Accounts live in `rs-admin/data/users.json` (argon2 password hashes, TOTP
  secrets), not the game database; `RS_ADMIN_DATA` (or `--data-dir`, before
  the subcommand) moves the folder. Only game staff get accounts: adding one
  (on the Staff page or with `user add`, which asks the worlds in `--worlds`
  and needs `RS_ADMIN_SECRET`) looks the name up in the game and refuses it
  unless that player's staff level is 1 or more. Every account sets
  up an authenticator app at its first sign-in. Roles match the game's staff
  levels (0 is a player):
  `trial` (1) is a moderator in training: kicks and mutes of up to 2 days,
  but no bans, lifting mutes or bans, sanctioning linked accounts, bulk
  dismissing or deciding appeals (the console refuses them, not just the
  page); `moderator` (1) kicks, mutes, bans, looks players up, reads chat logs,
  and frees stuck sessions, but changes nobody's account; `admin` (2)
  also reboots, broadcasts, toggles maintenance, sends stuck players home,
  resets passwords, restores
  items, rolls saves back (one account or a ring), changes the hiscores and
  what the economy tracks; `maintainer` (3)
  also manages staff accounts, changes players' stats and items, reloads
  scripts, and posts news. Nobody can moderate someone at or above their
  own level, nor see the sessions or logs (recordings, live spectating,
  chat, timeline, login addresses, progress charts, exports, item traces and
  trade graphs) of staff above
  it: lists leave them out, and their player page says so. Manage accounts on the Staff page, or with `rs-admin user
  list|add|passwd|role|reset-2fa|disable|enable|remove` (the way to create
  the first maintainer); changing a password, role or authenticator signs
  that account out everywhere.
- Everything done through it, sign-ins included, goes to
  `rs-admin/data/audit.jsonl` and the `audit` log target. The file is only
  ever appended to, and each entry is synced to disk before it shows; the
  Audit log page pages and searches all of it, back to the first entry (an
  entry's number is its place in the file, so it never changes). It,
  its export, search and Staff activity leave out what staff above you did
  (and names with no account, to all but maintainers), and show the
  address and browser only of your own entries and those of staff below
  you; maintainers see everything. Back the
  file up with the rest of `rs-admin/data/`. Cases are
  `rs-admin/data/cases.jsonl`: every change to one is appended there and the
  cases are rebuilt from it at start, so nothing is ever rewritten. Images
  and videos posted on cases go in `rs-admin/data/media/` (named by their
  content, never deleted), served only to signed-in staff and never through
  the website; back it up with the rest of the data folder.
- Security: sessions are in memory (a restart signs everyone out), 30 minutes
  idle / 12 hours max; `HttpOnly` + `SameSite=Strict` cookies, a CSRF token
  and an Origin check on every change; strict CSP; 5 failed sign-ins lock an
  account for 15 minutes.
- Keep it private: it binds to `127.0.0.1` by default. On the tailnet, serve
  it with `tailscale serve --bg 8150` and start it with `--https true
  --origin https://<host>.<tailnet>.ts.net`. Keep each world's admin API on
  its private address too (`--admin-host`).

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
| Modified client | An `ANTICHEAT_*` packet had the wrong contents (they're fixed per revision family: 225-245.2, 254-289). |
| No game loop | 20+ actions and none of the heartbeats the client's game loop sends (`ANTICHEAT_CYCLELOGIC3/4`, from 254 `4/7`): it isn't running the client. |
| Missing op checks | The client sends `ANTICHEAT_OPLOGIC*` with certain actions once a hidden count passes a threshold; the world mirrors the counts and expects them. |
| Actions without clicks | 254+: over a quarter of 20+ actions came without the click the client reports alongside each. |
| No mouse | 20+ actions and no mouse reports. |
| Cursor jumps | 244+: most of 20+ clicks landed over 40 pixels from the cursor's last report. |
| Clockwork clicks | 30+ clicks at very even intervals. |

Each session's score is the sum of its flags' weights; the measurements
behind them are kept with it. A real client scores 0; a packet bot that only
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
| `--site-origin <ORIGINS>`     | unset (`RS_SITE_ORIGIN`)    | The website's origins, space-separated (e.g. `"https://rustcity.org https://www.rustcity.org"`): the only other sites allowed to frame the client page. Unset, any site may. WebSockets always have to come from a page this world served. |
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
    website runs as its own restricted database role, `/appeal` and
    `/recover` also need, on top of news, hiscores and registration:

    ```sql
    GRANT SELECT (id, display_name, password_hash, muted_until, banned_until, reset_code_hash, reset_code_until) ON players TO rs_web;
    GRANT UPDATE (password_hash, reset_code_hash, reset_code_until) ON players TO rs_web;
    GRANT SELECT, INSERT, UPDATE ON appeals TO rs_web;
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