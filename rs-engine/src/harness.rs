//! Headless engine harness: a player without a socket, and the test commands
//! that need a live world.
//!
//! Built only with the `harness` feature, which `rs-test` enables. Nothing here
//! is reachable from the server binary.
//!
//! A harness player logs in through [`Engine::accept_login`], the same path a
//! real login completes on, so the `[login]` trigger, `on_login` and the zone
//! bookkeeping all run. What it lacks is a socket: [`HarnessClient`] holds the
//! other ends of the player's channels instead, so a test can feed it client
//! packets through the engine's real read path and drain what the engine
//! flushes back.

use crate::active_player::EnginePlayer;
use crate::clients::client_game::create_io;
use crate::engine::{Engine, LoginRequest, engine_mut};
use crate::player_save::PlayerProfile;
use rs_crypto::isaac::{Isaac, IsaacPair};
use rs_entity::{EntityLifeTime, Obj, Player};
use rs_grid::CoordGrid;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptPlayer;
use rs_vm::pointer::ScriptPointer;
use rs_vm::register::OpsRegistry;
use rs_vm::state::{LocRef, ScriptState};
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use std::cell::RefCell;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::sync::mpsc::{Sender, UnboundedReceiver};

/// ISAAC seed for every harness session. The client side keeps its own copy
/// of the engine's decode cipher, so it can mask opcodes the way a real
/// client does.
const SEED: [u32; 4] = [0; 4];

/// Rev-647 `ServerProt.java` lengths by opcode: -1 is var-byte, -2 is var-short.
const SERVER_PROT_SIZES: [i8; 133] = [
    -2, 8, -2, 2, 4, 2, 0, 6, -1, 2, -1, 2, 0, 1, -2, 4, -2, 2, -1, 4, -1, -1, -1, -2, 0, 4, 6, -1,
    3, 4, -1, -2, -1, 20, -2, 4, 4, 8, 7, 1, 1, -2, 3, 6, 4, 12, 4, -1, 10, 6, 8, 8, 3, -2, 6, 6,
    10, 12, 6, 2, 11, 3, 0, 9, -1, 6, 0, 0, 7, 1, -1, -1, 6, -2, 4, 3, 3, -2, 6, 10, 7, -1, 17, -1,
    -1, -1, -1, -2, 6, -2, -2, 6, 5, 10, 6, -1, 6, 3, 6, 3, 1, 6, 28, 3, 1, 5, 0, -2, -1, 7, 2, -2,
    0, -2, 6, 16, 3, -1, 6, -2, 4, 7, -2, 2, 1, -1, 12, 8, 0, 6, 0, 10, 9,
];

thread_local! {
    /// Sessions `setup_player` opened, kept open until the next reset. A
    /// dropped session would read to the engine as a disconnect.
    static SESSIONS: RefCell<Vec<HarnessClient>> = const { RefCell::new(Vec::new()) };
}

/// The client side of a harness player's session.
pub struct HarnessClient {
    pub pid: u16,
    packet_tx: Sender<(std::time::Instant, Vec<u8>)>,
    bytes_rx: UnboundedReceiver<Vec<u8>>,
    _disconnect_tx: Sender<()>,
    /// Mirrors the engine's `isaac_decode`, so opcodes arrive masked as a
    /// client masks them.
    isaac: Isaac,
    /// Mirrors the engine's `isaac_encode`, to unmask what it sends.
    out_isaac: Isaac,
}

impl HarnessClient {
    /// Queues one client packet: the opcode masked with ISAAC, then the
    /// frame's size header if it has one, then the payload.
    ///
    /// The engine reads it on its next input phase, exactly as it reads a
    /// packet off a socket.
    ///
    /// # Panics
    /// If `prot`'s frame is Fixed and `payload` is not that size.
    pub fn send(&mut self, prot: ClientProt, payload: &[u8]) {
        let mut bytes = Vec::with_capacity(payload.len() + 3);
        bytes.push((prot as u8).wrapping_add(self.isaac.next_int() as u8));
        match prot.info().frame {
            ClientProtFrame::Fixed(size) => {
                assert_eq!(payload.len(), size as usize, "{prot:?} is Fixed({size})")
            }
            ClientProtFrame::VarByte { .. } => bytes.push(payload.len() as u8),
            ClientProtFrame::VarShort { .. } => {
                bytes.extend_from_slice(&(payload.len() as u16).to_be_bytes())
            }
        }
        bytes.extend_from_slice(payload);
        self.packet_tx
            .try_send((std::time::Instant::now(), bytes))
            .expect("harness inbox has room");
    }

