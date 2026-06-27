use crate::clients::client_game::ClientHandle;
use crate::engine::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_cam::CamKind;
use rs_entity::player::*;
use rs_entity::{EntityLifeTime, Player};
use rs_grid::CoordGrid;
use rs_info::FocusKind;
use rs_inv::Inventory;
use rs_io::{Packet, PacketFrame};
use rs_pack::cache::inv::InvScope;
use rs_pack::cache::{ScriptVarType, VarValue};
use rs_pack::types::{Font, PlayerStat};
use rs_protocol::network::game::client::ClientProtMessage;
use rs_protocol::network::game::client::anticheat_cyclelogic1::AnticheatCycleLogic1;
use rs_protocol::network::game::client::anticheat_cyclelogic2::AnticheatCycleLogic2;
use rs_protocol::network::game::client::anticheat_cyclelogic3::AnticheatCycleLogic3;
use rs_protocol::network::game::client::anticheat_cyclelogic4::AnticheatCycleLogic4;
use rs_protocol::network::game::client::anticheat_cyclelogic5::AnticheatCycleLogic5;
use rs_protocol::network::game::client::anticheat_cyclelogic6::AnticheatCycleLogic6;
use rs_protocol::network::game::client::anticheat_oplogic1::AnticheatOpLogic1;
use rs_protocol::network::game::client::anticheat_oplogic2::AnticheatOpLogic2;
use rs_protocol::network::game::client::anticheat_oplogic3::AnticheatOpLogic3;
use rs_protocol::network::game::client::anticheat_oplogic4::AnticheatOpLogic4;
use rs_protocol::network::game::client::anticheat_oplogic5::AnticheatOpLogic5;
use rs_protocol::network::game::client::anticheat_oplogic6::AnticheatOpLogic6;
use rs_protocol::network::game::client::anticheat_oplogic7::AnticheatOpLogic7;
use rs_protocol::network::game::client::anticheat_oplogic8::AnticheatOpLogic8;
use rs_protocol::network::game::client::anticheat_oplogic9::AnticheatOpLogic9;
use rs_protocol::network::game::client::chat_setmode::ChatSetMode;
use rs_protocol::network::game::client::client_cheat::ClientCheat;
use rs_protocol::network::game::client::close_modal::CloseModal;
#[cfg(rev = "225")]
use rs_protocol::network::game::client::event_camera_position::EventCameraPosition;
use rs_protocol::network::game::client::friendlist_add::FriendListAdd;
use rs_protocol::network::game::client::friendlist_del::FriendListDel;
use rs_protocol::network::game::client::idk_savedesign::IdkSaveDesign;
use rs_protocol::network::game::client::idle_timer::IdleTimer;
use rs_protocol::network::game::client::if_button::IfButton;
use rs_protocol::network::game::client::ignorelist_add::IgnoreListAdd;
use rs_protocol::network::game::client::ignorelist_del::IgnoreListDel;
use rs_protocol::network::game::client::inv_button1::InvButton1;
use rs_protocol::network::game::client::inv_button2::InvButton2;
use rs_protocol::network::game::client::inv_button3::InvButton3;
use rs_protocol::network::game::client::inv_button4::InvButton4;
use rs_protocol::network::game::client::inv_button5::InvButton5;
use rs_protocol::network::game::client::inv_buttond::InvButtonD;
use rs_protocol::network::game::client::message_private::MessagePrivate;
use rs_protocol::network::game::client::message_public::MessagePublic;
use rs_protocol::network::game::client::move_gameclick::MoveGameClick;
use rs_protocol::network::game::client::move_minimapclick::MoveMinimapClick;
use rs_protocol::network::game::client::move_opclick::MoveOpClick;
use rs_protocol::network::game::client::no_timeout::NoTimeout;
use rs_protocol::network::game::client::opheld1::OpHeld1;
use rs_protocol::network::game::client::opheld2::OpHeld2;
use rs_protocol::network::game::client::opheld3::OpHeld3;
use rs_protocol::network::game::client::opheld4::OpHeld4;
use rs_protocol::network::game::client::opheld5::OpHeld5;
use rs_protocol::network::game::client::opheldt::OpHeldT;
use rs_protocol::network::game::client::opheldu::OpHeldU;
use rs_protocol::network::game::client::oploc1::OpLoc1;
use rs_protocol::network::game::client::oploc2::OpLoc2;
use rs_protocol::network::game::client::oploc3::OpLoc3;
use rs_protocol::network::game::client::oploc4::OpLoc4;
use rs_protocol::network::game::client::oploc5::OpLoc5;
use rs_protocol::network::game::client::oploct::OpLocT;
use rs_protocol::network::game::client::oplocu::OpLocU;
use rs_protocol::network::game::client::opnpc1::OpNpc1;
use rs_protocol::network::game::client::opnpc2::OpNpc2;
use rs_protocol::network::game::client::opnpc3::OpNpc3;
use rs_protocol::network::game::client::opnpc4::OpNpc4;
use rs_protocol::network::game::client::opnpc5::OpNpc5;
use rs_protocol::network::game::client::opnpct::OpNpcT;
use rs_protocol::network::game::client::opnpcu::OpNpcU;
use rs_protocol::network::game::client::opobj1::OpObj1;
use rs_protocol::network::game::client::opobj2::OpObj2;
use rs_protocol::network::game::client::opobj3::OpObj3;
use rs_protocol::network::game::client::opobj4::OpObj4;
use rs_protocol::network::game::client::opobj5::OpObj5;
use rs_protocol::network::game::client::opobjt::OpObjT;
use rs_protocol::network::game::client::opobju::OpObjU;
use rs_protocol::network::game::client::opplayer1::OpPlayer1;
use rs_protocol::network::game::client::opplayer2::OpPlayer2;
use rs_protocol::network::game::client::opplayer3::OpPlayer3;
use rs_protocol::network::game::client::opplayer4::OpPlayer4;
use rs_protocol::network::game::client::opplayert::OpPlayerT;
use rs_protocol::network::game::client::opplayeru::OpPlayerU;
#[cfg(rev = "225")]
use rs_protocol::network::game::client::rebuild_get_maps::RebuildGetMaps;
use rs_protocol::network::game::client::resume_p_countdialog::ResumePCountDialog;
use rs_protocol::network::game::client::resume_pause_button::ResumePauseButton;
use rs_protocol::network::game::client::send_snapshot::SendSnapshot;
use rs_protocol::network::game::client::tut_clickside::TutClickSide;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::client_prot_category::ClientProtCategory;
use rs_protocol::network::game::info_prot::PlayerInfoProt;
use rs_protocol::network::game::server::ServerProtMessage;
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
use rs_protocol::network::game::server::loc_del::LocDel;
use rs_protocol::network::game::server::obj_add::ObjAdd;
use rs_protocol::network::game::server::update_zone_full_follows::UpdateZoneFullFollows;
use rs_protocol::network::game::server::update_zone_partial_enclosed::UpdateZonePartialEnclosed;
use rs_protocol::network::game::server::update_zone_partial_follows::UpdateZonePartialFollows;
use rs_protocol::network::game::server_prot_priority::ServerProtPriority;
use rs_var::VarSet;
use rs_vm::ScriptError;
use rs_vm::engine::{ScriptEngine, ScriptPlayer, cache};
use rs_vm::state::ScriptState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_zone::ZoneMessage;
use rs_zone::zone_map::ZoneMap;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::net::IpAddr;
use tracing::{error, warn};

/// Maximum number of recycled outbound byte buffers kept per client in
/// [`ClientHandle::buffer_pool`]. Caps memory if a client's network task
/// returns buffers faster than they're reused; extras are dropped (freed).
const OUTPUT_POOL_CAP: usize = 8;

type PartialInvUpdate = Vec<(u16, Option<(u16, i32)>)>;
type FullInvUpdate = Vec<Option<(u16, i32)>>;

/// Wraps a [`Player`] entity together with its network [`ClientHandle`] and
/// per-tick buffered output packets.
///
/// `ActivePlayer` is the primary engine-side handle for a connected player,
/// providing methods for packet I/O, server protocol messages, modal UI
/// management, inventory synchronization, appearance generation, and
/// script execution.
///
/// Rate-limiting counters (`client_limit`, `user_limit`, `restricted_limit`)
/// track how many messages of each category have been processed this tick.
pub struct ActivePlayer {
    pub player: Player,
    pub handle: Box<ClientHandle>,
    pub buffered: Vec<Packet>,
    pub client_limit: u8,
    pub user_limit: u8,
    pub restricted_limit: u8,
    pub remote_ip: Option<IpAddr>,
}

impl ActivePlayer {
    /// Creates a new `ActivePlayer` with default spawn coordinates,
    /// initialized varp set, and the given client handle.
    ///
    /// # Arguments
    /// * `handle` - The network client handle for this player's connection.
    /// * `pid` - The player's unique slot index in the world player array.
    /// * `username` - The player's display name.
    /// * `low_memory` - Whether the client is running in low-memory mode.
    /// * `bot` - Whether this player is a bot (non-human) client.
    ///
    /// # Returns
    /// A fully initialized `ActivePlayer` at the default spawn location.
    pub fn new(
        handle: ClientHandle,
        pid: u16,
        username: Box<str>,
        low_memory: bool,
        bot: bool,
    ) -> Self {
        let uid = PlayerUid::new(username, pid);

        let coord = CoordGrid::new(3094, 0, 3106); // Tutorial island

        let c = cache();
        let vars = VarSet::new((0..c.varps.count()).map(|id| {
            c.varps
                .get_by_id(id as u16)
                .map(|v| v.var_type)
                .unwrap_or(ScriptVarType::Int)
        }));

        let mut player = Player::new(uid, coord, vars, bot);
        player.low_memory = low_memory;

        ActivePlayer {
            player,
            handle: Box::new(handle),
            buffered: Vec::new(),
            client_limit: 0,
            user_limit: 0,
            restricted_limit: 0,
            remote_ip: None,
        }
    }

    /// Sends a server protocol message to the client, routing it through
    /// the appropriate priority channel.
    ///
    /// Buffered-priority messages are queued and flushed at the end of the
    /// tick via [`encode`](Self::encode). Immediate-priority messages are
    /// sent to the network layer right away.
    ///
    /// # Arguments
    /// * `message` - The server protocol message to send.
    ///
    /// # Call Stack
    /// **Calls:** [`queue_buffered`](Self::queue_buffered) or
    /// [`write_immediate`](Self::write_immediate) depending on priority.
    pub fn write<M>(&mut self, message: M)
    where
        M: ServerProtMessage,
    {
        match M::PRIORITY {
            ServerProtPriority::Buffered => self.queue_buffered(message),
            ServerProtPriority::Immediate => self.write_immediate(message),
        }
    }

    /// Encodes a server protocol message into a packet and queues it for
    /// sending at end-of-tick.
    ///
    /// The opcode byte, frame header, and payload are written into a new
    /// [`Packet`]. Messages larger than 5000 bytes are silently dropped.
    ///
    /// # Arguments
    /// * `message` - The server protocol message to queue.
    ///
    /// # Side Effects
    /// * Appends a packet to `self.buffered`.
    ///
    /// # Call Stack
    /// **Called by:** [`write`](Self::write)
    fn queue_buffered<M>(&mut self, message: M)
    where
        M: ServerProtMessage,
    {
        let frame = M::FRAME as usize;
        let len = 1 + frame + message.sizeof();
        if len > 5000 {
            return;
        }
        let mut buf = Packet::new(len);
        buf.p1(M::PROT as u8);
        buf.pos += frame;
        let start = buf.pos;
        message.encode(&mut buf);
        match M::FRAME {
            PacketFrame::Fixed => {}
            PacketFrame::VarByte => buf.psize1((buf.pos - start) as u8),
            PacketFrame::VarShort => buf.psize2((buf.pos - start) as u16),
        }
        self.buffered.push(buf);
    }

