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

`rs-majula` is the project, Cargo workspace, and canonical engine name: a
from-scratch Rust reimplementation of a **RuneScape 2** game server, with
byte-identical protocol and content emulation, a single-threaded deterministic game
loop, and an async `tokio` host. The stock client connects and plays against
unmodified cache content.

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
| **Elixir** + `mix`      | `~> 1.15`                               | **Required to log in.** Runs the `rs-ether` sidecar, which the login flow depends on (cross-world login checks). The server boots and serves the web client without it, but every login attempt returns *login server offline*. Auto-spawned via `cmd /c` (Windows); on Linux/macOS start it from `rs-ether/` yourself. |
| **Git**                 | --                                      | `rs-ether` is a submodule -- clone with `--recursive`.                                                                                                                                                                                                                                                                  |

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
#    On first boot the ether sidecar runs `mix deps.get` + `ecto.create` +
#    `ecto.migrate` against Postgres, so Docker must be up first.
cargo run -p rs-server

# 3. Open the web client and log in
#    http://localhost:8080/rs2.cgi
```

> First build compiles the whole workspace and takes a while. For running a
> populated world locally, prefer the `dev-opt` profile (near-release speed,
> still has debuginfo): `cargo run --profile dev-opt -p rs-server`.

> For **maximum performance** (production or benchmarking), run in release mode --
> `cargo run --release -p rs-server` -- for full optimizations and fat LTO, at the
> cost of the longest compile.

Stop Postgres when you're done with `docker compose down` (add `-v` to also wipe
the database volume).

----

## Running a second world (cluster)

The `cargo world1` / `cargo world2` aliases bring up a two-node cluster (each
auto-spawns its own ether sidecar; they mesh via the shared `--cluster` list):

```bash
docker compose up -d
cargo world1   # --node-id 10, cluster world10+world11
cargo world2   # --node-id 11, in a second terminal -- the cluster meshes
```

See [`rs-ether/README.md`](rs-ether/README.md) for sidecar details.

----

## Command-line arguments

All configuration is via CLI flags (clap). This is the complete set -- run
`cargo run -p rs-server -- --help` for the canonical output.

| Argument                      | Default                     | Description                                                                                                    |
|-------------------------------|-----------------------------|----------------------------------------------------------------------------------------------------------------|
| `--host <HOST>`               | `0.0.0.0`                   | Bind address for the TCP game + HTTP listeners.                                                                |
| `--http-port <HTTP_PORT>`     | `8070 + node_id` (`8080`)   | HTTP port -- web client + cache archives.                                                                      |
| `--tcp-port <TCP_PORT>`       | `43584 + node_id` (`43594`) | TCP game-protocol port.                                                                                        |
| `--private-key <PRIVATE_KEY>` | `.keys/private.pem`         | RSA private key (PEM) for the login handshake.                                                                 |
| `--members`                   | `true`                      | Members world (vs free-to-play) content rules.                                                                 |
| `--client-pathfinder`         | `true`                      | Trust client-computed movement paths.                                                                          |
| `--no-tui`                    | `false`                     | Disable the TUI dashboard; use stdout logging (auto-falls back when stdout isn't a TTY).                       |
| `--verify`                    | `true`                      | Verify packed-cache byte-identity at boot (`pack_all`).                                                        |
| `--node-id <NODE_ID>`         | `10`                        | World node ID (10 = world 1, 11 = world 2, …); drives the derived port scheme.                                 |
| `--ether-port <ETHER_PORT>`   | `5000 + node_id` (`5010`)   | Ether sidecar TCP port.                                                                                        |
| `--db-host <DB_HOST>`         | `localhost`                 | Postgres hostname.                                                                                             |
| `--db-port <DB_PORT>`         | `5432`                      | Postgres port.                                                                                                 |
| `--db-name <DB_NAME>`         | `postgres`                  | Postgres database name.                                                                                        |
| `--db-user <DB_USER>`         | `postgres`                  | Postgres username.                                                                                             |
| `--db-pass <DB_PASS>`         | `password`                  | Postgres password.                                                                                             |
| `--cluster <CLUSTER>`         | `""`                        | Comma-separated cluster node list (e.g. `world10@127.0.0.1,world11@127.0.0.1`), forwarded to the sidecar mesh. |
| `--pepper <PEPPER>`           | `localhost`                 | Server-side pepper for password hashing.                                                                       |

`--members`, `--client-pathfinder`, `--no-tui`, and `--verify` are boolean flags
(defaults shown). The `--db-*` defaults match `docker-compose.yml`, so the server
connects with no extra flags.

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

## Multi-Revision

The engine supports multiple revision targets. The ones listed below can be targeted to your choice.
Simply change the `REV` located in `/.cargo/config.toml` and rebuild.

```toml
[env]
REV = "244"
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

----

*Bearer of the curse, seek misery.*