    /// Every packet the engine has flushed since the last call -- the
    /// session's only reader, since each read advances the ISAAC mirror -- as
    /// `(opcode, payload)`, read the way the 647 client reads them: a 1- or
    /// 2-byte smart opcode unmasked with ISAAC, framed by the client's own
    /// size for that opcode.
    ///
    /// # Panics
    /// On an opcode the client has no prot for: the client would fail too.
    pub fn packets(&mut self) -> Vec<(u16, Vec<u8>)> {
        let mut out = Vec::new();
        // A chunk is one write: during a cycle the engine gathers a player's
        // output and writes it at once, so a chunk can hold many packets.
        // They are read one after another, each framed by its size.
        while let Ok(chunk) = self.bytes_rx.try_recv() {
            let mut pos = 0;
            while pos < chunk.len() {
                let b0 = chunk[pos].wrapping_sub(self.out_isaac.next_int() as u8);
                pos += 1;
                let opcode = if b0 < 128 {
                    b0 as u16
                } else {
                    let b1 = chunk[pos].wrapping_sub(self.out_isaac.next_int() as u8);
                    pos += 1;
                    (((b0 - 128) as u16) << 8) | b1 as u16
                };
                let size = SERVER_PROT_SIZES
                    .get(opcode as usize)
                    .copied()
                    .unwrap_or_else(|| {
                        panic!("the server sent opcode {opcode}, which the client has no prot for")
                    });
                let len = match size {
                    -1 => {
                        pos += 1;
                        chunk[pos - 1] as usize
                    }
                    -2 => {
                        pos += 2;
                        u16::from_be_bytes([chunk[pos - 2], chunk[pos - 1]]) as usize
                    }
                    n => n as usize,
                };
                out.push((opcode, chunk[pos..pos + len].to_vec()));
                pos += len;
            }
        }
        out
    }

    /// The text of every MESSAGE_GAME among `packets`, as the client reads it:
    /// `gsmart type; g4; g1 flags; [names]; gjstr text`.
    ///
    /// Leaves out the VM's debug-build report of a slow script ("Warning [cpu
    /// time]: ..."), which a busy machine sends at random and the game never
    /// says.
    pub fn game_messages(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, String)> {
        let opcode = rs_protocol::network::game::server_prot::ServerProt::MessageGame as u16;
        packets
            .iter()
            .filter(|(op, _)| *op == opcode)
            .map(|(_, p)| {
                let mut pos = 0;
                let kind = if p[0] < 128 {
                    pos += 1;
                    p[0] as i32
                } else {
                    pos += 2;
                    (((p[0] as i32) << 8) | p[1] as i32) - 32768
                };
                pos += 4;
                let flags = p[pos];
                pos += 1;
                let skip_str = |pos: &mut usize| {
                    while p[*pos] != 0 {
                        *pos += 1;
                    }
                    *pos += 1;
                };
                if flags & 1 != 0 {
                    skip_str(&mut pos);
                    if flags & 2 != 0 {
                        skip_str(&mut pos);
                    }
                }
                let end = pos
                    + p[pos..]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(p.len() - pos);
                (kind, String::from_utf8_lossy(&p[pos..end]).into_owned())
            })
            .filter(|(_, text)| !text.starts_with("Warning [cpu time]"))
            .collect()
    }
}

impl Engine {
    /// Logs a player in with no socket behind it.
    ///
    /// Goes through [`accept_login`](Engine::accept_login) with a fresh
    /// profile, as a first login does, so the new-player defaults, `on_login`
    /// and the `[login]` trigger all run.
    ///
    /// A new account's `[login]` opens character creation and stops there:
    /// no game frame, no inventories bound to the backpack. For a player
    /// who is already in the game, use
    /// [`harness_login_returning`](Self::harness_login_returning).
    ///
    /// # Returns
    /// The client side of the session, or `None` when the world is full.
    pub fn harness_login(&mut self, username: &str) -> Option<HarnessClient> {
        self.harness_login_with(username, None)
    }

