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

use crate::active_player::{ActivePlayer, EnginePlayer};
use crate::clients::client_db::{profile_from_save, to_save};
use crate::clients::client_game::create_io;
use crate::core::{Engine, LoginRequest, engine_mut};
use crate::handlers::client_cheat::spawn_bots;
#[cfg(since_647)]
use crate::phases::controller::Controllers;
use crate::region::{login_coord, place_returning};
use crate::save::{PlayerProfile, load_binary, save_binary};
use rs_crypto::isaac::{Isaac, IsaacPair};
use rs_db::LoginDevice;
use rs_entity::build::STAT_COUNT;
use rs_entity::{EntityLifeTime, Obj, Player, StaffModLevel};
use rs_grid::CoordGrid;
use rs_io::Packet;
use rs_protocol::network::game::client::ClientProtEncode;
use rs_protocol::network::game::client::apcoordt::ApCoordT;
use rs_protocol::network::game::client::client_cheat::ClientCheat;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::if_button::IfButton;
use rs_protocol::network::game::client::if_button1::IfButton1;
use rs_protocol::network::game::client::if_button2::IfButton2;
use rs_protocol::network::game::client::if_button3::IfButton3;
use rs_protocol::network::game::client::if_button4::IfButton4;
use rs_protocol::network::game::client::if_button5::IfButton5;
use rs_protocol::network::game::client::if_button6::IfButton6;
use rs_protocol::network::game::client::if_button7::IfButton7;
use rs_protocol::network::game::client::if_button8::IfButton8;
use rs_protocol::network::game::client::if_button9::IfButton9;
use rs_protocol::network::game::client::if_button10::IfButton10;
use rs_protocol::network::game::client::if_buttond::IfButtonD;
use rs_protocol::network::game::client::if_buttont::IfButtonT;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::inv_buttond::InvButtonD;
use rs_protocol::network::game::client::move_gameclick::MoveGameClick;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opheld1::OpHeld1;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opheld2::OpHeld2;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opheld3::OpHeld3;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opheld4::OpHeld4;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opheld5::OpHeld5;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opheldt::OpHeldT;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opheldu::OpHeldU;
use rs_protocol::network::game::client::oploc1::OpLoc1;
use rs_protocol::network::game::client::oploc2::OpLoc2;
use rs_protocol::network::game::client::oploct::OpLocT;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::oplocu::OpLocU;
use rs_protocol::network::game::client::opnpc1::OpNpc1;
use rs_protocol::network::game::client::opnpc2::OpNpc2;
use rs_protocol::network::game::client::opnpc3::OpNpc3;
use rs_protocol::network::game::client::opnpc4::OpNpc4;
use rs_protocol::network::game::client::opnpc5::OpNpc5;
use rs_protocol::network::game::client::opnpct::OpNpcT;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opnpcu::OpNpcU;
use rs_protocol::network::game::client::opobj3::OpObj3;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opobj6::OpObj6;
use rs_protocol::network::game::client::opobjt::OpObjT;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opobju::OpObjU;
use rs_protocol::network::game::client::opplayer4::OpPlayer4;
use rs_protocol::network::game::client::opplayer6::OpPlayer6;
use rs_protocol::network::game::client::opplayer7::OpPlayer7;
use rs_protocol::network::game::client::opplayer8::OpPlayer8;
#[cfg(since_647)]
use rs_protocol::network::game::client::opplayer9::OpPlayer9;
#[cfg(since_647)]
use rs_protocol::network::game::client::opplayer10::OpPlayer10;
use rs_protocol::network::game::client::opplayert::OpPlayerT;
#[cfg(all(since_530, before_647))]
use rs_protocol::network::game::client::opplayeru::OpPlayerU;
#[cfg(before_647)]
use rs_protocol::network::game::client::pack_coord;
use rs_protocol::network::game::client::resume_p_countdialog::ResumePCountDialog;
use rs_protocol::network::game::client::resume_pause_button::ResumePauseButton;
use rs_protocol::network::game::client::url_request::UrlRequest;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_protocol::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use rs_var::VarSet;
use rs_ver::TypeId;
use rs_vm::engine::{ScriptPlayer, with_engine};
use rs_vm::ops::test;
use rs_vm::pointer::ScriptPointer;
use rs_vm::register::OpsRegistry;
use rs_vm::state::{LocRef, ScriptState};
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{RegionUid, ScriptError};
use runec::opcodes::{SETUP_PLAYER, TEST_OP};
use std::cell::RefCell;
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Instant;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};