    /// Flushes all queued buffered packets to the network outbox.
    ///
    /// Each packet's opcode byte is ISAAC-encrypted before sending.
    ///
    /// # Side Effects
    /// * Drains `self.buffered` and sends each packet via `handle.outbox`.
    ///
    /// # Call Stack
    /// **Called by:** [`encode`](Self::encode)
    fn write_buffered(&mut self) {
        let handle = &mut self.handle;
        for mut buf in self.buffered.drain(..) {
            buf.data[0] = (buf.data[0] as u32 + handle.isaac_encode.next_int()) as u8;
            let _ = handle.outbox.send(buf.data);
        }
    }

    /// Encodes and immediately sends a server protocol message to the client,
    /// bypassing the buffered queue.
    ///
    /// Uses the shared `handle.write_queue` buffer. The opcode byte is
    /// ISAAC-encrypted before sending. Messages larger than 5000 bytes are
    /// silently dropped.
    ///
    /// # Arguments
    /// * `message` - The server protocol message to send immediately.
    ///
    /// # Call Stack
    /// **Called by:** [`write`](Self::write)
    fn write_immediate<M>(&mut self, message: M)
    where
        M: ServerProtMessage,
    {
        let handle = &mut self.handle;
        let frame = M::FRAME as usize;
        let len = 1 + frame + message.sizeof();
        if len > 5000 {
            return;
        }
        let buf = &mut handle.write_queue;
        buf.pos = 0;
        buf.p1((M::PROT as u32 + handle.isaac_encode.next_int()) as u8);
        buf.pos += frame;
        let start = buf.pos;
        message.encode(buf);
        match M::FRAME {
            PacketFrame::Fixed => {}
            PacketFrame::VarByte => buf.psize1((buf.pos - start) as u8),
            PacketFrame::VarShort => buf.psize2((buf.pos - start) as u16),
        }
        // Reuse a recycled outbound buffer instead of allocating a fresh
        // Vec per immediate message. The TCP net task returns drained buffers
        // via recycle_rx; copy the encoded bytes into one and send it. (The
        // copy from write_queue is unchanged from the old to_vec(); only the
        // per-message allocation is eliminated.)
        let pos = buf.pos;
        while let Ok(returned) = handle.recycle_rx.try_recv() {
            if handle.buffer_pool.len() < OUTPUT_POOL_CAP {
                handle.buffer_pool.push(returned);
            }
        }
        let mut out = handle.buffer_pool.pop().unwrap_or_default();
        out.clear();
        out.extend_from_slice(&handle.write_queue.data[0..pos]);
        let _ = handle.outbox.send(out);
    }

    /// Flushes pending modal UI state changes and all buffered packets for
    /// this tick.
    ///
    /// Detects changes to `modal_main`, `modal_chat`, and `modal_side` since
    /// the last tick and sends the appropriate open/close interface packets.
    /// Then flushes all buffered packets via [`write_buffered`](Self::write_buffered).
    ///
    /// # Side Effects
    /// * Sends `if_close` and/or `if_open_*` packets as needed.
    /// * Flushes all buffered packets.
    ///
    /// # Call Stack
    /// **Calls:** [`if_close`](Self::if_close),
    /// [`if_open_main_side`](Self::if_open_main_side),
    /// [`if_open_main`](Self::if_open_main),
    /// [`if_open_chat`](Self::if_open_chat),
    /// [`if_open_side`](Self::if_open_side),
    /// [`write_buffered`](Self::write_buffered)
    pub fn encode(&mut self) {
        if self.player.modal_main != self.player.last_modal_main
            || self.player.modal_chat != self.player.last_modal_chat
            || self.player.modal_side != self.player.last_modal_side
            || self.player.refresh_modal_close
        {
            if self.player.refresh_modal_close {
                self.if_close();
            }
            self.player.refresh_modal_close = false;
            self.player.last_modal_main = self.player.modal_main;
            self.player.last_modal_chat = self.player.modal_chat;
            self.player.last_modal_side = self.player.modal_side;
        }

        if self.player.refresh_modal {
            if self.player.modal_state & MODAL_MAIN != MODAL_NONE
                && self.player.modal_state & MODAL_SIDE != MODAL_NONE
            {
                if let (Some(main), Some(side)) = (self.player.modal_main, self.player.modal_side) {
                    self.if_open_main_side(main, side);
                }
            } else if self.player.modal_state & MODAL_MAIN != MODAL_NONE {
                if let Some(main) = self.player.modal_main {
                    self.if_open_main(main);
                }
            } else if self.player.modal_state & MODAL_CHAT != MODAL_NONE {
                if let Some(chat) = self.player.modal_chat {
                    self.if_open_chat(chat);
                }
            } else if self.player.modal_state & MODAL_SIDE != MODAL_NONE {
                if let Some(side) = self.player.modal_side {
                    self.if_open_side(side);
                }
            }
            self.player.refresh_modal = false;
        }

        self.write_buffered();
    }

    pub fn cam_moveto(&mut self, x: u8, z: u8, height: u16, rate: u8, rate2: u8) {
        self.write(rs_protocol::network::game::server::cam_move_to::CamMoveTo {
            x,
            z,
            height,
            rate,
            rate2,
        })
    }

    pub fn cam_lookat(&mut self, x: u8, z: u8, height: u16, rate: u8, rate2: u8) {
        self.write(rs_protocol::network::game::server::cam_look_at::CamLookAt {
            x,
            z,
            height,
            rate,
            rate2,
        })
    }

    pub fn cam_shake(&mut self, direction: u8, jitter: u8, amplitude: u8, frequency: u8) {
        self.write(rs_protocol::network::game::server::cam_shake::CamShake {
            direction,
            jitter,
            amplitude,
            frequency,
        })
    }

    /// Flushes queued camera updates and refreshes the player's map view. Runs once per
    /// tick during the output phase.
    ///
    /// 1. Drains the camera queue, converting each update's absolute coordinate to a
    ///    build-area-local coordinate and writing the matching `CamMoveTo` / `CamLookAt`.
    /// 2. On a map-zone (mapsquare) crossing, fires the `[mapzone]` / `[mapzoneexit]`
    ///    triggers.
    /// 3. On a zone crossing, rebuilds the active zone list (delegated to
    ///    [`Player::update_map`]), toggles the multiway indicator, and fires the
    ///    `[zone]` / `[zoneexit]` triggers.
    ///
    /// # Side Effects
    /// * Drains `self.player.cam_queue` and writes camera / `SetMultiway` packets.
    /// * May rebuild the build area's active zone list and enqueue engine-queue scripts.
    pub fn update_map(&mut self) {
        // 1. Flush queued camera updates as build-area-local coordinates.
        let origin_x = CoordGrid::zone_origin(self.player.build_area.origin.x());
        let origin_z = CoordGrid::zone_origin(self.player.build_area.origin.z());

        let mut h = self.player.cam_queue.queue.head();
        while let Some(idx) = h {
            let info = self.player.cam_queue.queue.unlink(idx);
            let local_x = info.x.wrapping_sub(origin_x) as u8;
            let local_z = info.z.wrapping_sub(origin_z) as u8;
            match info.kind {
                CamKind::MoveTo => {
                    self.cam_moveto(local_x, local_z, info.height, info.rate, info.rate2)
                }
                CamKind::LookAt => {
                    self.cam_lookat(local_x, local_z, info.height, info.rate, info.rate2)
                }
            }
            h = self.player.cam_queue.queue.next();
        }

        let coord = self.player.pathing.coord;

        // 2. Map-zone (mapsquare) crossing -> mapzone enter/exit triggers. Map zones span
        //    every level, so the tracked coord is pinned to level 0.
        let map_zone = CoordGrid::new((coord.x() >> 6) << 6, 0, (coord.z() >> 6) << 6);
        if self.player.last_map_zone != map_zone {
            let old = self.player.last_map_zone;
            self.trigger_mapzone_exit(old.x(), old.z());
            self.trigger_mapzone(coord.x(), coord.z());
            self.player.last_map_zone = map_zone;
        }

        // 3. Zone crossing -> rebuild active zones (delegated, also advances `last_zone`),
        //    update multiway, fire zone enter/exit triggers. Capture the old zone first.
        let zone = CoordGrid::new(coord.zone_x() << 3, coord.y(), coord.zone_z() << 3);
        let zone_changed = self.player.last_zone != zone;
        let old_zone = self.player.last_zone;

        self.player.update_map();

        if zone_changed {
            let c = cache();
            let was_multi = c.is_multi(old_zone.x(), old_zone.z(), old_zone.y());
            let now_multi = c.is_multi(zone.x(), zone.z(), zone.y());
            if was_multi != now_multi {
                self.write(
                    rs_protocol::network::game::server::set_multiway::SetMultiway {
                        hide: now_multi,
                    },
                );
            }
            self.trigger_zone_exit(old_zone.y(), old_zone.x(), old_zone.z());
            self.trigger_zone(coord.y(), zone.x(), zone.z());
        }
    }

    /// Looks up a coordinate-keyed trigger script by name and, if one is registered,
    /// enqueues it on the engine queue. A no-op when no script matches (e.g. the sentinel
    /// exit triggers fired for the initial `(0, 0, 0)` zone on spawn).
    fn enqueue_zone_trigger(&mut self, name: &str) {
        let script_id = engine().scripts.get_by_name(name).map(|s| s.id);
        if let Some(id) = script_id {
            let _ = self
                .player
                .state
                .queues
                .add(rs_vm::state::QueuePriority::Engine, id, 0, None);
        }
    }

    /// Fires the `[mapzone,...]` trigger for the mapsquare containing `(x, z)`.
    fn trigger_mapzone(&mut self, x: u16, z: u16) {
        let name = format!("[mapzone,0_{}_{}]", x >> 6, z >> 6);
        self.enqueue_zone_trigger(&name);
    }

    /// Fires the `[mapzoneexit,...]` trigger for the mapsquare containing `(x, z)`.
    fn trigger_mapzone_exit(&mut self, x: u16, z: u16) {
        let name = format!("[mapzoneexit,0_{}_{}]", x >> 6, z >> 6);
        self.enqueue_zone_trigger(&name);
    }

    /// Fires the `[zone,...]` trigger for the zone containing `(x, z)` on `level`.
    fn trigger_zone(&mut self, level: u8, x: u16, z: u16) {
        let mx = x >> 6;
        let mz = z >> 6;
        let lx = ((x & 0x3f) >> 3) << 3;
        let lz = ((z & 0x3f) >> 3) << 3;
        let name = format!("[zone,{}_{}_{}_{}_{}]", level, mx, mz, lx, lz);
        self.enqueue_zone_trigger(&name);
    }

    /// Fires the `[zoneexit,...]` trigger for the zone containing `(x, z)` on `level`.
    fn trigger_zone_exit(&mut self, level: u8, x: u16, z: u16) {
        let mx = x >> 6;
        let mz = z >> 6;
        let lx = ((x & 0x3f) >> 3) << 3;
        let lz = ((z & 0x3f) >> 3) << 3;
        let name = format!("[zoneexit,{}_{}_{}_{}_{}]", level, mx, mz, lx, lz);
        self.enqueue_zone_trigger(&name);
    }

