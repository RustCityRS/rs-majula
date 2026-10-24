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

> **The first fully feature-complete, multi-revision RuneScape private server engine written in Rust.**
> It's also the first private server to build its game cache from source assets, with CRCs that
> match the original Jagex cache exactly.

`rs-majula` is a from-scratch Rust rewrite of a **RuneScape 2** game server. The protocol and
content match the original byte for byte, the game loop is single-threaded and deterministic,
and an async `tokio` host runs everything around it. The original client connects and plays
against unmodified cache content.

**Contents**

- [What's in the repo](#whats-in-the-repo)
- [What you need](#what-you-need)
- [Quick start](#quick-start)
- [Playing](#playing)
- [The website](#the-website)
- [The staff console](#the-staff-console)
- [Session replays and anti-cheat](#session-replays-and-anti-cheat)
- [Server options](#server-options)
- [Build profiles](#build-profiles)
- [Supported revisions](#supported-revisions)
- [Deploying](#deploying)
- [Contributing](#contributing)

----

## What's in the repo

| Folder | What it is |
|---|---|
| [`rs-engine/`](rs-engine/README.md) | The game engine and its sub-crates. Its README is the full technical write-up. |
| [`rs-pack/`](rs-pack/README.md) | The content and cache compiler. The server also runs it at startup. |
| `rs-protocol/` | The wire codec, with no game logic in it. |
| `rs-server/` | The game server: startup, sockets, the web client's HTTP server and the terminal dashboard. |
| `rs-db/` | The database layer and its migrations. |
| `rs-web/` | The website. |
| `rs-console/` | The staff console, plus `api/`, the admin API it shares with the worlds and the website. |
| `rs-ver/` | Picks the game revision at build time. |
| [`rs-ether/`](rs-ether/README.md) | An Elixir sidecar for cross-world social features. It's a **git submodule**, and logins need it. |
| `content/`, `.keys/`, `public/` | Game content (packed at startup), the RSA key pair and the web client. All included. |

----

## What you need

| Tool | Version | Why |
|---|---|---|
| **Rust** (via `rustup`) | stable, **1.95 or newer**, edition 2024 | `rust-toolchain.toml` pins it, so `rustup` installs the right one for you. Builds use `-C target-cpu=native`, so a binary is tuned to the machine that built it. |
| **A C linker** | any | Every Rust build needs one: MSVC Build Tools on Windows, `cc` or `clang` on Linux and macOS. |
| **Docker** with Compose | any | **Needed to log in.** It runs Postgres 17 (`docker-compose.yml`), where accounts live. |
| **Elixir** with `mix` | `~> 1.15` | **Needed to log in.** It runs the `rs-ether` sidecar, which checks logins across worlds. The server starts it from source for you. |
| **Git** | any | `rs-ether` is a submodule, so clone with `--recursive`. |

> [!NOTE]
> Without Postgres or the ether sidecar, the server still starts and serves the web client,
> but every login says **"login server offline"**.

----

## Quick start

Get the code:

```bash
git clone --recursive https://github.com/RustCityRS/rs-majula.git
cd rs-majula

# cloned without --recursive? fetch the submodule now
git submodule update --init --recursive
```

Run one world:

```bash
docker compose up -d       # start Postgres (its data survives restarts)
cargo run -p rs-server     # build and run world 1 (node 10)
```

Then open http://localhost:8080/rs2.cgi and log in with any name. Debug builds create the
account on first login and set up the database tables when they start, which is why Postgres
has to be up first. The RSA keys and game content are already in the repo, so there's nothing
to generate.

When you're done, `docker compose down` stops Postgres. Add `-v` to wipe the database too.

> [!TIP]
> The first build compiles everything and takes a while. For a busy world on your own machine,
> `cargo run --profile dev-opt -p rs-server` runs close to release speed and keeps debug info.

> [!TIP]
> `cargo run --release -p rs-server` is the fastest build, for production and benchmarks, but
> takes longest to compile. Release builds have no development defaults: set `RS_DB_PASS`,
> `RS_PEPPER` and `RS_COOKIE`, and run `rs-server migrate` once (see [Deploying](#deploying)).

### Two worlds at once

The `cargo world1` and `cargo world2` shortcuts start a two-world cluster on the `dev-opt`
profile. Each starts its own ether sidecar, and they find each other through the shared
`--cluster` list:

```bash
docker compose up -d
cargo world1   # node 10, in one terminal
cargo world2   # node 11, in a second terminal
```

See [`rs-ether/README.md`](rs-ether/README.md) for more on the sidecar.

----

## Playing

**In a browser.** Open http://localhost:8080/ in any browser with WebAssembly, phones
included.

**With the original Java client.** Find `public/<REV>/client.jar` for your revision and run:

```bash
java -cp client.jar client 10 0 highmem members 10
```

The arguments are: node id, port offset, `lowmem` or `highmem`, `free` or `members`, store id.

----

## The website

`rs-web` is the game's website: registration, news, the world list that opens each world's
web client, and the archived pages from 2004 and 2005. It uses the game's database, with the
same `--db-*` flags, `RS_DB_*` variables and defaults.

```bash
docker compose up -d
cargo run -p rs-web   # http://localhost:8000
```

> [!IMPORTANT]
> The website and the game must share one `RS_PEPPER`, or passwords made on the site won't
> work in game.

Like the game server, debug builds set up the database when they start. Release builds only
check it, and `rs-web migrate` applies any changes.

### Settings

| Flag | Default | What it does |
|---|---|---|
| `--port` | `8000` | The public port. |
| `--root` | `rs-web` | Where the templates, `public/` and `worlds.json` are. |
| `--public-host` | `localhost` | The site's hostname, for links it gives out. |
| `--https` | `false` | Set when the site is served over HTTPS. |
| `--real-ip-header` | unset | The header a proxy puts the visitor's address in (e.g. `cf-connecting-ip`), so per-address limits see the visitor, not the proxy. |
| `--metrics-host`, `--metrics-port` | `127.0.0.1`, `8160` | Prometheus metrics (requests, response times, registrations), away from the public port. Port `0` turns them off. |
| `--admin-host`, `--admin-port` | `127.0.0.1`, `8190` | The private admin API the staff console uses for news and messages. Port `0` turns it off. |
| `--media-dir` | `rs-web/media` in debug builds (`RS_WEB_MEDIA_DIR`) | Where news images and videos go. Release builds without it take no uploads. |
| `--log-format` | `text` | `json` writes JSON lines for journald and Loki. `deploy/grafana/web.alloy` ships those and the metrics. |

### How it works

- **Worlds.** `rs-web/worlds.json` lists the worlds on the world list. Each `address` is that
  world's HTTP port: `8070 + node_id`, so `8080` for world 1.
- **Player counts are live.** Each world writes a heartbeat to the `world_status` table about
  every 10 seconds. A world shows as OFFLINE once its heartbeat is 30 seconds old. World `N`
  is node `N + 9`.
- **Templates** use [minijinja](https://docs.rs/minijinja).
  - `templates/pages/` is served by URL, so `/faq/billing` shows `pages/faq/billing.html`.
  - `templates/views/` is rendered by handlers with data.
  - `templates/layout/` holds the shared pieces.
  - `templates/since_289/` holds the pages that look different from revision 289 on. A 289+
    build looks there first.

  Debug builds pick up template edits without a restart.
- **News** is written in Markdown and stored along with its HTML. Images, GIFs and videos
  (PNG, JPEG, GIF, WebP, MP4, WebM, up to 50 MB, checked by their content) come in through the
  admin API. They're saved in `--media-dir` under their content's hash and served at
  `/news/media/` with a year-long cache. Keep that folder outside the release folder and back
  it up, because it isn't in the database.
- **The Message Centre** (`/messages`) is where players read messages staff send them from the
  console. It's one way: players can't reply, and never see which staff member wrote. The site
  sets no cookies, so after one sign-in each page carries a token in a hidden form field. The
  token lasts 30 minutes from the last click, and a restart signs everyone out.
- **Shutting down.** Ctrl+C or SIGTERM stops new connections and lets open requests finish.

### Keeping bot farms out

- One address can make **3 accounts an hour and 5 a day**, and check **30 usernames a
  minute**. An IPv6 address counts together with the rest of its /64.
- The last step of registration has a
  [Cloudflare Turnstile](https://developers.cloudflare.com/turnstile/) check. It's on when
  `RS_TURNSTILE_SITE_KEY` and `RS_TURNSTILE_SECRET` are set, with the keys from a Turnstile
  widget in the Cloudflare dashboard that lists the site's hostname. The widget stays hidden
  unless Cloudflare wants a click.

To try the check locally, use Cloudflare's test keys:

```bash
RS_TURNSTILE_SITE_KEY=1x00000000000000000000AA RS_TURNSTILE_SECRET=1x0000000000000000000000000000000AA cargo run -p rs-web
```

| Test key | Behaviour |
|---|---|
| Site key `1x00000000000000000000AA` | Shows the box, then passes |
| Site key `1x00000000000000000000BB` | Passes without showing it |
| Site key `3x00000000000000000000FF` | Always asks for a click |
| Secret `2x0000000000000000000000000000000AA` | Fails every check |

----

## The staff console

`rs-console` runs every world from one web page, and works on phones too. It talks to each
world's private admin API, never straight to the database, and only game staff get accounts.

### First-time setup

```bash
cargo world1                                                    # a world, with its admin API on 8280
cargo run -p rs-console -- user add <name> --role maintainer   # asks for a password (12+ characters)
cargo run -p rs-console                                         # http://127.0.0.1:8150
```

`<name>` has to be a player with staff level 1 or higher, and `user add` asks a world to check.
Then open http://127.0.0.1:8150, sign in, and scan the QR code with an authenticator app. From
then on, every sign-in asks for its six-digit code. Debug builds share the secret `rs_admin`
with debug worlds, so there's nothing else to set.

### Roles

Roles follow the game's staff levels. Each one can do everything the one above it can.

| Role | Level | Can |
|---|---|---|
| `trial` | 1 | Kick, and mute for up to 2 days. No bans, lifting sanctions, deciding appeals, bulk dismissing, or sanctioning linked accounts. |
| `moderator` | 1 | Kick, mute and ban, look players up, read chat logs, free stuck sessions. Can't change anyone's account. |
| `admin` | 2 | Reboot, broadcast, maintenance mode, send players home, reset passwords, restore items, roll back saves, change the hiscores and what the economy tracks, post news. |
| `maintainer` | 3 | Manage staff accounts, change players' stats and items, reload scripts, shut down and start worlds. |

Nobody can moderate staff at or above their own level, or see the recordings, chat, timeline,
addresses or other logs of staff above them.

Manage accounts on the Staff page, or with
`rs-console user list|add|passwd|role|reset-2fa|disable|enable|remove`. Changing a password,
role or authenticator signs that account out everywhere.

### What it can do

**Worlds**

- Reboot with a countdown (staggered if you like, with a reason players see), cancel a reboot,
  and broadcast.
- **Maintenance mode** closes a world to new logins. Players already on stay, staff can still
  log in, and a restart opens it again.
- Maintainers can **shut a world down** so it stays off, and start it again.
- **Schedule** reboots and broadcasts for later, once or on repeat. If the console was down
  and a job is more than 5 minutes late, it's skipped, and the audit log says so.
- **Controls** are kill switches for trading, the duel arena, dropping, picking up, shops,
  banking, or a script by name. There's also a lockdown for accounts younger than a set age
  (staff are exempt). They stay on until turned off, and come back on any world that restarts.

**Players**

- Look anyone up, then kick, mute or ban. **Sanction templates** pick the length from a ladder,
  e.g. macroing is 2 days, then 2 weeks, then permanent.
- **Timeline:** logins, activity and wealth logs, chat, moderation, abuse reports and sessions,
  newest first.
- **Linked accounts:** every address they logged in from (kept 180 days) and the accounts that
  share one, with a button to ban them all. Moderators see addresses as labels like
  `addr-3f9a1c`. Admins see the real ones, and that's audited.
- **Devices:** each browser or client install they used, numbered D1, D2 and so on, with the
  addresses it was used from.
- **Link flags** warn about things like logins from a banned account's browser, headless
  browsers, or one browser shared by 4 or more accounts. The **ban chain** follows shared
  browsers and addresses up to 3 links out and bans them together.
- **Staff notes** (which can put a player on the watchlist), **progress charts** (XP, wealth
  and when they play) and **chat logs**, public and private, with a live mode. Reading chat is
  audited.

**Player support**

- **Session** shows why someone can't log in, and **Free session** releases a stuck one. It
  never lets go of a player who's online or whose newest save isn't in the database.
- **Send home** (admins) sends a player to Lumbridge or the start of Tutorial Island.
- **Reset password** (admins) gives a one-time code the player uses at `/recover` on the
  website within 3 days. Only its hash is stored.
- **Restore items** and **earlier saves** (admins). Saves are kept at logout, every hour, and
  before every staff change, for 30 days. You can compare, restore or roll back. Both need an
  open case, and a rollback can be undone.

**Moderation center**

- **Cases:** one per player being looked into, worked on by as many staff as needed. A case
  holds its evidence (copied in, so it outlives pruning), comments with images and videos, a
  status and a priority. It shows who else has it open.
- **Triage:** abuse reports and flagged sessions that aren't in a case yet. Open a case, add to
  one, or dismiss.
- **Detections:** the console opens cases by itself for anti-cheat scores of 50 or more,
  possible ban evasion, players reported 3 times in a day, and staff lifting sanctions on
  accounts linked to their own. New ones show up as a toast and a count in the menu and the
  tab title.
- **Appeals:** players appeal a ban or mute on the website (`/appeal`), and each appeal becomes
  a case within 30 seconds (this needs `--web-url`). Whoever gave the sanction can't decide it.
- **Staff activity** (admins): each staff member's actions and response times over 7, 30 or
  90 days, for coaching trial moderators.
- **Conflicts:** cases and notes about a staff member are hidden from them and from staff at or
  below their level. Staff can't look up their own account, and can only comment on accounts
  linked to theirs. Maintainers aren't held to this, but are detected and audited like anyone
  else.

**Replays and Investigate**

- **Replays:** every session is recorded and scored by anti-cheat checks. Watch any of them in
  the game's own client, with the player's mouse, at up to 8x speed, next to a timeline of what
  they did. A session that's still going can be watched live, a few seconds behind. Watching is
  audited. See [Session replays and anti-cheat](#session-replays-and-anti-cheat).
- **Investigate:** a live map of every online player (click one to watch, kick, mute or ban),
  the most flagged players, ban evasion, and likely bot farms grouped by shared addresses,
  sign-up times, names, click rhythm and hours played.

**Economy**

- Worlds log every item that passes between players (trades, stakes, drops, pickups), valued at
  the cache's prices.
- **Circulation** counts coins and tracked rares across every save every 15 minutes and charts
  them. A jump past an item's threshold (5% by default) raises an alert, which is often the
  first sign of a dupe.
- **Who owns**, **item trace** (where a stack came from, up to 6 holders back) and **trade
  graph** (which shows rings and mules).
- **Ring rollback** (admins) rolls a whole dupe ring back to before one moment, in one go.
- **Hiscores:** staff level 2 and up and banned players are never ranked. Admins can hide an
  account or remove some of its entries.

**Maintainer tools**

- **Script reload** recompiles one world's scripts and swaps them in without a restart. Check
  first: if they don't compile, nothing changes. It covers scripts only. Configs, maps and
  interfaces still need a restart.
- **Staff:** add accounts, change roles and passwords, reset 2FA, and sign people out
  everywhere. Each change needs your password and a fresh code. Click a name for their profile:
  sessions, numbers, audit entries, sanctions, cases and sign-ins.
- **Stats and items:** set stats, give or take items, and move players, online or offline. All
  audited.
- **News** (admins too): the website's news in Markdown, with images and videos, through its
  admin API (`--web-url`, `RS_ADMIN_WEB_URL`).

**Getting around:** Ctrl+K (or `/`) searches players, cases, recordings, audit entries, pages
and actions. Pins, saved filters, CSV export (audited) and your time zone follow your account.

### How it works

- **The admin API.** Each world serves one on `8270 + node_id` (`--admin-port`). Requests are
  signed with `RS_ADMIN_SECRET`, which the console and every world share. Mutes and bans go
  through a world exactly like `::mute` and `::ban`. Any online world answers lookups from the
  game database.
- **`rs-console/worlds.json`** lists the worlds and their private admin API `url`s. Unlike
  `rs-web/worlds.json`, it isn't public. A world's `client` is its public web address, which
  replays need, and its `unit` is its systemd unit, which the Start button needs.
- **Security.** Sessions live in memory, so a restart signs everyone out. They end after 30
  minutes idle or 12 hours at most. Cookies are `HttpOnly` and `SameSite=Strict`, every change
  needs a CSRF token and passes an Origin check, and there's a strict CSP. 5 failed sign-ins
  lock an account for 15 minutes.
- **The audit log.** The Audit log page, its export, search and Staff activity hide what staff
  above you did. They show addresses only for your own entries and those of staff below you.
  Maintainers see everything.
- **Keep it private.** It listens on `127.0.0.1` by default. On a tailnet, serve it with
  `tailscale serve --bg --https=8150 http://127.0.0.1:8150` and start it with
  `--https true --origin https://<host>.<tailnet>.ts.net:8150`. Keep each world's admin API
  private too (`--admin-host`).

**Data** lives in `rs-console/data/`. `RS_ADMIN_DATA` or `--data-dir` moves it. Back up the
whole folder.

| File | Holds |
|---|---|
| `users.json` | Accounts, with argon2 password hashes and TOTP secrets |
| `audit.jsonl` | Everything done, sign-ins included. Only ever added to. |
| `cases.jsonl` | Every change to a case. Only ever added to, and cases are rebuilt from it at startup. |
| `offences.jsonl` | Sanctions given by template, which the template ladders count. Trimmed to the newest 20,000 now and then. |
| `media/` | Images and videos posted on cases, shown only to signed-in staff |
| `notes.jsonl`, `prefs.json`, `controls.json`, `schedule.json`, `economy.json`, `economy-samples.jsonl` | Staff notes, account preferences, kill switches, scheduled jobs and economy tracking |

----

## Session replays and anti-cheat

### Recording

Every world records each player's session. That's every packet sent to them (without the
ISAAC cipher) and every packet their client sends back.

- **How long:** `--replay-days` sets how many days recordings are kept (7 by default). `0`
  records nothing and runs no checks.
- **Where:** in the `replays` and `replay_chunks` tables, pruned once older than the setting.
- **How big:** a few kilobytes per minute of play. Chunks are compressed with zstd by a task
  with its own database connection, so saves never wait on it. Older chunks stored before zstd
  use deflate, and still play.
- **Limits:** a recording (and its checks) ends after 20 days of one session, and chunks are
  dropped once 32 MB of them are waiting to be stored.

### Watching

**The player.** A world serves `/replay`: its own `client.js`, with `replay.js` loaded first.
The console shows it in a frame and hands it the recording. The script pretends to be the
world. It fakes the game connection, answers the login handshake, and pins `Math.random` while
the client picks its ISAAC seeds, so it can cipher the recorded packets the same way. It also
runs the page on a virtual clock, so pause, speed and fast-forward drive the client's own game
loop. Maps and models load from the world as usual.

A recording plays on a world of the revision it was recorded on, and the clients aren't changed
at all. Whatever the script needs from a client (its login, scene state, camera, drawing step,
connection timeout) it finds in the client's code by pattern.

**Seeking.** Fast-forward runs the client's game loop one 20ms tick per step, many steps at a
time, and skips drawing except just before a recorded click (the client builds its menus while
drawing). One tick per step matters: the client decides how many ticks to run from how long its
last steps took, and anything else leaves walking and the camera behind the recording. A seek
ends up the same as playing that stretch normally.

Every minute, the world records a **keyframe**: everything a freshly logged-in client needs to
show what the player saw. That's the map and what's on it, inventories, the players and NPCs in
view, the latest of each packet that sets lasting state (interface text, varps, stats), and
recent chat. Going back, or more than 15 seconds forward, logs the client out with the game's
own logout packet and back in from the nearest keyframe, then fast-forwards from there. Nothing
reloads.

Replays play sound effects but no music. The client renders a whole song up front, which is
seconds of heavy work, and would do it again on the fast clock.

**Buffering.** While it plays, the page keeps a frame of the game every half second. A second,
hidden, silent copy of the replay (the **scout**, at `/replay?scout=1`) sits under the replay
and sweeps the whole recording, coarse first and then down to a frame every 5 seconds (wider on
long sessions, at most about 600). It has to stay on screen, because Chrome runs a hidden one
only once a second. Frames are kept up to 160 MB, and past that every other one is dropped, so
hours of play stay covered.

Seeking somewhere buffered shows that frame at once while the client catches up behind it.
Hovering the timeline previews it, and a spot that isn't buffered yet jumps to the front of the
scout's queue. A bar under the timeline shows what's buffered.

**The dock.** Loading the client takes seconds, so the console keeps one (and its scout) loaded
between replays. It starts ahead of time on the replays list and on player pages, and only logs
in to a recording when one opens, so switching sessions is just a logout and a login. Moving a
frame around the page would reload it, so the dock stays fixed on top of the page, laid over
the replay page's stage, and waits out of sight (but still in view) elsewhere. It's let go five
minutes after you leave the last replay page. Finished recordings the console downloads stay in
memory (up to 256 MB) for the rest of the visit.

**Input.** The player's input comes from the client's own reports.

- From 254: `EVENT_MOUSE_MOVE` (the cursor every 50ms), `EVENT_MOUSE_CLICK` and
  `EVENT_APPLET_FOCUS`.
- Before 274: `EVENT_TRACKING` (mouse, keys, focus), which the world turns on at login and asks
  for about every 10 seconds. Input from the last few seconds before a disconnect is lost.
- The 225 web client doesn't report mouse moves, so its replays only show the cursor where it
  clicked.

**The camera** follows the client's `EVENT_CAMERA_POSITION` reports on 225 and 254+. 244 and
245.2 have none, so their replays press the recorded arrow keys instead. A minimap click also
carries the minimap's tilt and zoom (random, drifting a step every 10 seconds or so, against
screen-reading bots) and the camera's yaw, and so does 225's camera report. The replay sets
the client's to match, so its minimap looks like the player's did and a replayed minimap click
puts the flag where theirs went. On 244 and 245.2 the yaw also corrects the camera the arrow
keys turn.

**Chat**, private messages and `::` commands are typed into the client again from the packets
that sent them, a key at a time, ending as they were sent, so the client shows them as it did
(274+ reports no keys at all). The timeline lists them too.

**Free look.** The Free look button pauses and hands the client to you. Your mouse works its
menus, tabs and tooltips and the arrow keys turn the camera, while the player's cursor is still
drawn. The recording stands still but the client keeps running, so it responds. Nothing you do
is sent anywhere, and you can't type into it. Turning it off (or pressing play) puts the client
back as it was. If you clicked or turned anything, it restarts from the nearest keyframe to the
same spot.

### The checks

These flag sessions for staff to review. They never act on their own.

| Flag | Means |
|---|---|
| Modified client | An `ANTICHEAT_*` packet had the wrong contents (they're fixed per revision family: 225-245.2 and 254-289), or a packet was shorter than any the client sends of its kind. |
| Rejected requests | 10 or more requests (and a tenth of all actions) that the world turned down as ones the client wouldn't make: buttons on interfaces that aren't open or don't have them, objs in slots that hold something else. Mistyped cheat commands don't count. |
| Auto-banned | The world banned the account (48 hours) for a packet only a modified client sends: an abuse report with a reason the screen doesn't offer, an offender name that can't exist, or a private message to such a name. The stats say which. |
| No game loop | 20+ actions and none of the heartbeats the client's game loop sends (`ANTICHEAT_CYCLELOGIC3/4`, or `4/7` from 254). It isn't running the client. |
| Forged heartbeats | The game loop sends its two heartbeats a fixed number of cycles apart (`CYCLELOGIC4` every 111 and `3` every 128, plus `5` every 86 on 244-245.2; from 254, `7` every 63 and `4` every 193), so their counts always keep that ratio. Once there are 15 of the rarer one, a count more than 3 (and 6%) off what the ratio predicts means something else is sending them. A real client is within a count or two. |
| Never drew | 254+: 20+ actions and 30 `CYCLELOGIC4`s (about 2 minutes) without a single `CYCLELOGIC1`, which the client sends from drawing the game every 1175 frames. |
| Missing op checks | The client sends `ANTICHEAT_OPLOGIC*` with certain actions once a hidden count passes a threshold. The world mirrors the counts and expects them. |
| Actions without clicks | 254+: over a quarter of 20+ actions came without the click the client reports alongside each one. |
| Actions while unfocused | 3+ actions while the client said its window had lost focus (not on touch screens, whose keyboard takes focus). A real click focuses the window first, and a script's fake click doesn't. Before 254, the click's press is judged from the input reports. |
| Clicked what it wasn't shown | Actions in 3+ separate stretches (including "use" and spells) on an NPC or player the world wasn't sending the client, a loc outside its map, or an obj in a zone it wasn't sent since the map was built. Anything that just left view still counts as shown for 10 ticks, to allow for clicks made before the update arrived, and actions within 10 ticks of the first count once, so one stalled connection doesn't count several times. |
| Menu options without right-click | Most of 5+ actions on options a left click can't pick (worked out from the types, the way the client builds its menu) came with no right press to open the menu. Not counted for players' options, one-button mouse mode or touch screens. |
| No mouse | 20+ actions and no mouse reports at all. |
| Cursor jumps | 244+: most of 20+ clicks landed over 40 pixels from the cursor's last reported spot. |
| Clockwork clicks | 30+ clicks at very even intervals. |
| Forged minimap clicks | A minimap click ends in 14 bytes: the spot clicked, the camera's yaw, the minimap's random tilt and zoom, where the client drew the player, and the fixed values 57, 89 and 63. Two or more with wrong fixed values, values out of range, or a spot that (worked out as the client does) doesn't lead to the tile walked to. |
| Minimap position off | Most of 5+ minimap clicks drew the player more than 8 tiles from where the world has them. |
| Instant reactions | Half of 20+ reactions came faster than 180ms, network included. A reaction is the first action after the thing being worked on went away (a rock ran out, a tree fell, an NPC died) or after an obj they then took appeared, timed from when the world sent it to when the action arrived. Only counted after a second with no actions, so clicking away the whole time isn't a reaction. |
| Even reactions | 20+ reactions that hardly vary (standard deviation under 0.15 of the mean). People have a long tail. |
| Long days | 16+ hours played on 3 or more of the last 7 days (UTC), from the account's recorded sessions. |
| No long breaks | Played for 3 days without ever stopping for 4 hours. |
| Instant relogs | 4 in 5 of 5+ returns this week came within 15 seconds. A return is a login after a session the player didn't end themselves: a lost connection, an idle logout, a kick or ban, or a reboot. It's timed from when the world let them go (a lost connection keeps them in for a while, and the client reconnects by itself meanwhile), or from the world coming back after a reboot. Logouts are left out, because world hops are back in seconds. |
| Repeated laps | Of 150+ recent actions, a fifth of them walks, 85% or more of the runs of 4 were seen before: the same tiles, booth and order every lap. A person's walks land a tile or two off. |
| Same pixel clicks | Half of 20+ aimed clicks on inventory slots hit the same pixel of the slot. A click only counts after the cursor moved, so clicking the same slot again where the cursor already is (withdrawing one at a time) isn't aiming. Slots that scroll, like the bank, aren't judged. |
| Even slot clicks | 50+ clicks on inventory slots spread over the whole slot (8.3+ pixels from their average, where a perfectly even spread is 9.2) instead of bunching on the obj in the middle. Not for touch screens. |
| Straight mouse moves | 254+: 60% of 30+ moves (5+ samples, 40+ pixels) went in a dead straight line. |
| Steady mouse moves | 254+: 60% of 30+ moves were no faster through their middle third than 1.3 times their ends. People speed up and slow down, and move about twice as fast midway. |
| Repeated mouse curves | 254+: 40% of 20+ curved moves followed the shape of an earlier one. |
| Anti-AFK | An hour in total of 15+ minute stretches with no actions, clicks or chat, no idle report from the client, and nothing but tiny mouse nudges or one or two keys at even intervals (or, from 274, where keys aren't reported, nothing at all). |
| Camera never turned | 225 and 254+: 3+ hours and 300+ actions without the camera turning once. |

A session's score is the total of its flags' weights, and the measurements behind them are kept
with it. The checks on behaviour over time (Instant reactions and everything below it) weigh 15
to 30 each, so none of them opens a case alone. A real client scores 0. A packet bot that only
sends walks scores 110: no game loop, actions without clicks, and no mouse.

----

## Server options

Everything is set with flags. The database, secret and cluster settings can also come from the
environment variable shown. Run `cargo run -p rs-server -- --help` for the full list straight
from the server. `rs-server migrate` applies pending database changes and exits (put any flags
before `migrate`).

**The world**

| Flag | Default | What it does |
|---|---|---|
| `--node-id <ID>` | `10` | The world's node id. 10 is world 1, 11 is world 2, and so on. The ports below are worked out from it. |
| `--members <BOOL>` | `true` | Members or free-to-play rules. `--members false` runs a free world. |
| `--client-pathfinder` | `true` | Trust the paths the client works out for walking. |
| `--verify` | `true` | Check at startup that the packed cache matches byte for byte. |
| `--no-tui` | `false` | Log to stdout instead of showing the dashboard. It does this by itself when stdout isn't a terminal. |
| `--auto-register <BOOL>` | `true` in debug, `false` in release | Create an account the first time an unknown name logs in (debug builds also make it staff level 3). In release, players register on the website. |
| `--auto-migrate <BOOL>` | `true` in debug, `false` in release | Apply database changes at startup. When off, the server only checks the schema and keeps logins offline until `rs-server migrate` has run. |
| `--replay-days <DAYS>` | `7` (`RS_REPLAY_DAYS`) | Days to keep session recordings. `0` records nothing. |

**Network**

| Flag | Default | What it does |
|---|---|---|
| `--host <HOST>` | `0.0.0.0` | Address the game and HTTP ports listen on. |
| `--http-port <PORT>` | `8070 + node_id` (`8080`) | The web client and cache files. |
| `--tcp-port <PORT>` | `43584 + node_id` (`43594`) | The game protocol port. |
| `--private-key <PATH>` | `.keys/private.pem` | The RSA private key for the login handshake. |
| `--real-ip-header <NAME>` | unset | The header a reverse proxy puts the player's address in (e.g. `x-real-ip`), for per-address connection and login limits. Only set it when the proxy is the only way in, or players can fake their address. |
| `--site-origin <ORIGINS>` | unset (`RS_SITE_ORIGIN`) | The website's addresses, space-separated (e.g. `"https://rustcity.org https://www.rustcity.org"`). Only these may frame the client page, and only these get the browser's game id from `/uid` (for the website's account recovery form). When unset, any site can frame it and `/uid` tells nobody. WebSockets always have to come from a page this world served. |

**Database**

| Flag | Default | What it does |
|---|---|---|
| `--db-host <HOST>` | `localhost` (`RS_DB_HOST`) | Postgres hostname. |
| `--db-port <PORT>` | `5432` (`RS_DB_PORT`) | Postgres port. |
| `--db-name <NAME>` | `postgres` (`RS_DB_NAME`) | Database name. |
| `--db-user <USER>` | `postgres` (`RS_DB_USER`) | Database user. |
| `--db-pass <PASS>` | `RS_DB_PASS`, or `password` in debug only | Database password. Required in release builds. |

The defaults match `docker-compose.yml`, so locally the server connects with no extra flags.

**Secrets and the admin API**

| Flag | Default | What it does |
|---|---|---|
| `--pepper <PEPPER>` | `RS_PEPPER`, or `localhost` in debug only | The secret mixed into every password hash. Required in release builds. |
| `--cookie <COOKIE>` | `RS_COOKIE`, or `rs_secret` in debug only | The Erlang cookie every ether node in the cluster shares. Required in release builds. |
| `--admin-secret <SECRET>` | `RS_ADMIN_SECRET`, or `rs_admin` in debug only | Shared with `rs-console`, which signs every request with it. Required in release builds. |
| `--admin-port <PORT>` | `8270 + node_id` (`8280`) | The private admin API the staff console calls. `0` turns it off. |
| `--admin-host <HOST>` | `127.0.0.1` | Address the admin API listens on. Keep it private: loopback or a Tailscale address. |

**The ether cluster**

| Flag | Default | What it does |
|---|---|---|
| `--ether-port <PORT>` | `5000 + node_id` (`5010`) | The ether sidecar's port. |
| `--node-host <HOST>` | `127.0.0.1` (`RS_NODE_HOST`) | The host part of this world's ether node name (`world<id>@<host>`). Other worlds must be able to reach it. |
| `--cluster <LIST>` | empty (`RS_CLUSTER_HOSTS`) | The ether nodes, comma-separated (e.g. `world10@10.0.0.5,world11@10.0.0.6`). Empty means `world10` to `world20` on `--node-host`. |
| `--ether-release <PATH>` | unset (`RS_ETHER_RELEASE`) | A built ether release (`rs-ether/_build/prod/rel/rs_ether`) to run instead of `mix` from source. |

----

## Build profiles

These are set in `.cargo/config.toml`.

| Profile | Use it for | Command |
|---|---|---|
| `dev` (default) | Everyday work. No optimisation, full debug info, fast to compile, and hot reload is on. | `cargo run -p rs-server` |
| `dev-opt` | Running a real world locally. `dev` with `opt-level = 2`. | `cargo run --profile dev-opt -p rs-server` |
| `release` | Production and benchmarks. `opt-level = 3`, fat LTO, and `panic = "unwind"`, which the engine's crash recovery (`catch_unwind`) depends on. Slowest to compile. | `cargo run --release -p rs-server` |

----

## Supported revisions

The engine supports several revisions of the game. Pick one by changing `REV` in
`.cargo/config.toml`, then rebuild:

```toml
[env]
REV = "289"
```

| Revision | Date | Updates it includes |
|---|---|---|
| **225** | 18 May 2004 | [Big Chompy Bird Hunting](https://runescape.wiki/w/Update:Big_Chompy_Bird_Hunting) |
| **244** | 28 Jun 2004 | [New game update system](https://runescape.wiki/w/Update:New_game_update_system), [Agility improved and bug fixes](https://runescape.wiki/w/Update:Agility_improved_and_bug_fixes), [New in-game Player Moderators](https://runescape.wiki/w/Update:New_In-Game_Player_Moderators), [Special Attacks](https://runescape.wiki/w/Update:Special_Attacks), [Various tweaks to the game](https://runescape.wiki/w/Update:Various_tweaks_to_the_game), [Lots more improvements](https://runescape.wiki/w/Update:Lots_more_improvements) |
| **245.2** | 13 Jul 2004 | [Easier to rearrange bank](https://runescape.wiki/w/Update:Easier_to_rearrange_bank), [Priest In Peril](https://runescape.wiki/w/Update:Priest_In_Peril_Quest), [Nature Spirit](https://runescape.wiki/w/Update:Nature_Spirit_Quest) |
| **254** | 7 Sep 2004 | [Quest Journals and Chompy Hats](https://runescape.wiki/w/Update:Quest_Journals_and_Chompy_Hats), [Agility, Potions and Parties!](https://runescape.wiki/w/Update:Agility,_Potions_and_Parties!), [New Random Events!](https://runescape.wiki/w/Update:New_Random_Events!), [Death Plateau](https://runescape.wiki/w/Update:Death_Plateau_Quest), [Troll Stronghold](https://runescape.wiki/w/Update:Troll_Stronghold_Quest), [Herblore Additions](https://runescape.wiki/w/Update:Herblore_Additions), [Situation in the Sands](https://runescape.wiki/w/Update:Situation_in_the_Sands) |
| **274** | 24 Nov 2004 | [Tai Bwo Wannai Trio](https://runescape.wiki/w/Update:Tai_Bwo_Wannai_Trio), [Plague City Part 4](https://runescape.wiki/w/Update:Plague_City_Part_4_Released), [Skills, Duels and the Kalphite](https://runescape.wiki/w/Update:Skills,_Duels_and_the_Kalphite), [Eadgar's Ruse](https://runescape.wiki/w/Update:Eadgars_Ruse), [Morytania Expansion](https://runescape.wiki/w/Update:Morytania_Expansion), [Mort'ton, Shades and Mage Armour](https://runescape.wiki/w/Update:Mortton_Shades_and_Mage_Armour), [Mage Armour Updated](https://runescape.wiki/w/Update:Mage_Armour_Updated), [Treasure Trails and Changes](https://runescape.wiki/w/Update:Treasure_Trails_and_Changes), [The Fremennik Trials](https://runescape.wiki/w/Update:The_Fremennik_Trials), [Horror From The Deep](https://runescape.wiki/w/Update:Horror_From_The_Deep), [Burthorpe Games Room](https://runescape.wiki/w/Update:Burthorpe_Games_Room) |
| **289** | 17 Jan 2005 | [Throne Of Miscellania](https://runescape.wiki/w/Update:Throne_Of_Miscellania), [Monkey Madness](https://runescape.wiki/w/Update:Monkey_Madness), [Various Small Changes](https://runescape.wiki/w/Update:Various_Small_Changes.), [More Small Changes](https://runescape.wiki/w/Update:More_Small_Changes), [Castle Wars](https://runescape.wiki/w/Update:Castle_Wars), [Changes to Castle Wars](https://runescape.wiki/w/Update:Changes_to_Castle_Wars), [Santa, Flax and Castlewars](https://runescape.wiki/w/Update:Santa,_Flax_and_Castlewars), [The Haunted Mine](https://runescape.wiki/w/Update:The_Haunted_Mine), [Troll Romance, Banks and Chat](https://runescape.wiki/w/Update:Troll_Romance,_Banks_and_Chat), [In Search Of The Myreque](https://runescape.wiki/w/Update:In_Search_Of_The_Myreque), [Trawler Game Update](https://runescape.wiki/w/Update:Trawler_Game_Update), [Karamja Dungeon](https://runescape.wiki/w/Update:Karamja_Dungeon) |

----

## Deploying

The setup is one website, many worlds and one shared Postgres. Each world runs its own ether
node, and the nodes join into one cluster for cross-world social features.

1. **Secrets come from the environment.** Set `RS_DB_PASS`, `RS_PEPPER`, `RS_COOKIE` and
   `RS_ADMIN_SECRET`. The website needs the first two and the staff console only the last.
   Release builds won't start without them. Flags work too, but anyone who can list processes
   can read those. Give the website `RS_TURNSTILE_SITE_KEY` and `RS_TURNSTILE_SECRET` as well,
   or registration has no bot check.

2. **Build ether once.** In `rs-ether/`, run `MIX_ENV=prod mix deps.get` and then
   `MIX_ENV=prod mix release`. Point each world at the result with
   `RS_ETHER_RELEASE=rs-ether/_build/prod/rel/rs_ether`. The world's host doesn't need Elixir
   installed, just a matching OS.

3. **Migrate as a separate step**, before rolling out new binaries: `rs-server migrate` (the
   game's tables, then ether's). Release servers never migrate by themselves. They check the
   schema and keep logins offline until it's current. They're fine with changes from a *newer*
   build, so old worlds keep running during a rollout, as long as each migration is backward
   compatible.

4. **Patch every release you deploy.** The repo's `.keys/private.pem` is public. Copy
   `deploy/patch.py` to the server, and in each unpacked release, before starting the world, run
   `python3 /path/to/patch.py` (it needs `openssl`).
   - It writes a new key and patches `public/<rev>/client.js` to match. Run it on the server
     only, and never commit or upload the key. The Java client keeps the old key, so it can't
     log in afterwards.
   - It also turns the web client's login uid (a fixed 1337 in the build) into a random id per
     browser. Worlds log it with each login, so the staff console can link accounts played from
     the same browser, the way it does by address.

5. **Name nodes by hosts they can reach.** Give each world `--node-host` (its private IP or
   hostname) and every world the same `--cluster` list, e.g.
   `world10@10.0.0.5,world11@10.0.0.6`.

6. **Keep the cluster private.** Anyone with the cookie who can reach a node can run code on
   it. Firewall EPMD (`4369`) and the distribution ports to the private network. To pin those
   ports, give the world process
   `ERL_AFLAGS="-kernel inet_dist_listen_min 9100 inet_dist_listen_max 9110"`, and the sidecar
   inherits it.

7. **Put TLS on a reverse proxy.** The web client connects back to its page's own host over
   `ws` or `wss`, so terminate TLS for the website and for each world's HTTP port. Behind a
   proxy every connection seems to come from the proxy, so:
   - have the proxy put the player's address in a header (nginx:
     `proxy_set_header X-Real-IP $remote_addr;`),
   - start the world with `--real-ip-header x-real-ip`,
   - and listen only where the proxy can reach it (e.g. `--host 127.0.0.1`).

8. **Run one staff console, privately.**
   - Put `rs-console` on a host that can reach every world's admin API over the private network
     (on the worlds' own host, they can stay on `127.0.0.1`). Give it the same
     `RS_ADMIN_SECRET`, and `RS_ADMIN_DATA` on a disk that persists.
   - List the worlds' private admin URLs in a worlds file passed with `--worlds`. For replays,
     give each world its public `client` address (e.g. `https://w1.example.com`), which the
     console's browser loads `/replay` from.
   - Serve it over HTTPS on the private network only (e.g. `tailscale serve --bg 8150`), and
     start it with `--https true --origin https://<that host>`. Never expose it, or any admin
     API, to the internet.
   - Create the first account with `rs-console user add <name> --role maintainer`, run in the
     console's environment with its `--worlds`. The worlds must be up, since it asks one whether
     the name is game staff.
   - Worlds come back after a console reboot only if their service manager restarts them
     (systemd `Restart=always`).
   - To post world actions to a Discord channel (reboots, shutdowns, starts, cancelled reboots,
     broadcasts, maintenance, scheduled jobs, kill switches and lockdowns), make a webhook in the
     channel's settings (Integrations, then Webhooks) and give its URL as
     `RS_ADMIN_DISCORD_WEBHOOK`. Each action is one message, however many worlds it covered.
     The URL lets anyone post to the channel, so keep it as secret as `RS_ADMIN_SECRET`.
   - For news, give `rs-web` the same `RS_ADMIN_SECRET` and an admin port the console can reach
     privately (`--admin-host`, `--admin-port`), and point the console at it with
     `RS_ADMIN_WEB_URL`. Give the website a `--media-dir` outside the release folder for news
     images and videos, and let its database login insert, update and delete `news_posts`.

9. **Keep recordings out of routine backups.** Session recordings are big and short-lived, so
   dump with `pg_dump --exclude-table-data=replay_chunks`.

10. **Give the website's database login what it needs.** If the website runs as its own
    restricted database role, `/appeal`, `/recover`, `/recover/request`, `/account` and
    `/messages` (and the console's Messages page, through the admin API) need these grants on
    top of news, hiscores and registration:

    ```sql
    GRANT SELECT (id, display_name, password_hash, muted_until, banned_until, reset_code_hash, reset_code_until) ON players TO rs_web;
    GRANT UPDATE (password_hash, reset_code_hash, reset_code_until) ON players TO rs_web;
    GRANT SELECT, INSERT, UPDATE ON appeals TO rs_web;
    GRANT SELECT, INSERT, UPDATE ON recovery_requests TO rs_web;
    GRANT SELECT (id, moderator_id, target_id, action, hours, reason, created_at) ON moderation_logs TO rs_web;
    GRANT SELECT, INSERT, DELETE ON login_failures TO rs_web;
    GRANT SELECT, INSERT, UPDATE (read_at) ON staff_messages TO rs_web;
    -- the account page (/account)
    GRANT SELECT, DELETE ON players TO rs_web;
    GRANT SELECT ON login_logs, player_saves, save_history, hiscores, chat_logs, session_logs, wealth_events, snapshots TO rs_web;
    GRANT SELECT (id, player_id, node_id, started_at, ended_at, duration_ms) ON replays TO rs_web;
    GRANT SELECT (node_id) ON moderation_logs TO rs_web;
    GRANT SELECT, DELETE ON friends, ignores TO rs_web;
    -- a player's hiscores ranks (/hiscorepersonal, /index_lite.ws)
    GRANT SELECT ON hiscores_removed TO rs_web;
    ```

----

## Contributing

Everyone's welcome to help. I'm especially keen on performance work, whether that's CPU or
memory.

- If your PR improves performance, please include benchmarks from before and after.
- If your PR changes the engine or content, please include your sources.
- I'm on Windows and can't test on Linux or macOS myself, so I rely on GitHub Actions for those.

> [!TIP]
> Every PR runs a GitHub Action that checks every revision builds, compiles its content and
> passes its tests, so you don't need to test each revision yourself.

----

*Bearer of the curse, seek misery.*