/// ISAAC seed for every harness session. The client side keeps its own copy
/// of the engine's decode cipher, so it can mask opcodes the way a real
/// client does.
const SEED: [u32; 4] = [0; 4];

/// Rev-530 client (`rg.i`) lengths by opcode: -1 is var-byte, -2 is var-short.
#[cfg(all(since_530, before_647))]
const SERVER_PROT_SIZES: [i8; 256] = [
    -1, 0, 8, 0, 2, 0, 0, 0, 0, 12, 0, 1, 0, 3, 7, 0, 15, 6, 0, 0, 4, 7, -2, -1, 2, 0, 2, 8, 0, 0,
    0, 0, -2, 5, 0, 0, 8, 3, 6, 0, 0, 0, -1, 0, -1, 0, 0, 6, -2, 0, 12, 0, 0, 0, -1, -2, 10, 0, 0,
    0, 3, 0, -1, 0, 0, 5, 6, 0, 0, 8, -1, -1, 0, 8, 0, 0, 0, 0, 0, 0, 0, -1, 0, 0, 6, 2, 0, 0, 0,
    0, 1, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 5, 0, 0, -2, 0, 0, 0, 0, 0, 12, 2, 0, -2, -2, 20, 0, 0,
    10, 0, 15, 0, -1, 0, 8, -2, 0, 0, 0, 8, 0, 12, 0, 0, 7, 0, 0, 0, 0, 0, -1, -1, 0, 4, 5, 0, 0,
    0, 6, 0, 0, 0, 0, 8, 9, 0, 0, 0, 2, -1, 0, -2, 0, 4, 14, 0, 0, 0, 24, 0, -2, 5, 0, 0, 0, 10, 0,
    0, 4, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 2, 1, 0, 0, 2, -1, 1, 0, 0, 0, 0, 14, 0, 0, 0, 0, 10, 5,
    0, 0, 0, 0, 0, -2, 0, 0, 9, 0, 0, 8, 0, 0, 0, 0, -2, 6, 0, 0, 0, -2, 0, 3, 0, 1, 7, 0, 0, 0, 0,
    3, 0, 0, 0, 0, 0, 0, -1, 0, 0, 0, 0, 0, 3, 0, 0,
];

/// Rev-647 `ServerProt.java` lengths by opcode: -1 is var-byte, -2 is var-short.
#[cfg(all(since_647, before_742))]
const SERVER_PROT_SIZES: [i8; 133] = [
    -2, 8, -2, 2, 4, 2, 0, 6, -1, 2, -1, 2, 0, 1, -2, 4, -2, 2, -1, 4, -1, -1, -1, -2, 0, 4, 6, -1,
    3, 4, -1, -2, -1, 20, -2, 4, 4, 8, 7, 1, 1, -2, 3, 6, 4, 12, 4, -1, 10, 6, 8, 8, 3, -2, 6, 6,
    10, 12, 6, 2, 11, 3, 0, 9, -1, 6, 0, 0, 7, 1, -1, -1, 6, -2, 4, 3, 3, -2, 6, 10, 7, -1, 17, -1,
    -1, -1, -1, -2, 6, -2, -2, 6, 5, 10, 6, -1, 6, 3, 6, 3, 1, 6, 28, 3, 1, 5, 0, -2, -1, 7, 2, -2,
    0, -2, 6, 16, 3, -1, 6, -2, 4, 7, -2, 2, 1, -1, 12, 8, 0, 6, 0, 10, 9,
];

/// Rev-742 client (`jl`) lengths by opcode: -1 is var-byte, -2 is var-short.
#[cfg(since_742)]
const SERVER_PROT_SIZES: [i8; 163] = [
    8, -2, -1, 10, 6, 10, 3, 32, -1, 0, 3, 2, 0, 8, 8, 8, 8, 8, 0, 0, 10, 0, 3, -2, -2, 6, 6, 0, 0,
    -2, 5, -1, 1, 3, 0, 0, -1, 2, 4, 4, 1, 7, 10, 6, -1, 19, -1, 23, 0, 4, -1, 5, 2, 6, 2, -2, 4,
    -1, -1, -2, -1, -2, 3, 4, 25, -2, 6, -2, 20, 1, 3, 3, -2, 1, 14, 2, 1, 19, 8, 5, -1, 4, 6, 5,
    9, 12, -1, 0, 28, -2, -2, 6, 2, -2, -2, 6, 6, 8, -2, 6, 8, 2, 4, 9, 2, 10, -2, -2, 6, 6, 29, 1,
    5, 5, -2, 10, -1, 11, 12, 6, -1, -1, 2, -2, 1, 7, 10, 18, 3, 9, -2, -1, -1, 10, 8, 0, -2, -2,
    0, -1, -2, 21, 4, -2, -1, 2, -1, 2, 4, -2, 8, -1, 6, -2, 1, 22, 3, 6, 1, 1, 25, -2, -2,
];