    /// Sends a camera reset packet to the client, restoring the default camera.
    pub fn cam_reset(&mut self) {
        self.write(rs_protocol::network::game::server::cam_reset::CamReset);
    }

    /// Sends the chat filter settings (public, private, trade) to the client.
    pub fn chat_filter_settings(&mut self, public: u8, private: u8, trade: u8) {
        self.write(
            rs_protocol::network::game::server::chat_filter_settings::ChatFilterSettings {
                public,
                private,
                trade,
            },
        );
    }

    /// Sends a chunk of land (ground texture) map data to the client.
    #[cfg(rev = "225")]
    pub fn data_land(&mut self, x: u8, z: u8, off: u16, len: u16, data: &[u8]) {
        self.write(rs_protocol::network::game::server::data_land::DataLand {
            x,
            z,
            off,
            len,
            data,
        });
    }

    /// Signals to the client that all land data chunks for the given mapsquare
    /// have been sent.
    #[cfg(rev = "225")]
    pub fn data_land_done(&mut self, x: u8, z: u8) {
        self.write(rs_protocol::network::game::server::data_land_done::DataLandDone { x, z });
    }

    /// Sends a chunk of location map data to the client.
    #[cfg(rev = "225")]
    pub fn data_loc(&mut self, x: u8, z: u8, off: u16, len: u16, data: &[u8]) {
        self.write(rs_protocol::network::game::server::data_loc::DataLoc {
            x,
            z,
            off,
            len,
            data,
        });
    }

    /// Signals to the client that all location data chunks for the given
    /// mapsquare have been sent.
    #[cfg(rev = "225")]
    pub fn data_loc_done(&mut self, x: u8, z: u8) {
        self.write(rs_protocol::network::game::server::data_loc_done::DataLocDone { x, z });
    }

    /// Sends a hint arrow pointing at an NPC.
    ///
    /// # Arguments
    /// * `nid` - The world index of the NPC to highlight.
    pub fn hint_npc(&mut self, nid: u16) {
        self.write(rs_protocol::network::game::server::hint_arrow::HintArrow {
            hint: 1,
            arg1: nid,
            arg2: 0,
            arg3: 0,
        });
    }

    /// Sends a hint arrow hovering over a tile.
    ///
    /// # Arguments
    /// * `offset` - The arrow position type (`2`--`6`) selecting where over the tile the
    ///   arrow hovers.
    /// * `x` - The absolute tile X coordinate.
    /// * `z` - The absolute tile Z coordinate.
    /// * `height` - The vertical offset of the arrow above the tile.
    pub fn hint_tile(&mut self, offset: u8, x: u16, z: u16, height: u8) {
        self.write(rs_protocol::network::game::server::hint_arrow::HintArrow {
            hint: offset,
            arg1: x,
            arg2: z,
            arg3: height,
        });
    }

    /// Sends a hint arrow pointing at another player.
    ///
    /// # Arguments
    /// * `slot` - The player index (pid) to highlight.
    pub fn hint_player(&mut self, slot: u16) {
        self.write(rs_protocol::network::game::server::hint_arrow::HintArrow {
            hint: 10,
            arg1: slot,
            arg2: 0,
            arg3: 0,
        });
    }

    /// Clears any active hint arrow on the client. The wire type `-1` (`0xFF`) tells the
    /// client to remove the arrow.
    pub fn stop_hint(&mut self) {
        self.write(rs_protocol::network::game::server::hint_arrow::HintArrow {
            hint: 0xFF,
            arg1: 0,
            arg2: 0,
            arg3: 0,
        });
    }

    /// Sends an interface close packet to the client.
    pub fn if_close(&mut self) {
        self.write(rs_protocol::network::game::server::if_close::IfClose {});
    }

    /// Sends a packet to open both a main interface and a side interface.
    pub fn if_open_main_side(&mut self, com: u16, side: u16) {
        self.write(
            rs_protocol::network::game::server::if_openmain_side::IfOpenMainSide { com, side },
        );
    }

    /// Sends a packet to open a main (fullscreen) interface.
    pub fn if_open_main(&mut self, com: u16) {
        self.write(rs_protocol::network::game::server::if_openmain::IfOpenMain { com });
    }

    /// Sends a packet to open a chat-area interface.
    pub fn if_open_chat(&mut self, com: u16) {
        self.write(rs_protocol::network::game::server::if_openchat::IfOpenChat { com });
    }

    /// Sends a packet to open a side-panel interface.
    pub fn if_open_side(&mut self, com: u16) {
        self.write(rs_protocol::network::game::server::if_openside::IfOpenSide { com });
    }

    /// Sends the last-login info to the client (welcome screen).
    ///
    /// Reports the number of whole days between the player's previous login
    /// (`last_login_date`, from the saved profile) and this login (`last_date`,
    /// set during the join phase). A `last_login_date` of `0` (first login / no
    /// saved date) reports 0 days. This call does not mutate either timestamp.
    ///
    /// The source IP is hardcoded to loopback (`127.0.0.1` = `2130706433`) and
    /// the recovery code to `201`, which shows the standard welcome screen.
    pub fn last_login_info(&mut self) {
        let previous = if self.player.last_login_date == 0 {
            self.player.last_date
        } else {
            self.player.last_login_date
        };
        let days_since_login = ((self.player.last_date - previous) / (60 * 60 * 24)) as u16;
        self.write(
            rs_protocol::network::game::server::last_login_info::LastLoginInfo {
                ip: 2130706433,
                login: days_since_login,
                recovery: 201,
                messages: 0,
                #[cfg(since_244)]
                warn_members_in_non_members: !engine().members && self.member(),
            },
        );
    }

    /// Sets the animation playing on an interface component.
    pub fn if_setanim(&mut self, com: u16, seq: u16) {
        self.write(rs_protocol::network::game::server::if_setanim::IfSetAnim { com, seq });
    }

    /// Sets the color of an interface component.
    pub fn if_setcolour(&mut self, com: u16, colour: u16) {
        self.write(rs_protocol::network::game::server::if_setcolour::IfSetColour { com, colour });
    }

    /// Sets the visibility of an interface component.
    pub fn if_sethide(&mut self, com: u16, hide: bool) {
        self.write(rs_protocol::network::game::server::if_sethide::IfSetHide { com, hide });
    }

    /// Sets the model displayed on an interface component.
    pub fn if_setmodel(&mut self, com: u16, model: u16) {
        self.write(rs_protocol::network::game::server::if_setmodel::IfSetModel { com, model });
    }

    /// Sets an NPC head model on an interface component.
    pub fn if_setnpchead(&mut self, com: u16, npc: u16) {
        self.write(rs_protocol::network::game::server::if_setnpchead::IfSetNpcHead { com, npc });
    }

    /// Sets an item model on an interface component with a given zoom scale.
    pub fn if_setobject(&mut self, com: u16, obj: u16, scale: u16) {
        self.write(
            rs_protocol::network::game::server::if_setobject::IfSetObject { com, obj, scale },
        );
    }

    /// Sets the local player's head model on an interface component.
    pub fn if_setplayerhead(&mut self, com: u16) {
        self.write(rs_protocol::network::game::server::if_setplayerhead::IfSetPlayerHead { com });
    }

    /// Sets the position of an interface component.
    pub fn if_setposition(&mut self, com: u16, x: u16, y: u16) {
        self.write(rs_protocol::network::game::server::if_setposition::IfSetPosition { com, x, y });
    }

    /// Sets the scrollbar position on an interface component.
    #[cfg(since_245_2)]
    pub fn if_setscrollpos(&mut self, com: u16, y: u16) {
        self.write(rs_protocol::network::game::server::if_setscrollpos::IfSetScrollPos { com, y });
    }

    /// Opens a modal overlay interface component.
    #[cfg(since_244)]
    pub fn if_openoverlay(&mut self, com: u16) {
        self.write(rs_protocol::network::game::server::if_openoverlay::IfOpenOverlay { com });
    }

    /// Recolors an interface component model, remapping `src` color to `dst`.
    #[cfg(before_245_2)]
    pub fn if_setrecol(&mut self, com: u16, src: u16, dst: u16) {
        self.write(rs_protocol::network::game::server::if_setrecol::IfSetRecol { com, src, dst });
    }

    /// Assigns an interface component to a tab slot on the client.
    pub fn if_settab(&mut self, com: u16, tab: u8) {
        self.write(rs_protocol::network::game::server::if_settab::IfSetTab { com, tab });
    }

    /// Switches the client's currently selected (active) tab.
    pub fn if_settabactive(&mut self, tab: u8) {
        self.write(rs_protocol::network::game::server::if_settab_active::IfSetTabActive { tab });
    }

    /// Sets the text content of an interface component.
    pub fn if_settext(&mut self, com: u16, text: &str) {
        self.write(rs_protocol::network::game::server::if_settext::IfSetText { com, text });
    }

    /// Sends the logout packet and marks the player as having received it.
    ///
    /// # Side Effects
    /// * Sets `player.logout_sent = true`.
    pub fn logout(&mut self) {
        self.player.logout_sent = true;
        self.write(rs_protocol::network::game::server::logout::Logout);
    }

    /// Sends a game message to the player's chatbox.
    pub fn message_game(&mut self, text: &str) {
        self.write(rs_protocol::network::game::server::message_game::MessageGame { text });
    }

    /// Plays a MIDI jingle sound effect on the client.
    #[cfg(rev = "225")]
    pub fn midi_jingle(&mut self, delay: u16, bytes: &[u8]) {
        self.write(rs_protocol::network::game::server::midi_jingle::MidiJingle { delay, bytes });
    }

    /// Plays a MIDI jingle sound effect on the client.
    #[cfg(since_244)]
    pub fn midi_jingle(&mut self, id: u16, delay: u16) {
        self.write(rs_protocol::network::game::server::midi_jingle::MidiJingle { id, delay });
    }

    /// Starts playing a MIDI song on the client
    #[cfg(rev = "225")]
    pub fn midi_song(&mut self, name: &str, crc: i32, len: i32) {
        self.write(rs_protocol::network::game::server::midi_song::MidiSong { name, crc, len });
    }

    /// Starts playing a MIDI song on the client
    #[cfg(since_244)]
    pub fn midi_song(&mut self, id: u16) {
        self.write(rs_protocol::network::game::server::midi_song::MidiSong { id });
    }

    /// Sends the pre-encoded NPC info update payload to the client.
    pub fn npc_info(&mut self, bytes: &[u8]) {
        self.write(rs_protocol::network::game::server::npc_info::NpcInfo { bytes });
    }

    /// Sends the pre-encoded player info update payload to the client.
    pub fn player_info(&mut self, bytes: &[u8]) {
        self.write(rs_protocol::network::game::server::player_info::PlayerInfo { bytes });
    }

    /// Opens the count dialog (number input) on the client.
    pub fn p_countdialog(&mut self) {
        self.write(rs_protocol::network::game::server::p_countdialog::PCountDialog);
    }