    /// Logs in a player who finished character creation on an earlier
    /// session, so `[login]` opens the game frame and binds the backpack and
    /// worn inventories, as it does for any returning player.
    ///
    /// Everything else is a new player's: level 1 (10 hitpoints), the
    /// default appearance, empty inventories, at the new-player spawn.
    pub fn harness_login_returning(&mut self, username: &str) -> Option<HarnessClient> {
        let tutorial_done = *self.cache.varps.debugnames.get("learning_the_ropes")?;
        let profile = PlayerProfile {
            x: 3094,
            z: 3106,
            y: 0,
            body: [0, 10, 18, 26, 33, 36, 42],
            colors: [0; 5],
            gender: 0,
            runenergy: 10000,
            playtime: 0,
            stats: [0; rs_entity::build::STAT_COUNT],
            levels: [1; rs_entity::build::STAT_COUNT],
            // `login.rs2` sends anyone under 1000 to character creation.
            varps: vec![(tutorial_done, 1000)],
            invs: Vec::new(),
            afk_zones: [0; 2],
            last_afk_zone: 0,
            public_chat: 0,
            private_chat: 0,
            trade_chat: 0,
            last_date: 0,
            display_name: None,
        };
        self.harness_login_with(username, Some(profile))
    }

    fn harness_login_with(
        &mut self,
        username: &str,
        profile: Option<PlayerProfile>,
    ) -> Option<HarnessClient> {
        let pid = self.player_list.next_pid()?;
        let io = create_io(IsaacPair::new(&SEED, &SEED));
        let (pid_tx, _pid_rx) = tokio::sync::oneshot::channel();
        let request = LoginRequest {
            handle: io.handle,
            username: username.into(),
            password: "".into(),
            low_memory: false,
            remote_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            reconnect: false,
            uid: None,
            device: rs_db::LoginDevice::default(),
            pid_tx,
            // Fixed, the client's default: content opens `toplevel` (548) for
            // window mode 1 and the resizable root for anything else.
            display_mode: 1,
        };
        let engine = self as *mut Engine;
        // `ActivePlayer::new` and the `[login]` trigger read the global
        // engine and cache, which a direct call from a test has not installed.
        rs_vm::engine::with_engine(self, || {
            unsafe { &mut *engine }.accept_login(
                request,
                profile,
                rs_entity::StaffModLevel::Normal,
                None,
                0,
                pid,
                &[],
            )
        });
        Some(HarnessClient {
            pid,
            packet_tx: io.packet_tx,
            bytes_rx: io.bytes_rx,
            _disconnect_tx: io.disconnect_tx,
            isaac: Isaac::new(&SEED),
            out_isaac: Isaac::new(&SEED),
        })
    }

    /// Runs only the input phase: every player's queued packets are decoded
    /// and dispatched, and nothing else happens. A test reads what a handler
    /// set before the player phase acts on it.
    pub fn harness_inputs(&mut self) {
        let engine = self as *mut Engine;
        rs_vm::engine::with_engine(self, || unsafe { &mut *engine }.inputs());
    }

    /// Moves a player as `p_telejump` does: placed, not walked, and the
    /// client's scene rebuilt around the new tile on the next tick.
    pub fn harness_telejump(&mut self, pid: u16, x: u16, y: u8, z: u16) {
        let engine = self as *mut Engine;
        rs_vm::engine::with_engine(self, || {
            if let Some(active) = unsafe { &mut *engine }.get_player_mut(pid) {
                active.tele_jump(CoordGrid::new(x, y, z));
            }
        });
    }

    /// Awards experience as `stat_advance` does (in the engine's tenths): a
    /// level gained queues the stat's `[advancestat]` script.
    pub fn harness_add_xp(&mut self, pid: u16, stat: usize, xp: i32) {
        use rs_vm::engine::ScriptPlayer;
        let engine = self as *mut Engine;
        rs_vm::engine::with_engine(self, || {
            if let Some(active) = unsafe { &mut *engine }.get_player_mut(pid) {
                active.add_xp(stat, xp);
            }
        });
    }