thread_local! {
    /// Sessions `setup_player` opened, kept open until the next reset. A
    /// dropped session would read to the engine as a disconnect.
    static SESSIONS: RefCell<Vec<HarnessClient>> = const { RefCell::new(Vec::new()) };
}

/// The client side of a harness player's session.
pub struct HarnessClient {
    pub pid: u16,
    packet_tx: Sender<(Instant, Vec<u8>)>,
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
        bytes.push((prot as u8).wrapping_add(self.isaac.take_next_value() as u8));
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
            .try_send((Instant::now(), bytes))
            .expect("harness inbox has room");
    }

    /// Every packet the engine has flushed since the last call -- the
    /// session's only reader, since each read advances the ISAAC mirror -- as
    /// `(opcode, payload)`, read the way the client reads them: the opcode
    /// unmasked with ISAAC (one byte on 530, a 1- or 2-byte smart from 647),
    /// framed by the client's own size for that opcode.
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
                let b0 = chunk[pos].wrapping_sub(self.out_isaac.take_next_value() as u8);
                pos += 1;
                #[cfg(all(since_530, before_647))]
                let opcode = b0 as u16;
                #[cfg(since_647)]
                let opcode = if b0 < 128 {
                    b0 as u16
                } else {
                    let b1 = chunk[pos].wrapping_sub(self.out_isaac.take_next_value() as u8);
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
    /// `gsmart type; g4; g1 flags; [names]; gjstr text` on 647, and `gjstr
    /// text` with type 0 on 530.
    ///
    /// Leaves out the VM's debug-build report of a slow script ("Warning [cpu
    /// time]: ..."), which a busy machine sends at random and the game never
    /// says.
    pub fn game_messages(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, String)> {
        let opcode = ServerProt::MessageGame as u16;
        packets
            .iter()
            .filter(|(op, _)| *op == opcode)
            .map(|(_, p)| {
                #[cfg(all(since_530, before_647))]
                let (kind, pos) = (0, 0);
                #[cfg(since_647)]
                let (kind, pos) = {
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
                    (kind, pos)
                };
                let end = pos
                    + p[pos..]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(p.len() - pos);
                (kind, String::from_utf8_lossy(&p[pos..end]).into_owned())
            })
            // A debug build's cpu-time warning is wrapped at the chat box's width: its
            // lines run from "Warning [cpu time]" to the one with "opcount:".
            .filter({
                let mut warning = false;
                move |(_, text)| {
                    if text.starts_with("Warning [cpu time]") {
                        warning = true;
                    }
                    let keep = !warning;
                    if warning && text.contains("opcount:") {
                        warning = false;
                    }
                    keep
                }
            })
            .collect()
    }
}

/// 530's backpack, `inventory:inv`, is an if1 inventory: its client sends an
/// obj's held ops (OPHELD), its uses (OPHELDU, OP*U) and its drags
/// (INV_BUTTOND) for it, where 647 sends the IF_BUTTON family.
#[cfg(all(since_530, before_647))]
const IF1_BACKPACK: i32 = 149 << 16;

/// Client messages as each rev's client writes them. The tests name what the
/// player did; the rev's own fields (647's `ctrl`, the held obj of a targeted
/// op) are filled in the way an ordinary click fills them.
impl HarnessClient {
    /// Sends `msg` the way the client writes it.
    pub fn send_msg<M: ClientProtEncode>(&mut self, msg: &M) {
        let mut buf = Packet::new(5000);
        msg.encode(&mut buf);
        self.send(M::PROT, &buf.data[..buf.pos]);
    }