    /// Sends the rebuild (map region) packet to the client if the player has
    /// moved far enough to require a new build area.
    ///
    /// Collects mapsquare CRCs for the new region and includes them in the
    /// packet so the client can request missing map data.
    ///
    /// # Arguments
    /// * `reconnecting` - If `true`, forces a rebuild even when the player
    ///   has not moved.
    ///
    /// # Side Effects
    /// * Updates the player's build area origin and mapsquare list.
    pub fn rebuild_normal(&mut self, reconnecting: bool) {
        if !self
            .player
            .build_area
            .needs_rebuild(&self.player.pathing.coord)
            && !reconnecting
        {
            return;
        }
        let coord = self.player.pathing.coord;
        self.player.build_area.rebuild_normal(&coord);
        let zone_x = self.player.pathing.coord.zone_x();
        let zone_z = self.player.pathing.coord.zone_z();
        #[cfg(rev = "225")]
        {
            let cache = cache();
            let mut crcs = FxHashMap::default();
            let mapsquares = self.player.build_area.mapsquares.clone();
            for mapsquare in &mapsquares {
                let x = (mapsquare >> 8) as u8;
                let z = (mapsquare & 0xFF) as u8;
                if let Some(crc) = cache.mapcrcs.get(&('m', x, z)).copied() {
                    crcs.insert(('m', x, z), crc);
                }
                if let Some(crc) = cache.mapcrcs.get(&('l', x, z)).copied() {
                    crcs.insert(('l', x, z), crc);
                }
            }
            self.write(
                rs_protocol::network::game::server::rebuild_normal::RebuildNormal {
                    zone_x,
                    zone_z,
                    mapsquares: mapsquares.into_iter().collect(),
                    crcs: crcs.into_iter().collect(),
                },
            );
        }
        #[cfg(since_244)]
        self.write(
            rs_protocol::network::game::server::rebuild_normal::RebuildNormal { zone_x, zone_z },
        );
    }

    /// Sends an animation reset packet to the client, clearing all playing
    /// entity animations.
    pub fn reset_anims(&mut self) {
        self.write(rs_protocol::network::game::server::reset_anims::ResetAnims);
    }

    /// Tells the client to clear its cached varp values, forcing a full
    /// resync on the next varp transmit.
    pub fn reset_client_varcache(&mut self) {
        self.write(rs_protocol::network::game::server::reset_client_varcache::ResetClientVarCache);
    }

    /// Plays a synthesised sound effect on the client.
    pub fn synth_sound(&mut self, synth: u16, loops: u8, delay: u16) {
        self.write(
            rs_protocol::network::game::server::synth_sound::SynthSound {
                synth,
                loops,
                delay,
            },
        );
    }

    /// Clears the destination map flag (red minimap marker) on the client.
    pub fn unset_map_flag(&mut self) {
        self.write(rs_protocol::network::game::server::unset_map_flag::UnsetMapFlag);
    }

    /// Sends a full inventory update to the client for the given component.
    pub fn update_inv_full(&mut self, com: u16, objs: &[Option<(u16, i32)>]) {
        // prevent a client crash by capping to inv size || interface size
        let size = cache()
            .interfaces
            .get_by_id(com)
            .map(|c| c.width as usize * c.height as usize)
            .unwrap_or(objs.len())
            .min(objs.len());
        self.write(
            rs_protocol::network::game::server::update_inv_full::UpdateInvFull {
                com,
                objs: &objs[..size],
            },
        );
    }

    /// Sends a partial inventory update to the client for the given component.
    ///
    /// Each entry is `(slot, item)` for one changed slot.
    pub fn update_inv_partial(&mut self, com: u16, objs: &[(u16, Option<(u16, i32)>)]) {
        self.write(
            rs_protocol::network::game::server::update_inv_partial::UpdateInvPartial { com, objs },
        );
    }

    /// Tells the client to stop expecting inventory updates for a component.
    pub fn update_inv_stop_transmit(&mut self, com: u16) {
        self.write(
            rs_protocol::network::game::server::update_inv_stop_transmit::UpdateInvStopTransmit {
                com,
            },
        );
    }

    /// Sends the player's server-side player ID to the client.
    pub fn update_pid(&mut self, pid: u16) {
        #[cfg(rev = "225")]
        self.write(rs_protocol::network::game::server::update_pid::UpdatePid { pid });
        #[cfg(since_244)]
        self.write(rs_protocol::network::game::server::update_pid::UpdatePid {
            pid,
            members: self.member(),
        });
    }

    /// Sends stat and run energy updates to the client for any values that
    /// have changed since the last tick.
    ///
    /// Compares current stats/levels against the cached `last_stats`/`last_levels`
    /// and only sends packets for changed entries. Run energy is compared at
    /// the percent granularity to avoid excessive updates.
    ///
    /// # Side Effects
    /// * Updates `player.stat_block.last_xp`, `player.stat_block.last_levels`, and
    ///   `player.last_runenergy` caches.
    pub fn update_stats(&mut self) {
        let dirty: Vec<usize> = self.player.stats.collect_dirty().collect();
        for stat in dirty {
            self.update_stat(stat);
        }

        let energy_percent = self.player.runenergy / 100;
        let last_percent = self.player.last_runenergy.map(|e| e / 100);
        if last_percent != Some(energy_percent) {
            self.update_runenergy(self.player.runenergy);
            self.player.last_runenergy = Some(self.player.runenergy);
        }
    }

    /// Sends a single stat update packet to the client.
    pub fn update_stat(&mut self, stat: usize) {
        self.write(
            rs_protocol::network::game::server::update_stat::UpdateStat {
                stat: stat as u8,
                exp: self.player.stats.xp[stat] / 10,
                lvl: self.player.stats.levels[stat],
            },
        );
    }

    /// Sends a run energy update packet to the client.
    ///
    /// The energy value is divided by 100 to convert from internal
    /// representation to the client's 0-100 percentage scale.
    pub fn update_runenergy(&mut self, energy: u16) {
        self.write(
            rs_protocol::network::game::server::update_runenergy::UpdateRunEnergy {
                energy: (energy / 100) as u8,
            },
        );
    }

    /// Transmits a varp (player variable) value to the client, automatically
    /// choosing the small (1-byte) or large (4-byte) encoding based on the
    /// value range.
    pub fn varp_transmit(&mut self, id: u16, val: i32) {
        if val <= u8::MAX as i32 {
            self.varp_small(id, val as u8);
        } else {
            self.varp_large(id, val);
        }
    }

    /// Sends a varp update using the 4-byte (large) encoding.
    pub fn varp_large(&mut self, id: u16, val: i32) {
        self.write(rs_protocol::network::game::server::varp_large::VarpLarge { id, val });
    }

    /// Sends a varp update using the 1-byte (small) encoding.
    pub fn varp_small(&mut self, id: u16, val: u8) {
        self.write(rs_protocol::network::game::server::varp_small::VarpSmall { id, val });
    }

    /// Synchronizes dirty inventories to the client.
    ///
    /// Iterates over all registered inventory transmits. For each, retrieves
    /// the inventory (from shared world inventories or the player's own),
    /// sends a full update to every bound interface component, and clears
    /// the dirty flag on private inventories.
    ///
    /// # Arguments
    /// * `shared_invs` - The world-level shared inventory map.
    ///
    /// # Side Effects
    /// * Sends `update_inv_full` packets for each bound component.
    /// * Clears the `dirty` flag on non-shared inventories.
    pub fn update_invs(&mut self, shared_invs: &mut FxHashMap<u16, Inventory>) {
        thread_local! {
            // Full payload (every slot) for first-seen components.
            static FULL: RefCell<FullInvUpdate> = RefCell::new(Vec::with_capacity(48));
            // Partial payload (changed slots only) for already-seen components.
            static PARTIAL: RefCell<PartialInvUpdate> = RefCell::new(Vec::with_capacity(48));
        }

        // Take the transmit map out of the player so we can iterate it while
        // calling `&mut self` methods (update_inv_full), then put it back. This
        // avoids cloning the whole map + every `coms` Vec each tick per player.
        // Nothing in the loop mutates `inv_transmits`, so taking it is safe.
        let transmits = std::mem::take(&mut self.player.inv_transmits);

        // A `runweight` inventory changed (recompute & maybe send weight).
        let mut runweight_changed = false;
        // A player inventory was first-seen this tick (force a weight send so the
        // client has the correct value, e.g. right after logging in).
        let mut first_seen_weight = false;

        for (inv_id, coms) in &transmits {
            let inv_type = cache().invs.get_by_id(*inv_id);
            let is_shared = inv_type.is_some_and(|t| t.scope == InvScope::Shared);
            let runweight_inv = inv_type.is_some_and(|t| t.runweight);

            let inv = if is_shared {
                shared_invs.get(inv_id)
            } else {
                self.player.invs.get(inv_id)
            };

            let Some(inv) = inv else { continue };
            let has_dirty = !inv.dirty_slots.is_empty();
            let any_first_seen = coms.iter().any(|c| !self.player.inv_first_seen.contains(c));

            // Build the payloads once per inventory into reusable scratch buffers.
            // The `inv` borrow ends after these builds, freeing `self` for the
            // `&mut self` send calls below.
            if any_first_seen {
                FULL.with_borrow_mut(|f| {
                    f.clear();
                    f.extend(
                        inv.slots
                            .iter()
                            .map(|i| i.map(|it| (it.obj, it.num as i32))),
                    );
                });
            }
            if has_dirty {
                PARTIAL.with_borrow_mut(|p| {
                    p.clear();
                    p.extend(inv.collect_dirty());
                });
            }

            let mut any_full = false;
            for &com in coms {
                if !self.player.inv_first_seen.contains(&com) {
                    FULL.with_borrow(|f| self.update_inv_full(com, f));
                    self.player.inv_first_seen.insert(com);
                    any_full = true;
                } else if has_dirty {
                    PARTIAL.with_borrow(|p| self.update_inv_partial(com, p));
                }
            }

            // Weight tracking applies only to player (non-shared) inventories.
            if !is_shared {
                if any_full {
                    first_seen_weight = true;
                }
                if runweight_inv && (has_dirty || any_full) {
                    runweight_changed = true;
                }
            }
        }

        self.player.inv_transmits = transmits;

        if runweight_changed {
            let current = self.player.runweight;
            self.calculate_runweight();
            runweight_changed = current != self.player.runweight;
        }
        if runweight_changed || first_seen_weight {
            self.write(
                rs_protocol::network::game::server::update_runweight::UpdateRunWeight {
                    kg: (self.player.runweight / 1000) as u16,
                },
            );
        }
    }