    /// Opens a main-screen modal as `if_openmain` does. It goes out on the
    /// next tick's output phase, as every modal does.
    pub fn harness_open_main(&mut self, pid: u16, iface: u16) {
        self.with_harness_player(pid, |active| active.open_main_modal(iface));
    }

    /// Opens a side modal as `if_openside` does.
    pub fn harness_open_side(&mut self, pid: u16, iface: u16) {
        self.with_harness_player(pid, |active| active.open_side_modal(iface));
    }

    /// Closes the player's modals as `if_close` does.
    pub fn harness_close_modal(&mut self, pid: u16) {
        self.with_harness_player(pid, |active| {
            let _ = active.close_modal(true);
        });
    }

    /// Closes whatever is mounted on `parent`, as `if_closesub` does.
    pub fn harness_close_sub(&mut self, pid: u16, parent: i32) {
        self.with_harness_player(pid, |active| active.if_close_sub(parent));
    }

    fn with_harness_player(
        &mut self,
        pid: u16,
        f: impl FnOnce(&mut crate::active_player::ActivePlayer),
    ) {
        let engine = self as *mut Engine;
        rs_vm::engine::with_engine(self, || {
            if let Some(active) = unsafe { &mut *engine }.get_player_mut(pid) {
                f(active);
            }
        });
    }

    /// Drops an obj on the ground for everyone, as a map spawn would be.
    pub fn harness_add_obj(&mut self, x: u16, y: u8, z: u16, id: u16, count: u32) {
        let coord = CoordGrid::new(x, y, z);
        let engine = self as *mut Engine;
        rs_vm::engine::with_engine(self, || {
            let obj = Obj::new(coord, EntityLifeTime::Despawn, id, count);
            unsafe { &mut *engine }.add_obj(coord, obj, None, 200);
        });
    }

    /// Registers the `#testscript` commands on this engine and turns the
    /// call log on. Without this, a test command is an unknown opcode.
    pub fn harness_enable_tests(&mut self) {
        self.ops.extend(ops());
        rs_vm::ops::test::set_recording(true);
    }

    /// Returns the world to its just-started state between tests: every
    /// player out, the script and delayed-obj queues empty, zone traffic and
    /// shared inventories gone, and the clock back at 0.
    ///
    /// Npcs keep their spawns but lose any script they were suspended in.
    /// Locs a test changed revert on their own timers, or stay changed; a
    /// test that depends on a loc's state sets it up rather than assuming it.
    pub fn harness_reset(&mut self) {
        for pid in self.player_list.pids() {
            self.remove_player(pid);
        }
        SESSIONS.with(|s| s.borrow_mut().clear());
        self.world_queue.clear();
        self.obj_delayed_queue.clear();
        self.zones_tracking.clear();
        self.pending_zone_events.clear();
        self.invs.clear();
        self.pending_logins.clear();
        for npc in self.npc_list.npcs.iter_mut().flatten() {
            npc.npc.state.active_script = None;
        }
        self.clock = 0;
    }

    /// The entity state of a logged-in player, for assertions.
    pub fn harness_player(&self, pid: u16) -> Option<&Player> {
        self.get_player(pid).map(|active| &active.player)
    }

    /// Mutable entity state of a logged-in player, for test setup.
    pub fn harness_player_mut(&mut self, pid: u16) -> Option<&mut Player> {
        self.get_player_mut(pid).map(|active| &mut active.player)
    }
}

/// What a trigger's subject is, which decides how `test_op` resolves it.
enum Subject {
    Loc,
    Npc,
    Other,
}

fn subject_of(trigger: ServerTriggerType) -> Subject {
    use ServerTriggerType::*;
    match trigger {
        ApLoc1 | ApLoc2 | ApLoc3 | ApLoc4 | ApLoc5 | ApLocU | ApLocT | OpLoc1 | OpLoc2 | OpLoc3
        | OpLoc4 | OpLoc5 | OpLocU | OpLocT => Subject::Loc,
        ApNpc1 | ApNpc2 | ApNpc3 | ApNpc4 | ApNpc5 | ApNpcU | ApNpcT | OpNpc1 | OpNpc2 | OpNpc3
        | OpNpc4 | OpNpc5 | OpNpcU | OpNpcT => Subject::Npc,
        _ => Subject::Other,
    }
}