    /// OPLOC`op` on `loc` at (`x`, `z`).
    pub fn op_loc(&mut self, op: u8, x: u16, z: u16, loc: TypeId) {
        match op {
            1 => self.send_msg(&OpLoc1 {
                x,
                z,
                loc,
                #[cfg(since_647)]
                ctrl: false,
            }),
            2 => self.send_msg(&OpLoc2 {
                x,
                z,
                loc,
                #[cfg(since_647)]
                ctrl: false,
            }),
            _ => panic!("the harness has no OPLOC{op}"),
        }
    }

    /// OPLOCT: the component `com` (slot `slot`, holding `obj`) used on `loc`.
    pub fn op_loc_t(&mut self, x: u16, z: u16, loc: TypeId, com: i32, slot: u16, obj: TypeId) {
        #[cfg(all(since_530, before_647))]
        if com == IF1_BACKPACK {
            return self.send_msg(&OpLocU {
                x,
                z,
                loc,
                obj,
                slot,
                com,
            });
        }
        #[cfg(before_647)]
        let _ = obj;
        self.send_msg(&OpLocT {
            loc,
            x,
            z,
            #[cfg(since_647)]
            obj,
            slot,
            com,
            #[cfg(since_647)]
            ctrl: false,
        });
    }

    /// OPNPC`op` on npc `nid`.
    pub fn op_npc(&mut self, op: u8, nid: u16) {
        match op {
            1 => self.send_msg(&OpNpc1 {
                nid,
                #[cfg(since_647)]
                ctrl: false,
            }),
            2 => self.send_msg(&OpNpc2 {
                nid,
                #[cfg(since_647)]
                ctrl: false,
            }),
            3 => self.send_msg(&OpNpc3 {
                nid,
                #[cfg(since_647)]
                ctrl: false,
            }),
            4 => self.send_msg(&OpNpc4 {
                nid,
                #[cfg(since_647)]
                ctrl: false,
            }),
            5 => self.send_msg(&OpNpc5 {
                nid,
                #[cfg(since_647)]
                ctrl: false,
            }),
            _ => panic!("the harness has no OPNPC{op}"),
        }
    }

    /// OPNPCT: the component `com` (slot `slot`, holding `obj`) used on npc `nid`.
    pub fn op_npc_t(&mut self, nid: u16, com: i32, slot: u16, obj: TypeId) {
        #[cfg(all(since_530, before_647))]
        if com == IF1_BACKPACK {
            return self.send_msg(&OpNpcU {
                nid,
                obj,
                slot,
                com,
            });
        }
        #[cfg(before_647)]
        let _ = obj;
        self.send_msg(&OpNpcT {
            nid,
            #[cfg(since_647)]
            obj,
            slot,
            com,
            #[cfg(since_647)]
            ctrl: false,
        });
    }

    /// OPOBJ`op` on the ground obj `obj` at (`x`, `z`).
    pub fn op_obj(&mut self, op: u8, x: u16, z: u16, obj: TypeId) {
        match op {
            3 => self.send_msg(&OpObj3 {
                x,
                z,
                obj,
                #[cfg(since_647)]
                ctrl: false,
            }),
            _ => panic!("the harness has no OPOBJ{op}"),
        }
    }

    /// OPOBJT: the component `com` (slot `slot`, holding `held`) used on the
    /// ground obj `obj` at (`x`, `z`).
    pub fn op_obj_t(&mut self, x: u16, z: u16, obj: TypeId, com: i32, slot: u16, held: TypeId) {
        #[cfg(all(since_530, before_647))]
        if com == IF1_BACKPACK {
            return self.send_msg(&OpObjU {
                x,
                z,
                obj,
                use_obj: held,
                slot,
                com,
            });
        }
        #[cfg(before_647)]
        let _ = held;
        self.send_msg(&OpObjT {
            obj,
            x,
            z,
            #[cfg(since_647)]
            obj2: held,
            slot,
            com,
            #[cfg(since_647)]
            ctrl: false,
        });
    }

    /// OPPLAYER`op` on player `pid`.
    pub fn op_player(&mut self, op: u8, pid: u16) {
        macro_rules! op {
            ($ty:ident) => {
                self.send_msg(&$ty {
                    pid,
                    #[cfg(since_647)]
                    ctrl: false,
                })
            };
        }
        match op {
            4 => op!(OpPlayer4),
            6 => op!(OpPlayer6),
            7 => op!(OpPlayer7),
            8 => op!(OpPlayer8),
            #[cfg(since_647)]
            9 => op!(OpPlayer9),
            #[cfg(since_647)]
            10 => op!(OpPlayer10),
            _ => panic!("the harness has no OPPLAYER{op}"),
        }
    }