    /// Mirrors other players' inventories onto this player's interface components.
    ///
    /// Processes every `invother_transmit` listener: resolves the source player by
    /// script-uid, reads the requested inventory, and transmits it to the bound
    /// component. The first transmit for a component is a full update; subsequent
    /// ticks send only the source inventory's changed slots (its dirty set survives
    /// until the cleanup phase, so every viewer this tick sees the same changes).
    /// Listeners whose source player has logged out -- or whose uid no longer matches
    /// because the player slot was reused -- are skipped.
    ///
    /// # Arguments
    /// * `players` - The engine's player table, used to resolve source players by
    ///   uid. The active player has been taken out of this table during output
    ///   processing, so a listener targeting oneself resolves to an empty slot.
    ///
    /// # Side Effects
    /// * Sends `update_inv_full` / `update_inv_partial` packets for each resolved listener.
    pub fn update_other_invs(&mut self, players: &[Option<ActivePlayer>]) {
        if self.player.inv_other_transmits.is_empty() {
            return;
        }

        thread_local! {
            static FULL: RefCell<FullInvUpdate> = RefCell::new(Vec::with_capacity(48));
            static PARTIAL: RefCell<PartialInvUpdate> = RefCell::new(Vec::with_capacity(48));
        }

        // Take the map out so `&mut self` methods (update_inv_full) can be called
        // while iterating, then restore it. Nothing in the loop mutates the map.
        let transmits = std::mem::take(&mut self.player.inv_other_transmits);

        for (&com, &(uid, inv_id)) in &transmits {
            let pid = (uid & 0x7FF) as usize;
            let Some(source) = players.get(pid).and_then(|p| p.as_ref()) else {
                continue;
            };
            // Guard against a reused player slot: the live uid must still match.
            if source.player.uid.packed() as i32 != uid {
                continue;
            }
            let Some(inv) = source.player.invs.get(&inv_id) else {
                continue;
            };

            let first_seen = !self.player.inv_first_seen.contains(&com);
            let has_dirty = !inv.dirty_slots.is_empty();
            if first_seen {
                FULL.with_borrow_mut(|f| {
                    f.clear();
                    f.extend(
                        inv.slots
                            .iter()
                            .map(|i| i.map(|it| (it.obj, it.num as i32))),
                    );
                });
            } else if has_dirty {
                PARTIAL.with_borrow_mut(|p| {
                    p.clear();
                    p.extend(inv.collect_dirty());
                });
            }
            // `source`'s borrow ends above, so borrowing `self` mutably is now safe.
            if first_seen {
                FULL.with_borrow(|f| self.update_inv_full(com, f));
                self.player.inv_first_seen.insert(com);
            } else if has_dirty {
                PARTIAL.with_borrow(|p| self.update_inv_partial(com, p));
            }
        }

        self.player.inv_other_transmits = transmits;
    }

    /// Recomputes the player's carried weight, in grams, from every
    /// `runweight`-flagged inventory the player owns.
    ///
    /// Sums each non-stackable item's per-unit weight times its stack count.
    /// Stackable objects (coins, runes, arrows, ...) are excluded, as is any
    /// item whose obj type is missing from the cache.
    ///
    /// # Side Effects
    /// * Overwrites `self.player.runweight`.
    fn calculate_runweight(&mut self) {
        let c = cache();
        let mut weight = 0;
        for (inv_id, inv) in &self.player.invs {
            let Some(inv_type) = c.invs.get_by_id(*inv_id) else {
                continue;
            };
            if !inv_type.runweight {
                continue;
            }
            for item in inv.slots.iter().flatten() {
                let Some(obj_type) = c.objs.get_by_id(item.obj) else {
                    continue;
                };
                if obj_type.stackable {
                    continue;
                }
                weight += obj_type.weight as i32 * item.num as i32;
            }
        }
        self.player.runweight = weight;
    }

    /// Sends a game message to the player's chatbox, word-wrapping long
    /// messages to fit the chatbox width (456 pixels using the P12 font).
    ///
    /// Falls back to sending the text as a single message if the P12 font
    /// is not available in the cache.
    ///
    /// # Arguments
    /// * `text` - The message text (may be longer than one chatbox line).
    pub fn message_game_wrapped(&mut self, text: &str) {
        if let Some(font) = cache().fonts.get(Font::P12) {
            let lines = font.split(text, 456);
            for line in lines {
                self.message_game(&line);
            }
        } else {
            self.message_game(text);
        }
    }

    /// Transmits all transmittable varps to the client.
    ///
    /// Iterates over every varp type in the cache. For each that has
    /// `transmit = true`, sends the current value to the client.
    ///
    /// # Side Effects
    /// * Sends varp_small or varp_large packets for every transmittable varp.
    pub fn sync_varps(&mut self) {
        let varp_types = &cache().varps;
        for id in 0..self.player.vars.len() {
            let Some(varp) = varp_types.get_by_id(id as u16) else {
                continue;
            };
            if !varp.transmit {
                continue;
            }
            let val = self.player.vars.get(id as u16).as_int();
            self.varp_transmit(id as u16, val);
        }
    }

    /// Assigns an interface component to a tab slot and sends the update to
    /// the client.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `tab` - The tab index (0-based). Out-of-range values are ignored.
    ///
    /// # Side Effects
    /// * Updates `player.tabs[tab]` and sends an `if_settab` packet.
    pub fn set_tab(&mut self, com: u16, tab: u8) {
        if (tab as usize) >= self.player.tabs.len() {
            return;
        }
        self.player.tabs[tab as usize] = Some(com);
        self.if_settab(com, tab);
    }

    /// Re-sends all tab assignments to the client.
    ///
    /// Used during reconnection to restore tab state.
    pub fn sync_tabs(&mut self) {
        for index in 0..self.player.tabs.len() {
            if let Some(tab) = &self.player.tabs[index] {
                self.if_settab(*tab, index as u8);
            }
        }
    }

    /// Opens a chat-area modal interface, closing any existing modal first
    /// if needed.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open as a chat modal.
    ///
    /// # Side Effects
    /// * May send an `if_close` packet if a previous modal was replaced.
    pub fn open_chat_modal(&mut self, com: u16) {
        if self.player.open_chat_modal(com) {
            self.if_close();
        }
    }

    /// Opens a main + side modal interface pair, closing any existing modal
    /// first if needed.
    ///
    /// # Arguments
    /// * `com` - The main interface component ID.
    /// * `side` - The side interface component ID.
    pub fn open_main_side_modal(&mut self, com: u16, side: u16) {
        if self.player.open_main_side_modal(com, side) {
            self.if_close();
        }
    }

    /// Opens a main (fullscreen) modal interface, closing any existing modal
    /// first if needed.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open.
    pub fn open_main_modal(&mut self, com: u16) {
        if self.player.open_main_modal(com) {
            self.if_close();
        }
    }

    /// Opens a side-panel modal interface.
    ///
    /// Unlike the main and chat modals, opening a side modal does not displace
    /// any existing modal, so no `if_close` is needed.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open.
    pub fn open_side_modal(&mut self, com: u16) {
        self.player.open_side_modal(com);
    }

    /// Opens a tutorial interface: sends the `TutOpen` packet and records the
    /// tutorial as the active tutorial modal.
    ///
    /// # Arguments
    /// * `com` - The interface component ID to open.
    pub fn open_tutorial(&mut self, com: u16) {
        self.write(rs_protocol::network::game::server::tut_open::TutOpen { com });
        self.player.open_tutorial(com);
    }

    /// Flashes a tutorial tab on the client to draw the player's attention.
    ///
    /// # Arguments
    /// * `tab` - The tab index to flash.
    pub fn tut_flash(&mut self, tab: u8) {
        self.write(rs_protocol::network::game::server::tut_flash::TutFlash { tab });
    }

    /// Closes the active tutorial interface, if one is open.
    ///
    /// Fires the tutorial interface's `IfClose` trigger (a missing trigger is
    /// ignored), clears the tracked tutorial, and tells the client to close it
    /// by sending `TutOpen(-1)`.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`] if the close trigger fails.
    pub fn close_tutorial(&mut self) -> Result<(), ScriptError> {
        if let Some(modal_tutorial) = self.player.modal_tutorial {
            if let Err(e) = engine_mut().run_script_by_trigger(
                (ServerTriggerType::IfClose, Some(modal_tutorial), None),
                Some(ScriptSubject::Player(self.player.uid)),
                None,
                Some(false),
                Some(false),
                None,
            ) && !matches!(e, ScriptError::TriggerNotFound(_))
            {
                return Err(e);
            }
            self.player.modal_tutorial = None;
            // -1 (u16::MAX on the wire) tells the client to close the tutorial.
            self.write(rs_protocol::network::game::server::tut_open::TutOpen { com: u16::MAX });
        }
        Ok(())
    }

    /// Marks the player's appearance as needing a rebuild, using the given
    /// inventory as the worn equipment source.
    ///
    /// # Arguments
    /// * `inv_id` - The inventory type ID that provides equipped items for
    ///   the appearance.
    ///
    /// # Side Effects
    /// * Sets `player.info.appearance` and enables the
    ///   `PlayerInfoProt::Appearance` mask.
    pub fn buildappearance(&mut self, inv_id: u16) {
        self.player.info.appearance = Some(inv_id);
        self.player.info.masks |= PlayerInfoProt::Appearance as u16;
    }

    /// Recalculates the player's combat level and, if it changed, rebuilds the
    /// appearance so the new combat level is reflected to other players.
    pub fn recalc_combat_and_appearance(&mut self) {
        let new_combat = self.player.get_combat_level();
        if new_combat != self.player.combat_level {
            self.player.combat_level = new_combat;
            if let Some(appearance) = self.player.info.appearance {
                self.buildappearance(appearance);
            }
        }
    }

    /// Applies damage to this player, clamping hitpoints to zero if the damage
    /// exceeds the current value.
    ///
    /// # Arguments
    /// * `amount` - The amount of damage to deal.
    /// * `damage_type` - The hitmark type identifier (e.g. block, damage, poison).
    ///
    /// # Side Effects
    /// * Reduces `player.stats.levels[Hitpoints]` by `amount` (saturating at 0).
    /// * Populates the damage info fields (`damage_taken`, `damage_type`,
    ///   `damage_current`, `damage_base`) and sets the `PlayerInfoProt::Damage`
    ///   mask for the next info update. On 244, a second hit within the same
    ///   tick fills the `damage2_*` fields and sets `PlayerInfoProt::Damage2`
    ///   instead, alternating via the per-tick `damage_slot` counter.
    pub fn damage(&mut self, amount: u8, damage_type: u8) {
        let current = self.player.stats.levels[PlayerStat::Hitpoints as usize];
        let taken = if current.saturating_sub(amount) == 0 {
            self.player.stats.levels[PlayerStat::Hitpoints as usize] = 0;
            current
        } else {
            self.player.stats.levels[PlayerStat::Hitpoints as usize] =
                current.saturating_sub(amount);
            amount
        };
        let remaining = self.player.stats.levels[PlayerStat::Hitpoints as usize];
        let base = self.player.stats.base_levels[PlayerStat::Hitpoints as usize];

        #[cfg(since_244)]
        if self.player.info.apply_damage2(
            taken,
            damage_type,
            remaining,
            base,
            PlayerInfoProt::Damage2 as u16,
        ) {
            return;
        }

        self.player.info.apply_damage(
            taken,
            damage_type,
            remaining,
            base,
            PlayerInfoProt::Damage as u16,
        );
    }

    /// Executes the player's pending walk trigger script, if any.
    ///
    /// Walk triggers fire when the player moves and allow scripts to override
    /// the destination. They are processed during client input and before
    /// each interaction to support `p_walk` calls that modify the movement
    /// queue during the trigger.
    ///
    /// Skipped if the player is in a protected or delayed script state.
    ///
    /// # Side Effects
    /// * Clears `player.walktrigger` and runs the associated script.
    /// * Logs an error if the script execution fails.
    pub fn process_walktrigger(&mut self) {
        let Some(trigger) = self.player.walktrigger else {
            return;
        };
        if self.player.state.protect || self.player.state.delayed {
            return;
        }
        self.player.walktrigger = None;
        if let Some(script) = engine().get_script(trigger).cloned() {
            let state = ScriptState::init(
                script,
                Some(ScriptSubject::Player(self.player.uid)),
                None,
                None,
            );
            if let Err(e) = engine_mut().run_script_by_state(
                state,
                Some(ScriptSubject::Player(self.player.uid)),
                Some(true),
                None,
            ) {
                error!(
                    "error running walktrigger script for player {}: {e}",
                    self.player.uid.pid()
                );
            }
        }
    }