/// The test commands that need a live engine, plus every engine-independent
/// one from [`rs_vm::ops::test`].
///
/// # Opcodes Registered
///
/// - `SETUP_PLAYER` -- logs a harness player in and makes it the script's
///   active player.
/// - `TEST_OP` -- fires a trigger against a loc at a coord, or the first npc
///   of a type, with the active player as subject.
pub fn ops() -> OpsRegistry {
    let mut ops = rs_vm::ops::test::build();
    ops.insert(rs_pack::cache::script::SETUP_PLAYER, setup_player);
    ops.insert(rs_pack::cache::script::TEST_OP, test_op);
    ops
}

// 50100
fn setup_player(s: &mut ScriptState) -> Result<(), ScriptError> {
    let username = s.pop_string();
    let engine = engine_mut();
    let client = engine
        .harness_login(&username)
        .ok_or_else(|| ScriptError::Runtime("setup_player: no free player slot".into()))?;
    let uid = engine
        .get_player(client.pid)
        .map(|active| active.uid())
        .ok_or_else(|| ScriptError::Runtime("setup_player: login did not land".into()))?;
    SESSIONS.with(|s| s.borrow_mut().push(client));
    rs_vm::ops::test::forget_calls();
    s.active_player = Some(uid);
    s.pointers.add(ScriptPointer::ActivePlayer);
    s.pointers.add(ScriptPointer::ProtectedActivePlayer);
    s.push_int(uid.packed() as i32);
    Ok(())
}

// 50055
fn test_op(s: &mut ScriptState) -> Result<(), ScriptError> {
    let coord = s.pop_int() as u32;
    let subject_id = s.pop_int();
    let trigger_byte = s.pop_int();
    let trigger = ServerTriggerType::try_from(trigger_byte as u8)
        .map_err(|_| ScriptError::Runtime(format!("test_op: bad trigger {trigger_byte}")))?;
    let player = s
        .active_player
        .ok_or_else(|| ScriptError::Runtime("test_op: no active player".into()))?;
    let engine = engine_mut();
    let cache = engine.cache;
    let grid = CoordGrid::from(coord);

    let (target, category) = match subject_of(trigger) {
        Subject::Loc => {
            let id = subject_id as u16;
            let zone = engine.zones.zone(grid.x(), grid.y(), grid.z());
            let loc = zone
                .and_then(|z| z.get_loc(grid.x(), grid.z(), id).map(|i| z.locs[i]))
                .ok_or_else(|| {
                    ScriptError::Runtime(format!(
                        "test_op: loc {id} not at {}_{}_{}_{}_{}",
                        grid.y(),
                        grid.x() >> 6,
                        grid.z() >> 6,
                        grid.x() & 63,
                        grid.z() & 63
                    ))
                })?;
            let target = ScriptSubject::Loc(LocRef {
                coord: grid,
                id,
                shape: loc.shape(),
                angle: loc.angle(),
                layer: loc.layer(),
                serial: loc.serial(),
            });
            let category = cache.locs.get_by_id(id).and_then(|t| t.category);
            (Some(target), category.map(|c| c as i32))
        }
        Subject::Npc => {
            let id = subject_id as u16;
            let npc = engine
                .npc_list
                .npcs
                .iter()
                .flatten()
                .find(|n| n.npc.uid.id() == id)
                .map(|n| n.npc.uid)
                .ok_or_else(|| ScriptError::Runtime(format!("test_op: no npc of type {id}")))?;
            let category = cache.npcs.get_by_id(id).and_then(|t| t.category);
            (Some(ScriptSubject::Npc(npc)), category.map(|c| c as i32))
        }
        Subject::Other => (None, None),
    };

    let type_id = (subject_id >= 0).then_some(subject_id);
    match engine.run_script_by_trigger(
        (trigger, type_id, category),
        Some(ScriptSubject::Player(player)),
        target,
        Some(true),
        Some(true),
        None,
    ) {
        // An op nothing answers is what a test of a missing trigger checks
        // for, through the absence of what the trigger would have done.
        Err(ScriptError::TriggerNotFound(_)) => Ok(()),
        other => other,
    }
}