    /// OPPLAYERT: the component `com` (slot `slot`, holding `obj`) used on player `pid`.
    pub fn op_player_t(&mut self, pid: u16, com: i32, slot: u16, obj: TypeId) {
        #[cfg(all(since_530, before_647))]
        if com == IF1_BACKPACK {
            return self.send_msg(&OpPlayerU {
                pid,
                obj,
                slot,
                com,
            });
        }
        #[cfg(before_647)]
        let _ = obj;
        self.send_msg(&OpPlayerT {
            pid,
            #[cfg(since_647)]
            obj,
            slot,
            com,
            #[cfg(since_647)]
            ctrl: false,
        });
    }

    /// A click on an if1 component's button (IF_BUTTON), as 530's client sends for one with a
    /// button type.
    #[cfg(all(since_530, before_647))]
    pub fn if1_button(&mut self, com: i32) {
        self.send_msg(&IfButton { com });
    }

    /// IF_BUTTON`op` on component `com`, slot `slot` (holding `obj`, from 647).
    pub fn if_button(&mut self, op: u8, com: i32, slot: u16, obj: TypeId) {
        // 647's backpack puts the obj's iops 1-3 on buttons 1-3, iop4 on 7,
        // iop5 on 8 and examine on 10.
        #[cfg(all(since_530, before_647))]
        if com == IF1_BACKPACK {
            return match op {
                1 => self.send_msg(&OpHeld1 { obj, slot, com }),
                2 => self.send_msg(&OpHeld2 { obj, slot, com }),
                3 => self.send_msg(&OpHeld3 { obj, slot, com }),
                7 => self.send_msg(&OpHeld4 { obj, slot, com }),
                8 => self.send_msg(&OpHeld5 { obj, slot, com }),
                10 => self.send_msg(&OpObj6 { obj }),
                _ => panic!("530's backpack has no op for IF_BUTTON{op}"),
            };
        }
        #[cfg(before_647)]
        let _ = obj;
        macro_rules! button {
            ($ty:ident) => {
                self.send_msg(&$ty {
                    com,
                    slot,
                    #[cfg(since_647)]
                    obj,
                })
            };
        }
        match op {
            1 => button!(IfButton1),
            2 => button!(IfButton2),
            3 => button!(IfButton3),
            4 => button!(IfButton4),
            5 => button!(IfButton5),
            6 => button!(IfButton6),
            7 => button!(IfButton7),
            8 => button!(IfButton8),
            9 => button!(IfButton9),
            10 => button!(IfButton10),
            _ => panic!("the harness has no IF_BUTTON{op}"),
        }
    }

    /// IF_BUTTONT: component `com` (slot `slot`, `obj`) used on component
    /// `com2` (slot `slot2`, `obj2`).
    pub fn if_button_t(
        &mut self,
        com: i32,
        slot: u16,
        obj: TypeId,
        com2: i32,
        slot2: u16,
        obj2: TypeId,
    ) {
        // A spell on a backpack obj is OPHELDT, the obj first.
        #[cfg(all(since_530, before_647))]
        if com != IF1_BACKPACK && com2 == IF1_BACKPACK {
            return self.send_msg(&OpHeldT {
                obj: obj2,
                slot: slot2,
                com: com2,
                com2: com,
                slot2: slot,
            });
        }
        #[cfg(all(since_530, before_647))]
        if com == IF1_BACKPACK && com2 == IF1_BACKPACK {
            return self.send_msg(&OpHeldU {
                obj,
                slot,
                com,
                obj2,
                slot2,
                com2,
            });
        }
        #[cfg(before_647)]
        let _ = (obj, obj2);
        self.send_msg(&IfButtonT {
            com,
            slot,
            #[cfg(since_647)]
            obj,
            com2,
            slot2,
            #[cfg(since_647)]
            obj2,
        });
    }