    /// Generates the binary appearance block for this player.
    ///
    /// Encodes gender, head icons, equipped items (with slot-skip logic
    /// for items that hide other slots), body part model IDs, color,
    /// movement animation IDs, the base-37 username, and combat level into
    /// a byte buffer. Stores the result in `player.info.last_appearance_info`.
    ///
    /// # Arguments
    /// * `clock` - The current game tick, stored as the appearance timestamp.
    ///
    /// # Side Effects
    /// * Updates `player.info.last_appearance` and
    ///   `player.info.last_appearance_info`.
    ///
    /// # Panics
    /// Panics if any of the required animation fields (`readyanim`, `turnanim`,
    /// `walkanim`, `walkanim_b`, `walkanim_l`, `walkanim_r`, `runanim`)
    /// are `None`.
    pub fn generateappearance(&mut self, clock: u32) {
        use std::cell::RefCell;
        // Reused per-thread scratch buffer so each appearance rebuild doesn't
        // allocate (and zero) a fresh Packet. Only the final boxed slice -- the
        // value actually stored on the player -- is allocated. The engine is
        // single-threaded, so a thread-local is sound and contention-free.
        thread_local! {
            static SCRATCH: RefCell<Packet> = RefCell::new(Packet::new(256));
        }

        let appearance: Box<[u8]> = SCRATCH.with_borrow_mut(|buf| {
            buf.pos = 0;
            buf.p1(self.player.gender);
            buf.p1(self.player.headicons);

            let mut skipped_slots = [false; 12];
            if let Some(worn) = self
                .player
                .invs
                .get(&self.player.info.appearance.unwrap_or(0))
            {
                for item in worn.slots.iter().flatten() {
                    if let Some(obj) = cache().objs.get_by_id(item.obj) {
                        if let Some(wp2) = obj.wearpos2.filter(|&w| (w as usize) < 12) {
                            skipped_slots[wp2 as usize] = true;
                        }
                        if let Some(wp3) = obj.wearpos3.filter(|&w| (w as usize) < 12) {
                            skipped_slots[wp3 as usize] = true;
                        }
                    }
                }
            }

            let worn_inv_id = self.player.info.appearance.unwrap_or(0);
            for slot in 0..12u16 {
                if skipped_slots[slot as usize] {
                    buf.p1(0);
                    continue;
                }
                let equip = self
                    .player
                    .invs
                    .get(&worn_inv_id)
                    .and_then(|inv| inv.get(slot));
                if let Some(item) = equip {
                    buf.p2(0x200 + item.obj);
                } else {
                    let appearance_value = self.get_appearance_in_slot(slot as usize);
                    if appearance_value < 1 {
                        buf.p1(0);
                    } else {
                        buf.p2(appearance_value as u16);
                    }
                }
            }

            for &color in &self.player.colours {
                buf.p1(color);
            }

            buf.p2(self.player.info.readyanim.unwrap());
            buf.p2(self.player.info.turnanim.unwrap());
            buf.p2(self.player.info.walkanim.unwrap());
            buf.p2(self.player.info.walkanim_b.unwrap());
            buf.p2(self.player.info.walkanim_l.unwrap());
            buf.p2(self.player.info.walkanim_r.unwrap());
            buf.p2(self.player.info.runanim.unwrap());
            buf.p8(self.uid().username37() as i64);
            buf.p1(self.player.combat_level);

            Box::from(&buf.data[..buf.pos])
        });

        self.player.info.last_appearance = Some(clock);
        self.player.info.last_appearance_info = Some(appearance);
    }

    /// Returns the body model kit value for the given equipment slot when
    /// no item is worn there.
    ///
    /// Maps equipment slot indices to body part indices and returns the
    /// kit ID offset by 0x100. Returns 0 for slots that have no body
    /// part mapping or when the body part is hidden (value < 0).
    ///
    /// # Arguments
    /// * `slot` - The equipment slot index (0..12).
    ///
    /// # Returns
    /// The kit model value (0x100 + body_part), or 0 if hidden/unmapped.
    fn get_appearance_in_slot(&self, slot: usize) -> i32 {
        let part = match slot {
            8 => self.player.body[0],
            11 => self.player.body[1],
            4 => self.player.body[2],
            6 => self.player.body[3],
            9 => self.player.body[4],
            7 => self.player.body[5],
            10 => self.player.body[6],
            _ => return 0,
        };
        if part < 0 { 0 } else { 0x100 + part }
    }

    /// Plays an animation (sequence) on this player, respecting priority.
    ///
    /// If the player already has an animation playing, the new animation only
    /// replaces it when its sequence priority is equal to or higher.
    ///
    /// # Arguments
    /// * `id` - The sequence (anim) ID to play, or `None` to clear.
    /// * `delay` - The tick delay before the animation begins.
    ///
    /// # Side Effects
    /// * Updates `player.info` animation fields and sets the
    ///   `PlayerInfoProt::Anim` mask.
    pub fn anim(&mut self, id: Option<u16>, delay: u8) {
        let cur_pri = self
            .player
            .info
            .anim_id
            .and_then(|a| cache().seqs.get_by_id(a))
            .map(|s| s.priority as u16);
        let new_pri = id
            .and_then(|a| cache().seqs.get_by_id(a))
            .map(|s| s.priority as u16);
        self.player
            .info
            .set_anim(id, delay, PlayerInfoProt::Anim as u16, cur_pri, new_pri);
    }

    /// Clears the player's movement queue and removes the map destination flag
    /// on the client.
    ///
    /// # Side Effects
    /// * Clears `player.waypoints` and sends an `unset_map_flag` packet.
    pub fn clear_waypoints(&mut self) {
        self.player.clear_waypoints();
        self.unset_map_flag();
    }

    /// Sets a varp (player variable) to the given value and optionally
    /// transmits it to the client.
    ///
    /// # Arguments
    /// * `id` - The varp ID.
    /// * `value` - The new value.
    /// * `transmit` - Whether to send the update to the client.
    ///
    /// # Side Effects
    /// * Updates the player's varp set.
    /// * If `transmit` is true, sends a varp packet to the client.
    pub fn set_varp(&mut self, id: u16, value: VarValue, transmit: bool) {
        self.player.vars.set(id, value.clone());
        if transmit {
            self.varp_transmit(id, value.as_int());
        }
    }

    /// Sets a varp identified by its debug name, resolving the numeric id
    /// through the cache's name index.
    ///
    /// This mirrors the reference engine's `setVar(VarPlayerType.X, ...)`
    /// usage, where the engine pushes a well-known varp without hardcoding its
    /// id. The value is coerced to the varp's declared [`ScriptVarType`], and
    /// the varp's own `transmit` flag governs whether the change is sent to the
    /// client -- identical to how a scripted `setvar` write behaves.
    ///
    /// # Arguments
    /// * `name` - The varp debug name (e.g. `"run"`).
    /// * `value` - The integer value to store (booleans use `0` / `1`).
    ///
    /// # Returns
    /// `true` if a varp with that name exists and was set; `false` if no such
    /// varp is present in the cache.
    ///
    /// # Side Effects
    /// * Updates the player's varp set and, when the varp is transmittable,
    ///   sends a varp packet to the client.
    pub fn set_varp_by_name(&mut self, name: &str, value: i32) -> bool {
        let Some(varp) = cache().varps.get_by_debugname(name) else {
            return false;
        };
        let id = varp.id;
        let transmit = varp.transmit;
        let value = VarValue::from_int(varp.var_type, value);
        self.set_varp(id, value, transmit);
        true
    }

    /// Synchronizes the client's `run` varp with the player's current `run`
    /// flag.
    ///
    /// The engine stores run state as a plain `player.run` bool, but the client
    /// run orb is driven by a varp. Whenever the engine flips run on its own --
    /// the `P_RUN` script op, or run energy hitting zero in
    /// `Player::update_energy` -- this pushes the new value so the orb stays in
    /// sync. No-ops silently if the cache has no `run` varp.
    pub fn sync_run(&mut self) {
        self.set_varp_by_name("option_run", self.player.run as i32);
    }
}

/// Engine-level player operations that bridge client I/O, zone updates,
/// modal UI management, and teleportation.
///
/// Implemented on [`ActivePlayer`] to provide the core per-tick processing
/// methods used by the game engine loop.
pub trait EnginePlayer {
    /// Drains the client inbox and decodes inbound packets up to per-category
    /// rate limits. Returns `true` if any inbound data arrived this tick
    /// (used to track connection liveness for the no-response timeout).
    fn decode(&mut self) -> bool;

    /// Reads and dispatches a single client protocol message.
    fn read(&mut self) -> Option<()>;

    fn on_login(&mut self);

    /// Re-sends all state needed after a client reconnection.
    fn on_reconnect(&mut self) -> Result<(), ScriptError>;

    /// Writes a single zone event message to the client.
    fn write_zone_message(&mut self, message: &ZoneMessage);

    /// Sends full and incremental zone updates (objects, locs, events) to
    /// the client for all zones in the build area.
    fn update_zones(&mut self, zones: &ZoneMap, clock: u32);

    /// Closes any open modal interface, optionally clearing the weak script
    /// queue, and fires `IfClose` triggers.
    fn close_modal(&mut self, clear_weak_queue: bool) -> Result<(), ScriptError>;

    /// Clears the player's current interaction and closes any modal.
    fn clear_pending_action(&mut self) -> Result<(), ScriptError>;

    /// Stops all player actions: clears waypoints and pending actions.
    fn stop_action(&mut self) -> Result<(), ScriptError>;

    /// Teleports the player to the given coordinate with a jump animation.
    fn tele_jump(&mut self, coord: CoordGrid);

    /// Teleports the player to the given coordinate.
    fn tele(&mut self, coord: CoordGrid);
}

impl EnginePlayer for ActivePlayer {
    /// Drains the network inbox into a read queue, then processes packets
    /// in a loop until per-category rate limits are reached or the queue
    /// is empty.
    ///
    /// Messages that would exceed the 5000-byte read queue limit are held
    /// as pending for the next tick.
    ///
    /// # Returns
    /// `true` if at least one new message arrived from the network inbox
    /// this tick (a held-over `pending_msg` does not count -- it was
    /// received on an earlier tick).
    ///
    /// # Side Effects
    /// * Resets rate-limit counters to 0.
    /// * Processes client messages via [`read`](Self::read).
    fn decode(&mut self) -> bool {
        let handle = &mut self.handle;
        let mut received = false;
        let mut next = handle.pending_msg.take();
        loop {
            let msg = match next.take() {
                Some(m) => m,
                None => match handle.inbox.try_recv() {
                    Ok(m) => {
                        received = true;
                        m
                    }
                    Err(_) => break,
                },
            };
            if handle.read_queue.len() + msg.len() > 5000 {
                handle.pending_msg = Some(msg);
                break;
            }
            handle.read_queue.extend(msg);
        }

        self.client_limit = 0;
        self.user_limit = 0;
        self.restricted_limit = 0;

        while self.client_limit < ClientProtCategory::ClientEvent as u8
            && self.user_limit < ClientProtCategory::UserEvent as u8
            && self.restricted_limit < ClientProtCategory::RestrictedEvent as u8
            && !self.handle.read_queue.is_empty()
        {
            self.read();
        }

        received
    }