    /// IF_BUTTOND: slot `slot` of `com` (holding `obj`) dragged onto slot
    /// `slot2` of `com2` (holding `obj2`).
    pub fn if_button_d(
        &mut self,
        com: i32,
        slot: u16,
        obj: TypeId,
        com2: i32,
        slot2: u16,
        obj2: TypeId,
    ) {
        #[cfg(all(since_530, before_647))]
        if com == IF1_BACKPACK && com2 == IF1_BACKPACK {
            return self.send_msg(&InvButtonD {
                com,
                slot,
                slot2,
                mode: 0,
            });
        }
        #[cfg(before_647)]
        let _ = (obj, obj2);
        self.send_msg(&IfButtonD {
            com,
            slot,
            #[cfg(since_647)]
            obj,
            com2,
            slot2,
            #[cfg(since_647)]
            obj2,
        });
    }

    /// APCOORDT: the component `com` (slot `slot`, holding `obj`) used on the
    /// tile (`x`, `z`).
    pub fn ap_coord_t(&mut self, x: u16, z: u16, com: i32, slot: u16, obj: TypeId) {
        #[cfg(before_647)]
        let _ = obj;
        self.send_msg(&ApCoordT {
            x,
            z,
            #[cfg(since_647)]
            obj,
            slot,
            com,
        });
    }

    /// RESUME_PAUSEBUTTON on component `com`, slot `slot`.
    pub fn resume_pausebutton(&mut self, com: i32, slot: u16) {
        self.send_msg(&ResumePauseButton { com, slot });
    }

    /// RESUME_P_COUNTDIALOG with `input`.
    pub fn resume_p_countdialog(&mut self, input: i32) {
        self.send_msg(&ResumePCountDialog { input });
    }

    /// MOVE_GAMECLICK to (`x`, `z`): the client sends the destination, and its
    /// own path only before 647.
    pub fn move_gameclick(&mut self, x: u16, z: u16) {
        #[cfg(before_647)]
        self.send_msg(&MoveGameClick {
            path: vec![pack_coord(x, z)],
            ctrl: false,
        });
        #[cfg(since_647)]
        self.send_msg(&MoveGameClick { x, z, ctrl: false });
    }

    /// CLIENT_CHEAT `cheat`, as typed in the chat box (`::` stripped).
    pub fn client_cheat(&mut self, cheat: &str) {
        self.send_msg(&ClientCheat {
            cheat: cheat.into(),
            #[cfg(since_647)]
            flags: [0, 0],
        });
    }

    /// URL_REQUEST for `url`, the link's `text` and its `mode`.
    pub fn url_request(&mut self, url: &str, text: &str, mode: u8) {
        self.send_msg(&UrlRequest {
            url: url.into(),
            text: text.into(),
            #[cfg(since_742)]
            extra: String::new(),
            mode,
        });
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
        self.harness_login_returning_at(username, 3094, 3106)
    }

    /// [`harness_login_returning`](Self::harness_login_returning), logging in
    /// at `x`, `z` on level 0 instead of the new-player spawn, for content
    /// that `[login]` decides from where the player stands.
    pub fn harness_login_returning_at(
        &mut self,
        username: &str,
        x: u16,
        z: u16,
    ) -> Option<HarnessClient> {
        let profile = self.returning_profile(CoordGrid::new(x, 0, z), None)?;
        self.harness_login_with(username, Some(profile))
    }

    /// A returning player whose save holds `coord` and `exit`, as one saved
    /// inside a region does.
    pub fn harness_login_saved_at(
        &mut self,
        username: &str,
        coord: CoordGrid,
        exit: Option<CoordGrid>,
    ) -> Option<HarnessClient> {
        let profile = self.returning_profile(coord, exit)?;
        self.harness_login_with(username, Some(profile))
    }

    /// A save at `coord` with `exit`, of a player past the tutorial with
    /// default appearance and empty inventories.
    fn returning_profile(
        &self,
        coord: CoordGrid,
        exit: Option<CoordGrid>,
    ) -> Option<PlayerProfile> {
        let tutorial_done = self.cache.varps.get_by_debugname("tutorial")?.id;
        Some(PlayerProfile {
            x: coord.x(),
            z: coord.z(),
            y: coord.y(),
            body: [0, 10, 18, 26, 33, 36, 42],
            colors: [0; 5],
            gender: 0,
            runenergy: 10000,
            playtime: 0,
            stats: [0; STAT_COUNT],
            levels: [1; STAT_COUNT],
            // `login.rs2` sends anyone under 1000 to character creation.
            varps: vec![(tutorial_done.g2(), 1000)],
            invs: Vec::new(),
            afk_zones: [0; 2],
            last_afk_zone: 0,
            public_chat: 0,
            private_chat: 0,
            trade_chat: 0,
            last_date: 0,
            exit,
            #[cfg(since_647)]
            display_name: None,
        })
    }