    /// Reads a single client protocol message from the read queue, decodes
    /// it, and dispatches it to the appropriate handler.
    ///
    /// Decrypts the opcode byte using the ISAAC cipher, determines the
    /// packet length from the frame type (fixed, var-byte, var-short),
    /// and routes to the matching `ClientProt` handler. On success,
    /// increments the appropriate rate-limit counter.
    ///
    /// # Returns
    /// `Some(())` if a message was processed, `None` if the queue is
    /// empty or incomplete.
    ///
    /// # Side Effects
    /// * Increments `client_limit`, `user_limit`, or `restricted_limit`.
    /// * May send game messages or log errors on handler failure.
    ///
    /// # Call Stack
    /// **Called by:** [`decode`](Self::decode)
    fn read(&mut self) -> Option<()> {
        let (prot, data, info) = {
            let handle = &mut self.handle;

            let opcode = handle
                .read_queue
                .pop_front()?
                .wrapping_sub(handle.isaac_decode.next_int() as u8);

            let prot = ClientProt::try_from(opcode).ok().or_else(|| {
                warn!("Unknown opcode: {}", opcode);
                None
            })?;

            let info = prot.info();

            let len: usize = match info.frame.0 {
                PacketFrame::VarByte => handle.read_queue.pop_front()? as usize,
                PacketFrame::VarShort => {
                    let hi = handle.read_queue.pop_front()? as usize;
                    let lo = handle.read_queue.pop_front()? as usize;
                    (hi << 8) | lo
                }
                PacketFrame::Fixed => info.frame.1.unwrap_or(0) as usize,
            };

            if handle.read_queue.len() < len {
                return None;
            }

            let data: Vec<u8> = handle.read_queue.drain(..len).collect();
            (prot, data, info)
        };

        #[rustfmt::skip]
        let success = {
            let mut buf = Packet::from(data);
            let len = buf.len();
            let result = match prot {
                ClientProt::AnticheatCycleLogic1 => AnticheatCycleLogic1::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatCycleLogic2 => AnticheatCycleLogic2::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatCycleLogic3 => AnticheatCycleLogic3::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatCycleLogic4 => AnticheatCycleLogic4::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatCycleLogic5 => AnticheatCycleLogic5::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatCycleLogic6 => AnticheatCycleLogic6::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic1 => AnticheatOpLogic1::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic2 => AnticheatOpLogic2::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic3 => AnticheatOpLogic3::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic4 => AnticheatOpLogic4::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic5 => AnticheatOpLogic5::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic6 => AnticheatOpLogic6::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic7 => AnticheatOpLogic7::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic8 => AnticheatOpLogic8::decode(&mut buf, len).handle(self),
                ClientProt::AnticheatOpLogic9 => AnticheatOpLogic9::decode(&mut buf, len).handle(self),
                ClientProt::ChatSetMode => ChatSetMode::decode(&mut buf, len).handle(self),
                ClientProt::ClientCheat => ClientCheat::decode(&mut buf, len).handle(self),
                ClientProt::CloseModal => CloseModal::decode(&mut buf, len).handle(self),
                #[cfg(rev = "225")]
                ClientProt::EventCameraPosition => EventCameraPosition::decode(&mut buf, len).handle(self),
                ClientProt::FriendListAdd => FriendListAdd::decode(&mut buf, len).handle(self),
                ClientProt::FriendListDel => FriendListDel::decode(&mut buf, len).handle(self),
                ClientProt::IdkSaveDesign => IdkSaveDesign::decode(&mut buf, len).handle(self),
                ClientProt::IdleTimer => IdleTimer::decode(&mut buf, len).handle(self),
                ClientProt::IfButton => IfButton::decode(&mut buf, len).handle(self),
                ClientProt::IgnoreListAdd => IgnoreListAdd::decode(&mut buf, len).handle(self),
                ClientProt::IgnoreListDel => IgnoreListDel::decode(&mut buf, len).handle(self),
                ClientProt::InvButton1 => InvButton1::decode(&mut buf, len).handle(self),
                ClientProt::InvButton2 => InvButton2::decode(&mut buf, len).handle(self),
                ClientProt::InvButton3 => InvButton3::decode(&mut buf, len).handle(self),
                ClientProt::InvButton4 => InvButton4::decode(&mut buf, len).handle(self),
                ClientProt::InvButton5 => InvButton5::decode(&mut buf, len).handle(self),
                ClientProt::InvButtonD => InvButtonD::decode(&mut buf, len).handle(self),
                ClientProt::MessagePrivate => MessagePrivate::decode(&mut buf, len).handle(self),
                ClientProt::MessagePublic => MessagePublic::decode(&mut buf, len).handle(self),
                ClientProt::MoveGameClick => MoveGameClick::decode(&mut buf, len).handle(self),
                ClientProt::MoveMinimapClick => MoveMinimapClick::decode(&mut buf, len).handle(self),
                ClientProt::MoveOpClick => MoveOpClick::decode(&mut buf, len).handle(self),
                ClientProt::NoTimeout => NoTimeout::decode(&mut buf, len).handle(self),
                ClientProt::OpHeld1 => OpHeld1::decode(&mut buf, len).handle(self),
                ClientProt::OpHeld2 => OpHeld2::decode(&mut buf, len).handle(self),
                ClientProt::OpHeld3 => OpHeld3::decode(&mut buf, len).handle(self),
                ClientProt::OpHeld4 => OpHeld4::decode(&mut buf, len).handle(self),
                ClientProt::OpHeld5 => OpHeld5::decode(&mut buf, len).handle(self),
                ClientProt::OpHeldT => OpHeldT::decode(&mut buf, len).handle(self),
                ClientProt::OpHeldU => OpHeldU::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc1 => OpLoc1::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc2 => OpLoc2::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc3 => OpLoc3::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc4 => OpLoc4::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc5 => OpLoc5::decode(&mut buf, len).handle(self),
                ClientProt::OpLocT => OpLocT::decode(&mut buf, len).handle(self),
                ClientProt::OpLocU => OpLocU::decode(&mut buf, len).handle(self),
                ClientProt::OpObj1 => OpObj1::decode(&mut buf, len).handle(self),
                ClientProt::OpObj2 => OpObj2::decode(&mut buf, len).handle(self),
                ClientProt::OpObj3 => OpObj3::decode(&mut buf, len).handle(self),
                ClientProt::OpObj4 => OpObj4::decode(&mut buf, len).handle(self),
                ClientProt::OpObj5 => OpObj5::decode(&mut buf, len).handle(self),
                ClientProt::OpObjT => OpObjT::decode(&mut buf, len).handle(self),
                ClientProt::OpObjU => OpObjU::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc1 => OpNpc1::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc2 => OpNpc2::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc3 => OpNpc3::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc4 => OpNpc4::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc5 => OpNpc5::decode(&mut buf, len).handle(self),
                ClientProt::OpNpcT => OpNpcT::decode(&mut buf, len).handle(self),
                ClientProt::OpNpcU => OpNpcU::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer1 => OpPlayer1::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer2 => OpPlayer2::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer3 => OpPlayer3::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer4 => OpPlayer4::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayerT => OpPlayerT::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayerU => OpPlayerU::decode(&mut buf, len).handle(self),
                #[cfg(rev = "225")]
                ClientProt::RebuildGetMaps => RebuildGetMaps::decode(&mut buf, len).handle(self),
                ClientProt::ResumePCountDialog => ResumePCountDialog::decode(&mut buf, len).handle(self),
                ClientProt::ResumePauseButton => ResumePauseButton::decode(&mut buf, len).handle(self),
                ClientProt::SendSnapshot => SendSnapshot::decode(&mut buf, len).handle(self),
                ClientProt::TutClickSide => TutClickSide::decode(&mut buf, len).handle(self),
                _ => Err(ScriptError::Client(format!("Unhandled opcode: {:?}", prot))),
            };
            match result {
                Ok(_) => true,
                Err(err) => {
                    #[cfg(debug_assertions)]
                    self.message_game_wrapped(&err.to_string());
                    error!("{err}");
                    false
                }
            }
        };

        match info.category {
            ClientProtCategory::ClientEvent => self.client_limit += 1,
            ClientProtCategory::UserEvent => {
                if success {
                    self.user_limit += 1
                }
            }
            ClientProtCategory::RestrictedEvent => self.restricted_limit += 1,
        }

        Some(())
    }

    /// Performs the initial login sequence for this player.
    ///
    /// Sends the map rebuild, chat filter settings, interface close, player
    /// ID, var cache reset, all varps, all stats, run energy, and animation
    /// reset packets.
    ///
    /// # Side Effects
    /// * Sends multiple server protocol packets to the client.
    ///
    /// # Call Stack
    /// **Calls:** [`rebuild_normal`](Self::rebuild_normal),
    /// [`chat_filter_settings`](Self::chat_filter_settings),
    /// [`if_close`](Self::if_close), [`update_pid`](Self::update_pid),
    /// [`reset_client_varcache`](Self::reset_client_varcache),
    /// [`sync_varps`](Self::sync_varps), [`update_stat`](Self::update_stat),
    /// [`update_runenergy`](Self::update_runenergy),
    /// [`reset_anims`](Self::reset_anims)
    fn on_login(&mut self) {
        self.rebuild_normal(false);
        self.chat_filter_settings(
            self.player.public as u8,
            self.player.private as u8,
            self.player.trade as u8,
        );
        self.if_close();
        self.update_pid(self.player.uid.pid());
        self.reset_client_varcache();
        self.sync_varps();
        for i in 0..self.player.stats.len() {
            self.update_stat(i);
        }
        self.update_runenergy(self.player.runenergy);
        self.reset_anims();
        let coord = self.player.pathing.coord;
        let snapshot_coord = CoordGrid::new(coord.x().saturating_sub(1), coord.y(), coord.z());
        self.player.pathing.last_step_coord = snapshot_coord;
        self.player.pathing.follow_coord = snapshot_coord;
    }

    /// Handles a client reconnection by re-sending all volatile state.
    ///
    /// On a reconnect (login response 15) the client resumes with whatever
    /// state it had when the connection dropped, so everything that may have
    /// changed during the gap is pushed again: the var cache is reset and all
    /// varps re-synced, the build area is rebuilt with a forced map rebuild
    /// (the client skips the reload when it is still in the same zone), any
    /// open modal is closed, tabs are re-assigned, every transmitted inventory
    /// is marked unseen so the next inventory flush sends full contents, all
    /// stats are re-sent, and animations are reset. Finally, the local player
    /// is flagged `tele`+`jump` so the next player-info packet re-places the
    /// client absolutely (its position may have drifted while disconnected).
    ///
    /// The engine-side per-observer view state (tracked players/NPCs) is reset
    /// by [`Engine::adopt_session`] before this runs, so the post-reconnect
    /// info packets re-add every visible entity from a clean slate.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`] if closing a modal fails.
    ///
    /// # Call Stack
    /// **Called by:** `Engine::adopt_session`
    /// **Calls:** [`reset_client_varcache`](Self::reset_client_varcache),
    /// [`sync_varps`](Self::sync_varps),
    /// [`rebuild_normal`](Self::rebuild_normal),
    /// [`close_modal`](Self::close_modal),
    /// [`sync_tabs`](Self::sync_tabs),
    /// [`update_stat`](Self::update_stat),
    /// [`update_runenergy`](Self::update_runenergy),
    /// [`reset_anims`](Self::reset_anims)
    fn on_reconnect(&mut self) -> Result<(), ScriptError> {
        self.reset_client_varcache();
        self.sync_varps();
        self.player.build_area.clear(true);
        self.rebuild_normal(true);
        self.close_modal(true)?;
        self.sync_tabs();
        self.player.inv_first_seen.clear();
        for i in 0..self.player.stats.len() {
            self.update_stat(i);
        }
        self.update_runenergy(self.player.runenergy);
        self.reset_anims();
        self.player.pathing.tele = true;
        self.player.pathing.jump = true;
        Ok(())
    }

    /// Dispatches a zone event message to the client by cloning and writing
    /// the appropriate server protocol variant (ObjAdd, ObjDel, LocAddChange,
    /// LocDel, MapAnim, etc.).
    ///
    /// # Arguments
    /// * `message` - The zone event to relay to the client.
    fn write_zone_message(&mut self, message: &ZoneMessage) {
        match message {
            ZoneMessage::ObjAdd(m) => self.write(m.clone()),
            ZoneMessage::ObjDel(m) => self.write(m.clone()),
            ZoneMessage::ObjCount(m) => self.write(m.clone()),
            ZoneMessage::ObjReveal(m) => self.write(m.clone()),
            ZoneMessage::LocAddChange(m) => self.write(m.clone()),
            ZoneMessage::LocDel(m) => self.write(m.clone()),
            ZoneMessage::LocAnim(m) => self.write(m.clone()),
            ZoneMessage::LocMerge(m) => self.write(m.clone()),
            ZoneMessage::MapAnim(m) => self.write(m.clone()),
            ZoneMessage::MapProjAnim(m) => self.write(m.clone()),
        }
    }

    /// Sends zone updates for all active zones in the player's build area.
    ///
    /// For newly loaded zones, sends a full zone clear followed by all
    /// visible objects and changed/despawned locations -- a complete snapshot.
    /// For already-loaded zones, sends only this tick's incremental shared
    /// (enclosed) and follows-type events visible to this player. The two are
    /// mutually exclusive per zone: a newly-loaded zone's snapshot already
    /// reflects this tick's changes, so replaying its delta events too would
    /// double-send them (a duplicate `ObjAdd`/loc update on the entry tick).
    ///
    /// # Arguments
    /// * `zones` - The zone map containing zone state and events.
    /// * `clock` - The current game tick, used for object visibility checks.
    ///
    /// # Side Effects
    /// * Marks zones as loaded in the player's build area.
    /// * Sends zone update packets to the client.
    fn update_zones(&mut self, zones: &ZoneMap, clock: u32) {
        let origin_x = self.player.build_area.origin.zone_origin_x();
        let origin_z = self.player.build_area.origin.zone_origin_z();
        let user37 = self.uid().username37();

        let build_area = &mut self.player.build_area;
        build_area
            .loaded_zones
            .retain(|z| build_area.active_zones.contains(z));

        let active_len = build_area.active_zones.len();
        let loaded_before = build_area.loaded_zones.len();

        for i in 0..active_len {
            let zone_coord = self.player.build_area.active_zones[i];
            if !rsmod::is_zone_allocated(zone_coord.x(), zone_coord.z(), zone_coord.y()) {
                continue;
            }
            // `zone_coord` is the zone's origin-tile coord -- the same value the
            // old code read back from `zone.coord` -- so compute the build-area
            // relative offset from it (works whether the zone exists).
            let x = zone_coord.x().wrapping_sub(origin_x) as u8;
            let z = zone_coord.z().wrapping_sub(origin_z) as u8;
            let newly_loaded =
                !self.player.build_area.loaded_zones[..loaded_before].contains(&zone_coord);

            // Read-only lookup. A zone that was never instantiated has no
            // objs/locs/events, so it yields exactly what the old `zone_mut`
            // path produced (a full-clear when newly loaded) -- without
            // creating and leaving an empty zone in the map.
            match zones.zone(zone_coord.x(), zone_coord.y(), zone_coord.z()) {
                Some(zone) => {
                    if newly_loaded {
                        self.write(UpdateZoneFullFollows { x, z });

                        let mut wrote_follows = false;

                        for obj in zone.visible_objs(user37, clock) {
                            if !wrote_follows {
                                self.write(UpdateZonePartialFollows { x, z });
                                wrote_follows = true;
                            }
                            self.write(ObjAdd {
                                coord: obj.packed_zone_coord(),
                                id: obj.id(),
                                count: obj.count() as u16,
                            });
                        }

                        for loc in &zone.locs {
                            let coord = loc.packed_zone_coord();
                            let shape_angle = loc.packed_shape_angle();
                            if loc.lifetime() == EntityLifeTime::Respawn && !loc.visible() {
                                if !wrote_follows {
                                    self.write(UpdateZonePartialFollows { x, z });
                                    wrote_follows = true;
                                }
                                self.write(LocDel { coord, shape_angle });
                            } else if loc.lifetime() == EntityLifeTime::Despawn || loc.is_changed()
                            {
                                if !wrote_follows {
                                    self.write(UpdateZonePartialFollows { x, z });
                                    wrote_follows = true;
                                }
                                self.write(LocAddChange {
                                    coord,
                                    shape_angle,
                                    id: loc.id(),
                                });
                            }
                        }
                    } else {
                        if let Some(bytes) = zone.shared_bytes() {
                            self.write(UpdateZonePartialEnclosed { x, z, bytes });
                        }

                        if zone.has_follows_events() {
                            self.write(UpdateZonePartialFollows { x, z });
                            for message in zone.visible_follows_events(user37) {
                                self.write_zone_message(message);
                            }
                        }
                    }
                }
                None => {
                    if newly_loaded {
                        self.write(UpdateZoneFullFollows { x, z });
                    }
                }
            }

            if !self.player.build_area.loaded_zones.contains(&zone_coord) {
                self.player.build_area.loaded_zones.push(zone_coord);
            }
        }
    }

    /// Closes all open modal interfaces (main, chat, side) and fires
    /// their `IfClose` script triggers.
    ///
    /// Optionally clears the weak script queue and always clears the
    /// protect flag (unless delayed). For each open modal, runs its
    /// `IfClose` trigger, clears interface listeners, and stops
    /// inventory transmits for the closed interface.
    ///
    /// # Arguments
    /// * `clear_weak_queue` - Whether to clear pending weak queue scripts.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`] if a trigger script fails.
    ///
    /// # Side Effects
    /// * Resets `modal_state` to `MODAL_NONE`.
    /// * Clears suspended scripts and interface listeners.
    /// * Sends `update_inv_stop_transmit` for cleared inventory bindings.
    /// * Sets `refresh_modal_close = true`.
    fn close_modal(&mut self, clear_weak_queue: bool) -> Result<(), ScriptError> {
        if clear_weak_queue {
            self.player.state.queues.weak.clear();
        }

        if !self.player.state.delayed {
            self.player.state.protect = false;
        }

        if self.player.modal_state == MODAL_NONE {
            return Ok(());
        }

        self.player.modal_state = MODAL_NONE;
        self.player.clear_suspended_script();

        if let Some(modal_main) = self.player.modal_main {
            if let Err(e) = engine_mut().run_script_by_trigger(
                (ServerTriggerType::IfClose, Some(modal_main), None),
                Some(ScriptSubject::Player(self.player.uid)),
                None,
                Some(false),
                Some(false),
                None,
            ) && !matches!(e, ScriptError::TriggerNotFound(_))
            {
                return Err(e);
            }
            let modal = self.player.modal_main;
            let cleared = self.player.clear_interface_listeners(modal, |com| {
                cache().interfaces.get_by_id(com).map(|i| i.root_layer)
            });
            for com in cleared {
                self.clear_inv_transmits(com);
            }
            self.player.modal_main = None;
        }

        if let Some(modal_chat) = self.player.modal_chat {
            if let Err(e) = engine_mut().run_script_by_trigger(
                (ServerTriggerType::IfClose, Some(modal_chat), None),
                Some(ScriptSubject::Player(self.player.uid)),
                None,
                Some(false),
                Some(false),
                None,
            ) && !matches!(e, ScriptError::TriggerNotFound(_))
            {
                return Err(e);
            }
            let modal = self.player.modal_chat;
            let cleared = self.player.clear_interface_listeners(modal, |com| {
                cache().interfaces.get_by_id(com).map(|i| i.root_layer)
            });
            for com in cleared {
                self.clear_inv_transmits(com);
            }
            self.player.modal_chat = None;
        }

        if let Some(modal_side) = self.player.modal_side {
            if let Err(e) = engine_mut().run_script_by_trigger(
                (ServerTriggerType::IfClose, Some(modal_side), None),
                Some(ScriptSubject::Player(self.player.uid)),
                None,
                Some(false),
                Some(false),
                None,
            ) && !matches!(e, ScriptError::TriggerNotFound(_))
            {
                return Err(e);
            }
            let modal = self.player.modal_side;
            let cleared = self.player.clear_interface_listeners(modal, |com| {
                cache().interfaces.get_by_id(com).map(|i| i.root_layer)
            });
            for com in cleared {
                self.clear_inv_transmits(com);
            }
            self.player.modal_side = None;
        }

        self.player.refresh_modal_close = true;
        Ok(())
    }

    /// Clears the player's current interaction target and closes any open
    /// modal interfaces.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`] from modal close.
    ///
    /// # Call Stack
    /// **Calls:** [`close_modal`](Self::close_modal)
    fn clear_pending_action(&mut self) -> Result<(), ScriptError> {
        self.player.clear_interaction();
        self.close_modal(true)?;
        Ok(())
    }

    /// Fully stops the player: clears all waypoints and pending actions.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`] from clearing pending actions.
    ///
    /// # Call Stack
    /// **Calls:** [`clear_waypoints`](Self::clear_waypoints),
    /// [`clear_pending_action`](Self::clear_pending_action)
    fn stop_action(&mut self) -> Result<(), ScriptError> {
        self.clear_waypoints();
        self.clear_pending_action()
    }

    /// Teleports the player to the given coordinate with a visible jump
    /// animation on the client.
    ///
    /// # Arguments
    /// * `coord` - The destination coordinate.
    ///
    /// # Side Effects
    /// * Sets `player.jump = true` and delegates to [`tele`](Self::tele).
    ///
    /// # Call Stack
    /// **Calls:** [`tele`](Self::tele)
    fn tele_jump(&mut self, coord: CoordGrid) {
        self.tele(coord);
        self.player.pathing.jump = true;
    }

    /// Teleports the player to the given coordinate.
    ///
    /// If the target zone is not allocated, sends an "Invalid teleport!"
    /// game message instead of teleporting. If the destination is on a
    /// different level, a jump flag is set.
    ///
    /// # Arguments
    /// * `coord` - The destination coordinate.
    ///
    /// # Side Effects
    /// * Sets `player.pathing.coord` and `player.pathing.tele = true`.
    /// * Sets `player.jump = true` when changing levels.
    fn tele(&mut self, coord: CoordGrid) {
        match self.player.pathing.teleport(coord) {
            None => self.message_game("Invalid teleport!"),
            Some((look_x, look_z)) => {
                self.player.info.focus(
                    FocusKind::Player,
                    CoordGrid::fine(look_x, self.player.pathing.size),
                    CoordGrid::fine(look_z, self.player.pathing.size),
                    false,
                );
            }
        }
    }
}