    fn harness_login_with(
        &mut self,
        username: &str,
        mut profile: Option<PlayerProfile>,
    ) -> Option<HarnessClient> {
        // Placed as a login from the database is, before anything is sent.
        if let Some(profile) = profile.as_mut() {
            place_returning(profile);
        }
        let pid = self.player_list.next_pid()?;
        let mut io = create_io(IsaacPair::new(&SEED, &SEED));
        let request = LoginRequest {
            handle: io.handle,
            username: username.into(),
            password: "".into(),
            low_memory: false,
            remote_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            reconnect: false,
            uid: None,
            device: LoginDevice::default(),
            // Fixed, the client's default: content opens `toplevel` (548) for
            // window mode 1 and the resizable root for anything else.
            display_mode: 1,
            #[cfg(since_647)]
            lobby: false,
            #[cfg(since_647)]
            world: 1,
        };
        let engine = self as *mut Engine;
        // `ActivePlayer::new` and the `[login]` trigger read the global
        // engine and cache, which a direct call from a test has not installed.
        with_engine(self, || {
            unsafe { &mut *engine }.accept_login(
                request,
                profile,
                StaffModLevel::Normal,
                None,
                0,
                pid,
                &[],
            )
        });
        let reply = io.bytes_rx.try_recv().expect("login response");
        assert_eq!(reply[0], 2);
        #[cfg(all(since_530, before_647))]
        {
            assert_eq!(reply[1], 0);
            assert_eq!(reply.len(), 12);
        }
        #[cfg(since_647)]
        {
            assert_eq!(reply[2], 0);
            assert_eq!(reply.len(), 2 + reply[1] as usize);
        }
        assert_eq!(u16::from_be_bytes([reply[8], reply[9]]), pid);
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
        with_engine(self, || unsafe { &mut *engine }.inputs());
    }

    pub fn harness_local_bots(&mut self, coord: CoordGrid, count: u16) -> u16 {
        with_engine(self, || spawn_bots(coord, count, 15))
    }

    /// Moves a player as `p_telejump` does: placed, not walked, and the
    /// client's scene rebuilt around the new tile on the next tick.
    pub fn harness_telejump(&mut self, pid: u16, x: u16, y: u8, z: u16) {
        let engine = self as *mut Engine;
        with_engine(self, || {
            let engine = unsafe { &mut *engine };
            if let Some(active) = engine.get_player_mut(pid) {
                active.tele_jump(CoordGrid::new(x, y, z));
            }
        });
    }

    /// Awards experience as `stat_advance` does (in the engine's tenths): a
    /// level gained queues the stat's `[advancestat]` script.
    pub fn harness_add_xp(&mut self, pid: u16, stat: usize, xp: i32) {
        let engine = self as *mut Engine;
        with_engine(self, || {
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

    fn with_harness_player(&mut self, pid: u16, f: impl FnOnce(&mut ActivePlayer)) {
        let engine = self as *mut Engine;
        with_engine(self, || {
            if let Some(active) = unsafe { &mut *engine }.get_player_mut(pid) {
                f(active);
            }
        });
    }

    /// Drops an obj on the ground for everyone, as a map spawn would be.
    pub fn harness_add_obj(&mut self, x: u16, y: u8, z: u16, id: TypeId, count: u32) {
        let coord = CoordGrid::new(x, y, z);
        let engine = self as *mut Engine;
        with_engine(self, || {
            let obj = Obj::new(coord, EntityLifeTime::Despawn, id, count);
            unsafe { &mut *engine }.add_obj(coord, obj, None, 200);
        });
    }

    /// Registers the `#testscript` commands on this engine and turns the
    /// call log on. Without this, a test command is an unknown opcode.
    ///
    /// Also stops the 500-tick AFK random-event roll: the reset clock starts
    /// at 0, so every test's first tick would roll it, and skilling scripts
    /// then spawn an event instead of their product.
    pub fn harness_enable_tests(&mut self) {
        self.ops.extend(ops());
        test::set_recording(true);
        self.afk_events = false;
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
        self.pending_zone_events.clear();
        self.invs.clear();
        self.pending_logins.clear();
        for npc in self.npc_list.npcs.iter_mut().flatten() {
            npc.npc.state.active_script = None;
            // The clock goes back to 0 below, so an npc's clock-valued vars from an earlier
            // test (its last fight, its action delay, whom it fought) would read as the
            // future: its single-combat lock held until the clock caught up ("Someone else
            // is fighting that."). Its vars start fresh, as at its spawn.
            #[cfg(since_530)]
            {
                npc.npc.vars = VarSet::new(self.cache.varns.types.iter().map(|v| v.var_type));
            }
        }
        for uid in self.regions.uids() {
            self.free_region(uid);
        }
        // What a test did outside a cycle (and the logouts above) leaves events on the
        // zones it touched: they go with the tracking, as at the end of a tick. A zone
        // with events that is not tracked is one no player may be sent.
        for coord in self.zones_tracking.drain() {
            if let Some(zone) = self.zones.zones.get_mut(&coord) {
                zone.reset();
            }
        }
        self.drop_region_zones();
        #[cfg(since_647)]
        {
            self.controllers = Controllers::default();
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

    /// Takes a region as `region_create` does.
    pub fn harness_region_create(
        &mut self,
        levels: i32,
        east: i32,
        north: i32,
    ) -> Option<RegionUid> {
        self.region_create(levels, east, north).ok()
    }

    /// Copies a zone onto region `uid` as `region_set` does.
    pub fn harness_region_set(
        &mut self,
        uid: RegionUid,
        level: i32,
        east: i32,
        north: i32,
        source: CoordGrid,
        rotation: i32,
    ) -> Result<(), ScriptError> {
        self.region_set(uid, level, east, north, source, rotation)
    }

    /// Sets region `uid`'s exit as `region_setexitcoord` does.
    pub fn harness_region_set_exit(
        &mut self,
        uid: RegionUid,
        exit: CoordGrid,
    ) -> Result<(), ScriptError> {
        self.region_set_exit(uid, exit)
    }

    /// The coord `x`, `z` tiles from region `uid`'s south-west corner on
    /// `level`.
    pub fn harness_region_coord(
        &self,
        uid: RegionUid,
        level: i32,
        x: i32,
        z: i32,
    ) -> Option<CoordGrid> {
        self.region_coord(uid, level, x, z).ok()
    }

    /// Frees region `uid` now, as the engine does once its last player
    /// leaves.
    pub fn harness_region_free(&mut self, uid: RegionUid) {
        self.free_region(uid);
    }

    /// Spawns controller `id` at `coord` as `controller_add` does.
    #[cfg(since_647)]
    pub fn harness_controller_add(&mut self, coord: CoordGrid, id: TypeId) -> bool {
        self.controller_add(coord, id, 0).is_ok()
    }

    /// What player `pid` would save now, read back from its `player_saves`
    /// row: the tile, the exit, and where a login puts that save.
    pub fn harness_saved_coord(
        &self,
        pid: u16,
    ) -> Option<(CoordGrid, Option<CoordGrid>, CoordGrid)> {
        let profile = profile_from_save(to_save(&self.profile_of(self.get_player(pid)?)));
        let saved = CoordGrid::new(profile.x, profile.y, profile.z);
        Some((saved, profile.exit, login_coord(saved, profile.exit)))
    }

    /// Player `pid`'s saved exit, read back from its `player_saves` row and
    /// from its `.sav` file.
    pub fn harness_saved_exits(&self, pid: u16) -> Option<(Option<CoordGrid>, Option<CoordGrid>)> {
        let profile = self.profile_of(self.get_player(pid)?);
        let row = profile_from_save(to_save(&profile)).exit;
        let file = load_binary(&save_binary(&profile, self.cache)).ok()?.exit;
        Some((row, file))
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
    let mut ops = test::build();
    ops.insert(SETUP_PLAYER, setup_player);
    ops.insert(TEST_OP, test_op);
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
    test::forget_calls();
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
            let id = TypeId::from_int(subject_id);
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
            (Some(target), category.map(|c| c.int()))
        }
        Subject::Npc => {
            let id = TypeId::from_int(subject_id);
            let npc = engine
                .npc_list
                .npcs
                .iter()
                .flatten()
                .find(|n| n.npc.uid.id() == id)
                .map(|n| n.npc.uid)
                .ok_or_else(|| ScriptError::Runtime(format!("test_op: no npc of type {id}")))?;
            let category = cache.npcs.get_by_id(id).and_then(|t| t.category);
            (Some(ScriptSubject::Npc(npc)), category.map(|c| c.int()))
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
