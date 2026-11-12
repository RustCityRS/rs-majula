use crate::clients::client_db::{AUTO_BAN_HOURS, DbRequest};
use crate::clients::client_game::{ClientHandle, MAX_UNSENT};
use crate::core::{Engine, OtherPlayers, engine, engine_mut};
use crate::counters::{self, AutoBan, COUNTERS};
use crate::handlers::ClientGameHandler;
#[cfg(since_647)]
use crate::handlers::reflection_check_reply::ReflectionChecks;
use crate::pathfinding::PendingPath;
use crate::phases::logout::TIMEOUT_NO_RESPONSE;
use crate::phases::shared::EntityId;
#[cfg(since_647)]
use crate::region::{REGION_X, template_square};
#[cfg(before_274)]
use crate::watch::TRACKING_TICKS;
use crate::watch::menu::InvOp;
use crate::watch::schedule::End;
use crate::watch::{MapBase, Op, Target, Using, Watch, menu};
use mem::take;
use rs_cam::CamKind;
use rs_db::{ChatLog, LogBatch, SessionLog, WealthEvent};
use rs_entity::player::*;
use rs_entity::{EntityLifeTime, Player};
use rs_grid::{CoordGrid, ZoneCoordGrid};
use rs_info::FocusKind;
#[cfg(since_647)]
use rs_info::PlayerRenderer;
use rs_inv::Inventory;
use rs_io::{Packet, PacketFrame};
use rs_pack::cache::inv::InvScope;
#[cfg(since_647)]
use rs_pack::cache::map::XteaKeyStore;
#[cfg(since_254)]
use rs_pack::cache::varbit::VarbitType;
use rs_pack::cache::{ScriptVarType, VarValue};
#[cfg(since_289)]
use rs_pack::types::MultiVar;
use rs_pack::types::{Font, PlayerStat};
use rs_protocol::network::game::MAX_PACKET_SIZE;
#[cfg(before_647)]
use rs_protocol::network::game::client::move_minimapclick::MinimapTrailer;
use rs_protocol::network::game::client::{ClientProtMessage, unpack_coord};
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatCycleLogic1;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatCycleLogic2;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatCycleLogic3;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatCycleLogic4;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatCycleLogic5;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatCycleLogic6;
#[cfg(all(since_254, before_647))]
use rs_protocol::network::game::client_prot::AnticheatCycleLogic7;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic1;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic2;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic3;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic4;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic5;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic6;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic7;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic8;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::AnticheatOpLogic9;
use rs_protocol::network::game::client_prot::ChatSetMode;
use rs_protocol::network::game::client_prot::ClientCheat;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::client_prot::CloseModal;
#[cfg(since_254)]
use rs_protocol::network::game::client_prot::EventAppletFocus;
#[cfg(any(rev = "225", since_254))]
use rs_protocol::network::game::client_prot::EventCameraPosition;
#[cfg(since_254)]
use rs_protocol::network::game::client_prot::EventMouseClick;
#[cfg(since_254)]
use rs_protocol::network::game::client_prot::EventMouseMove;
#[cfg(before_274)]
use rs_protocol::network::game::client_prot::EventTracking;
use rs_protocol::network::game::client_prot::FriendListAdd;
use rs_protocol::network::game::client_prot::FriendListDel;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::IdkSaveDesign;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::IdleTimer;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::IfButton;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::IfButtonD;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::IfButtonT;
use rs_protocol::network::game::client_prot::IgnoreListAdd;
use rs_protocol::network::game::client_prot::IgnoreListDel;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::InvButton1;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::InvButton2;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::InvButton3;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::InvButton4;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::InvButton5;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::InvButtonD;
#[cfg(since_254)]
use rs_protocol::network::game::client_prot::MapBuildComplete;
use rs_protocol::network::game::client_prot::MessagePrivate;
use rs_protocol::network::game::client_prot::MessagePublic;
use rs_protocol::network::game::client_prot::MoveGameClick;
use rs_protocol::network::game::client_prot::MoveMinimapClick;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::MoveOpClick;
use rs_protocol::network::game::client_prot::NoTimeout;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpHeld1;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpHeld2;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpHeld3;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpHeld4;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpHeld5;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpHeldT;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpHeldU;
use rs_protocol::network::game::client_prot::OpLoc1;
use rs_protocol::network::game::client_prot::OpLoc2;
use rs_protocol::network::game::client_prot::OpLoc3;
use rs_protocol::network::game::client_prot::OpLoc4;
use rs_protocol::network::game::client_prot::OpLoc5;
use rs_protocol::network::game::client_prot::OpLocT;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpLocU;
use rs_protocol::network::game::client_prot::OpNpc1;
use rs_protocol::network::game::client_prot::OpNpc2;
use rs_protocol::network::game::client_prot::OpNpc3;
use rs_protocol::network::game::client_prot::OpNpc4;
use rs_protocol::network::game::client_prot::OpNpc5;
use rs_protocol::network::game::client_prot::OpNpcT;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpNpcU;
use rs_protocol::network::game::client_prot::OpObj1;
use rs_protocol::network::game::client_prot::OpObj2;
use rs_protocol::network::game::client_prot::OpObj3;
use rs_protocol::network::game::client_prot::OpObj4;
use rs_protocol::network::game::client_prot::OpObj5;
use rs_protocol::network::game::client_prot::OpObjT;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpObjU;
use rs_protocol::network::game::client_prot::OpPlayer1;
use rs_protocol::network::game::client_prot::OpPlayer2;
use rs_protocol::network::game::client_prot::OpPlayer3;
use rs_protocol::network::game::client_prot::OpPlayer4;
#[cfg(since_254)]
use rs_protocol::network::game::client_prot::OpPlayer5;
use rs_protocol::network::game::client_prot::OpPlayerT;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::OpPlayerU;
#[cfg(rev = "225")]
use rs_protocol::network::game::client_prot::RebuildGetMaps;
use rs_protocol::network::game::client_prot::ResumePCountDialog;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::ResumePStringDialog;
use rs_protocol::network::game::client_prot::ResumePauseButton;
use rs_protocol::network::game::client_prot::SendSnapshot;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::SetChatfiltersettings;
#[cfg(before_647)]
use rs_protocol::network::game::client_prot::TutClickSide;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::WindowStatus;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::{
    ApCoordT, ClanJoinchatLeavechat, ClanKickUser, ClanchannelKickUser, ClickWorldMap,
    ClientDetailoptionsStatus, CreateCheckName, EventKeyboard, FaceSquare, FriendSetRank,
    MapBuildLoading, MessageQuickchatPrivate, MessageQuickchatPublic, OpPlayer6, OpPlayer7,
    OpPlayer8, OpPlayer9, OpPlayer10, ReflectionCheckReply, ResumePClanforumqfcDialog,
    ResumePHslDialog, ResumePNameDialog, ResumePObjDialog, SoundSongPreloaded, TransmitvarVerifyId,
    UrlRequest, VideoEnd, WorldLatency, WorldlistFetch,
};
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::{
    IfButton1, IfButton2, IfButton3, IfButton4, IfButton5, IfButton6, IfButton7, IfButton8,
    IfButton9, IfButton10,
};
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::{OpLoc6, OpNpc6, OpObj6};
use rs_protocol::network::game::client_prot_category::ClientProtCategory;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;
use rs_protocol::network::game::info::player::appearance::{Appearance, Look, WEAR_SLOTS, Wear};
use rs_protocol::network::game::info_prot::PlayerInfoProt;
use rs_protocol::network::game::server::ServerProtMessage;
#[cfg(before_647)]
use rs_protocol::network::game::server::cam_look_at::CamLookAt;
#[cfg(since_647)]
use rs_protocol::network::game::server::cam_lookat::CamLookat;
#[cfg(before_647)]
use rs_protocol::network::game::server::cam_move_to::CamMoveTo;
#[cfg(since_647)]
use rs_protocol::network::game::server::cam_moveto::CamMoveto;
use rs_protocol::network::game::server::cam_reset::CamReset;
use rs_protocol::network::game::server::cam_shake::CamShake;
use rs_protocol::network::game::server::chat_filter_settings::ChatFilterSettings;
#[cfg(since_647)]
use rs_protocol::network::game::server::chat_filter_settings_privatechat::ChatFilterSettingsPrivatechat;
#[cfg(rev = "225")]
use rs_protocol::network::game::server::data_land::DataLand;
#[cfg(rev = "225")]
use rs_protocol::network::game::server::data_land_done::DataLandDone;
#[cfg(rev = "225")]
use rs_protocol::network::game::server::data_loc::DataLoc;
#[cfg(rev = "225")]
use rs_protocol::network::game::server::data_loc_done::DataLocDone;
#[cfg(before_274)]
use rs_protocol::network::game::server::enable_tracking::EnableTracking;
#[cfg(before_274)]
use rs_protocol::network::game::server::finish_tracking::FinishTracking;
#[cfg(all(since_254, before_647))]
use rs_protocol::network::game::server::friendlist_loaded::FriendListLoaded;
#[cfg(since_647)]
use rs_protocol::network::game::server::friendlist_loaded::FriendlistLoaded;
use rs_protocol::network::game::server::hint_arrow::HintArrow;
#[cfg(since_647)]
use rs_protocol::network::game::server::hint_arrow::HintArrowKind;
#[cfg(before_647)]
use rs_protocol::network::game::server::if_close::IfClose;
#[cfg(since_647)]
use rs_protocol::network::game::server::if_closesub::IfClosesub;
#[cfg(since_647)]
use rs_protocol::network::game::server::if_movesub::IfMovesub;
#[cfg(before_647)]
use rs_protocol::network::game::server::if_openchat::IfOpenChat;
#[cfg(before_647)]
use rs_protocol::network::game::server::if_openmain::IfOpenMain;
#[cfg(before_647)]
use rs_protocol::network::game::server::if_openmain_side::IfOpenMainSide;
#[cfg(all(since_244, before_647))]
use rs_protocol::network::game::server::if_openoverlay::IfOpenOverlay;
#[cfg(before_647)]
use rs_protocol::network::game::server::if_openside::IfOpenSide;
#[cfg(since_647)]
use rs_protocol::network::game::server::if_opensub::IfOpensub;
#[cfg(since_647)]
use rs_protocol::network::game::server::if_opentop::IfOpentop;
use rs_protocol::network::game::server::if_setanim::IfSetAnim;
use rs_protocol::network::game::server::if_setcolour::IfSetColour;
#[cfg(since_647)]
use rs_protocol::network::game::server::if_setevents::IfSetEvents;
#[cfg(since_647)]
use rs_protocol::network::game::server::if_setgraphic::IfSetGraphic;
use rs_protocol::network::game::server::if_sethide::IfSetHide;
use rs_protocol::network::game::server::if_setmodel::IfSetModel;
use rs_protocol::network::game::server::if_setnpchead::IfSetNpcHead;
use rs_protocol::network::game::server::if_setobject::IfSetObject;
use rs_protocol::network::game::server::if_setplayerhead::IfSetPlayerHead;
use rs_protocol::network::game::server::if_setposition::IfSetPosition;
#[cfg(before_245_2)]
use rs_protocol::network::game::server::if_setrecol::IfSetRecol;
#[cfg(since_245_2)]
use rs_protocol::network::game::server::if_setscrollpos::IfSetScrollPos;
#[cfg(before_647)]
use rs_protocol::network::game::server::if_settab::IfSetTab;
#[cfg(before_647)]
use rs_protocol::network::game::server::if_settab_active::IfSetTabActive;
use rs_protocol::network::game::server::if_settext::IfSetText;
use rs_protocol::network::game::server::last_login_info::LastLoginInfo;
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
use rs_protocol::network::game::server::loc_del::LocDel;
use rs_protocol::network::game::server::logout::Logout;
use rs_protocol::network::game::server::message_game::MessageGame;
#[cfg(since_647)]
use rs_protocol::network::game::server::message_public;
#[cfg(any(rev = "225", since_244))]
use rs_protocol::network::game::server::midi_jingle::MidiJingle;
#[cfg(any(rev = "225", since_244))]
use rs_protocol::network::game::server::midi_song::MidiSong;
#[cfg(since_274)]
use rs_protocol::network::game::server::minimap_toggle::MinimapToggle;
use rs_protocol::network::game::server::npc_info::NpcInfo;
use rs_protocol::network::game::server::obj_add::ObjAdd;
#[cfg(before_647)]
use rs_protocol::network::game::server::p_countdialog::PCountDialog;
use rs_protocol::network::game::server::player_info::PlayerInfo;
#[cfg(any(rev = "225", since_244))]
use rs_protocol::network::game::server::rebuild_normal::RebuildNormal;
#[cfg(since_647)]
use rs_protocol::network::game::server::rebuild_region::RebuildRegion;
#[cfg(before_647)]
use rs_protocol::network::game::server::reset_anims::ResetAnims;
#[cfg(before_647)]
use rs_protocol::network::game::server::reset_client_varcache::ResetClientVarCache;
#[cfg(since_647)]
use rs_protocol::network::game::server::run_client_script::{RunClientScript, ScriptParam};
#[cfg(since_647)]
use rs_protocol::network::game::server::set_map_flag::SetMapFlag;
#[cfg(before_647)]
use rs_protocol::network::game::server::set_multiway::SetMultiway;
#[cfg(since_254)]
use rs_protocol::network::game::server::set_player_op::SetPlayerOp;
use rs_protocol::network::game::server::synth_sound::SynthSound;
#[cfg(before_647)]
use rs_protocol::network::game::server::tut_flash::TutFlash;
#[cfg(before_647)]
use rs_protocol::network::game::server::tut_open::TutOpen;
#[cfg(before_647)]
use rs_protocol::network::game::server::unset_map_flag::UnsetMapFlag;
#[cfg(since_647)]
use rs_protocol::network::game::server::update_inv_full::InvItem;
use rs_protocol::network::game::server::update_inv_full::UpdateInvFull;
use rs_protocol::network::game::server::update_inv_partial::UpdateInvPartial;
#[cfg(before_647)]
use rs_protocol::network::game::server::update_inv_stop_transmit::UpdateInvStopTransmit;
#[cfg(any(rev = "225", all(since_244, before_647)))]
use rs_protocol::network::game::server::update_pid::UpdatePid;
use rs_protocol::network::game::server::update_runenergy::UpdateRunEnergy;
#[cfg(before_647)]
use rs_protocol::network::game::server::update_runweight::UpdateRunWeight;
use rs_protocol::network::game::server::update_stat::UpdateStat;
use rs_protocol::network::game::server::update_zone_full_follows::UpdateZoneFullFollows;
use rs_protocol::network::game::server::update_zone_partial_enclosed::UpdateZonePartialEnclosed;
use rs_protocol::network::game::server::update_zone_partial_follows::UpdateZonePartialFollows;
#[cfg(since_647)]
use rs_protocol::network::game::server::varc_large::VarcLarge;
#[cfg(since_647)]
use rs_protocol::network::game::server::varc_small::VarcSmall;
#[cfg(since_647)]
use rs_protocol::network::game::server::varc_str_large::VarcStrLarge;
#[cfg(since_647)]
use rs_protocol::network::game::server::varc_str_small::VarcStrSmall;
use rs_protocol::network::game::server::varp_large::VarpLarge;
use rs_protocol::network::game::server::varp_small::VarpSmall;
#[cfg(since_647)]
use rs_protocol::network::game::server::vorbis_speech_sound::VorbisSpeechSound;
#[cfg(since_647)]
use rs_protocol::network::game::server::vorbis_speech_stop::VorbisSpeechStop;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_protocol::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_254)]
use rs_util::bits::{fits_bit_range, getbit_range, setbit_range_toint};
use rs_var::VarSet;
use rs_vm::engine::{ScriptEngine, ScriptPlayer};
use rs_vm::state::QueuePriority;
#[cfg(since_647)]
use rs_vm::state::ScriptArgument;
use rs_vm::state::ScriptState;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ComId, ScriptError};
use rs_zone::ZoneMessage;
use rs_zone::zone_map::ZoneMap;
#[cfg(before_647)]
use rsmod::rsmod::flag::zone_flag::ZoneFlag;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::cell::RefCell;
#[cfg(since_647)]
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write as _;
use std::mem;
use std::net::IpAddr;
use std::str;
use std::time::{Duration, Instant};
use tracing::{debug, error, warn};

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
    pub buffered: Packet,
    pub buffered_starts: Vec<u32>,
    pub client_limit: u8,
    pub user_limit: u8,
    pub restricted_limit: u8,
    pub remote_ip: Option<IpAddr>,
    pub muted_until: Option<Instant>,
    pub unread_messages: u16,
    pub last_report: Option<u32>,
    pub last_social: Option<u32>,
    pub friends: Vec<i64>,
    pub ignores: Vec<i64>,
    pub watch: Box<Watch>,
    pub(crate) capture: Option<Vec<Vec<u8>>>,
    pub(crate) pending_path: Option<PendingPath>,
    #[cfg(since_647)]
    pub(crate) reflection: ReflectionChecks,
}

/// The interface a component is on. From 647 a component is addressed as
/// `interface << 16 | component`, so that is the hash's high half.
fn root_layer(com: ComId) -> Option<i32> {
    #[cfg(before_647)]
    let root = engine()
        .interfaces()
        .get_by_id(com as u16)
        .map(|i| i.root_layer);
    #[cfg(since_647)]
    let root = Some(com >> 16);
    root
}

/// The wear positions (`equip.constant`) that show the body kits.
const WEARPOS_TORSO: usize = 4;
const WEARPOS_ARMS: usize = 6;
const WEARPOS_LEGS: usize = 7;
const WEARPOS_HEAD: usize = 8;
const WEARPOS_HANDS: usize = 9;
const WEARPOS_FEET: usize = 10;
const WEARPOS_JAW: usize = 11;

/// Where each of the player's seven body kits shows, in `body` order.
const BODY_WEARPOS: [usize; 7] = [
    WEARPOS_HEAD,
    WEARPOS_JAW,
    WEARPOS_TORSO,
    WEARPOS_ARMS,
    WEARPOS_HANDS,
    WEARPOS_LEGS,
    WEARPOS_FEET,
];

/// The BAS a player has until scripts set one: 2011's unarmed stance.
#[cfg(since_647)]
const DEFAULT_BAS: u16 = 1426;

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
        let varps = engine().varps();
        let vars = VarSet::new((0..varps.count()).map(|id| {
            varps
                .get_by_id(id as u16)
                .map(|v| v.var_type)
                .unwrap_or(ScriptVarType::Int)
        }));

        let mut player = Player::new(uid, coord, vars, bot);
        #[cfg(since_647)]
        {
            player.varcs = VarSet::new(engine().varcs().types().map(|v| v.var_type));
        }
        player.low_memory = low_memory;

        ActivePlayer {
            player,
            handle: Box::new(handle),
            buffered: Packet::new(0),
            buffered_starts: Vec::new(),
            client_limit: 0,
            user_limit: 0,
            restricted_limit: 0,
            remote_ip: None,
            muted_until: None,
            unread_messages: 0,
            last_report: None,
            last_social: None,
            friends: Vec::new(),
            ignores: Vec::new(),
            watch: Box::default(),
            capture: None,
            pending_path: None,
            #[cfg(since_647)]
            reflection: ReflectionChecks::default(),
        }
    }

    /// Sends a server protocol message to the client, routing it through
    /// the appropriate priority channel.
    ///
    /// Buffered-priority messages are queued and flushed at the end of the
    /// tick via [`encode`](Self::encode). Immediate-priority messages are
    /// sent right away, or during a tick join the player's output, which goes
    /// out as one write.
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
        if let Some(capture) = &mut self.capture {
            if let Some(buf) = Self::plaintext(message) {
                capture.push(buf.data);
            }
            return;
        }
        match M::PRIORITY {
            ServerProtPriority::Buffered => self.queue_buffered(message),
            ServerProtPriority::Immediate => self.write_immediate(message),
        }
    }

    /// Encodes a server protocol message into a packet and queues it for
    /// sending at end-of-tick.
    ///
    /// The opcode byte, frame header, and payload are written into a new
    /// [`Packet`]. Messages larger than [`MAX_PACKET_SIZE`] are silently dropped.
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
        let start = self.buffered.pos;
        if !Self::encode_into(&mut self.buffered, message) {
            return;
        }
        self.buffered_starts.push(start as u32);
        let rest = &self.buffered.data[start + 1..self.buffered.pos];
        self.watch.remember(M::PROT as u8, M::FRAME as usize, rest);
    }

    /// [`plaintext`](Self::plaintext) written at the end of `out`, which
    /// grows to fit. Whether it fit in a packet.
    fn encode_into<M>(out: &mut Packet, message: M) -> bool
    where
        M: ServerProtMessage,
    {
        let frame = M::FRAME as usize;
        #[cfg(before_647)]
        let opcode_bytes = 1;
        #[cfg(since_647)]
        let opcode_bytes = if (M::PROT as u16) < 128 { 1 } else { 2 };
        let len = opcode_bytes + frame + message.sizeof();
        if len > MAX_PACKET_SIZE {
            return false;
        }
        if out.data.len() < out.pos + len {
            out.data.resize(out.pos + len, 0);
        }
        #[cfg(before_647)]
        out.p1(M::PROT as u8);
        #[cfg(since_647)]
        {
            let opcode = M::PROT as u16;
            if opcode < 128 {
                out.p1(opcode as u8);
            } else {
                out.p1((opcode >> 8) as u8 + 128);
                out.p1(opcode as u8);
            }
        }
        out.pos += frame;
        let start = out.pos;
        message.encode(out);
        match M::FRAME {
            PacketFrame::Fixed => {}
            PacketFrame::VarByte => out.psize1((out.pos - start) as u8),
            PacketFrame::VarShort => out.psize2((out.pos - start) as u16),
        }
        true
    }

    /// A message as a packet: opcode (unciphered), frame and body. `None`
    /// past [`MAX_PACKET_SIZE`].
    fn plaintext<M>(message: M) -> Option<Packet>
    where
        M: ServerProtMessage,
    {
        let frame = M::FRAME as usize;
        #[cfg(before_647)]
        let opcode_bytes = 1;
        // Rev-647 opcodes are 1-or-2-byte SMART values on the wire.
        #[cfg(since_647)]
        let opcode_bytes = if (M::PROT as u16) < 128 { 1 } else { 2 };
        let len = opcode_bytes + frame + message.sizeof();
        if len > MAX_PACKET_SIZE {
            return None;
        }
        let mut buf = Packet::new(len);
        #[cfg(before_647)]
        buf.p1(M::PROT as u8);
        #[cfg(since_647)]
        {
            let opcode = M::PROT as u16;
            if opcode < 128 {
                buf.p1(opcode as u8);
            } else {
                buf.p1((opcode >> 8) as u8 + 128);
                buf.p1(opcode as u8);
            }
        }
        buf.pos += frame;
        let start = buf.pos;
        message.encode(&mut buf);
        match M::FRAME {
            PacketFrame::Fixed => {}
            PacketFrame::VarByte => buf.psize1((buf.pos - start) as u8),
            PacketFrame::VarShort => buf.psize2((buf.pos - start) as u16),
        }
        buf.data.truncate(buf.pos);
        Some(buf)
    }

    /// Flushes all queued buffered packets to the network outbox, with the
    /// rest of the tick's output, as one write.
    ///
    /// Each packet's opcode byte is ISAAC-encrypted before sending.
    ///
    /// # Side Effects
    /// * Drains `self.buffered` and sends the tick's output to the socket task.
    ///
    /// # Call Stack
    /// **Called by:** [`encode`](Self::encode)
    fn write_buffered(&mut self) {
        let handle = &mut self.handle;
        let clock = engine().clock;
        let watched = cfg!(before_274) || self.watch.recording();
        let end = self.buffered.pos;
        let data = &mut self.buffered.data;
        for (i, &start) in self.buffered_starts.iter().enumerate() {
            let start = start as usize;
            let next = self
                .buffered_starts
                .get(i + 1)
                .map_or(end, |&next| next as usize);
            if watched {
                self.watch
                    .server(clock, data[start], &data[start + 1..next]);
            }
            // Rev-647 opcodes were queued as plaintext SMART: a first byte
            // >= 128 marks a 2-byte opcode, which consumes two ISAAC values.
            // ISAAC is applied here (at flush) rather than at queue time so
            // the cipher stays in wire order relative to immediate packets.
            #[cfg(since_647)]
            if data[start] >= 128 {
                let i0 = handle.isaac_encode.next_int();
                let i1 = handle.isaac_encode.next_int();
                data[start] = (data[start] as u32 + i0) as u8;
                data[start + 1] = (data[start + 1] as u32 + i1) as u8;
                continue;
            }
            data[start] = (data[start] as u32 + handle.isaac_encode.next_int()) as u8;
        }
        if end > 0 {
            handle.gather(&data[..end]);
            data[..end].fill(0);
        }
        self.buffered.pos = 0;
        self.buffered_starts.clear();
        handle.flush();
    }

    /// Encodes and immediately sends a server protocol message to the client,
    /// bypassing the buffered queue.
    ///
    /// Uses the shared `handle.write_queue` buffer. The opcode byte is
    /// ISAAC-encrypted before sending. Messages larger than [`MAX_PACKET_SIZE`]
    /// are silently dropped. During a tick the message joins the player's
    /// output instead (see
    /// [`Engine::gather_output`](crate::core::Engine::gather_output)).
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
        #[cfg(before_647)]
        let opcode_bytes = 1;
        #[cfg(since_647)]
        let opcode_bytes = if (M::PROT as u16) < 128 { 1 } else { 2 };
        let len = opcode_bytes + frame + message.sizeof();
        if len > MAX_PACKET_SIZE {
            return;
        }
        let buf = &mut handle.write_queue;
        buf.pos = 0;
        #[cfg(before_647)]
        buf.p1((M::PROT as u32 + handle.isaac_encode.next_int()) as u8);
        // 1-or-2-byte SMART opcode, ISAAC applied per byte.
        #[cfg(since_647)]
        {
            let opcode = M::PROT as u16;
            if opcode < 128 {
                buf.p1((opcode as u32 + handle.isaac_encode.next_int()) as u8);
            } else {
                buf.p1((((opcode >> 8) + 128) as u32 + handle.isaac_encode.next_int()) as u8);
                buf.p1(((opcode & 0xFF) as u32 + handle.isaac_encode.next_int()) as u8);
            }
        }
        buf.pos += frame;
        let start = buf.pos;
        message.encode(buf);
        match M::FRAME {
            PacketFrame::Fixed => {}
            PacketFrame::VarByte => buf.psize1((buf.pos - start) as u8),
            PacketFrame::VarShort => buf.psize2((buf.pos - start) as u16),
        }
        self.watch
            .server(engine().clock, M::PROT as u8, &buf.data[1..buf.pos]);
        self.watch
            .remember(M::PROT as u8, frame, &buf.data[1..buf.pos]);
        let pos = buf.pos;
        handle.gather_written(pos);
        if !engine().gather_output {
            handle.flush();
        }
    }

    /// [`update_zones`](EnginePlayer::update_zones) when neither zone list
    /// has changed since its last full pass and no zone it couldn't load has
    /// collision data now: no zone needs loading, so only the zones in
    /// `changed` can have updates. `false` if it needs the full pass.
    fn update_settled_zones(
        &mut self,
        zones: &ZoneMap,
        changed: &FxHashSet<ZoneCoordGrid>,
    ) -> bool {
        let Some(unloaded) = self.player.build_area.zones_settled else {
            return false;
        };
        let mut bits = unloaded;
        while bits != 0 {
            let zone = self.player.build_area.active_zones[bits.trailing_zeros() as usize];
            if rsmod::is_zone_allocated(zone.x(), zone.z(), zone.y()) {
                return false;
            }
            bits &= bits - 1;
        }
        if changed.is_empty() {
            return true;
        }

        let origin_x = self.player.build_area.origin.zone_origin_x();
        let origin_z = self.player.build_area.origin.zone_origin_z();
        let user37 = self.uid().username37();
        for i in 0..self.player.build_area.active_zones.len() {
            let zone_coord = self.player.build_area.active_zones[i];
            if unloaded >> i & 1 != 0 {
                continue;
            }
            if !changed.contains(&zone_coord) {
                debug_assert!(
                    zones
                        .zone(zone_coord.x(), zone_coord.y(), zone_coord.z())
                        .is_none_or(
                            |zone| zone.shared_bytes().is_none() && !zone.has_follows_events()
                        ),
                    "zone {zone_coord:?} has updates but isn't tracked"
                );
                continue;
            }
            if !rsmod::is_zone_allocated(zone_coord.x(), zone_coord.z(), zone_coord.y()) {
                continue;
            }
            let Some(zone) = zones.zone(zone_coord.x(), zone_coord.y(), zone_coord.z()) else {
                continue;
            };
            let x = zone_coord.x().wrapping_sub(origin_x) as u8;
            let z = zone_coord.z().wrapping_sub(origin_z) as u8;
            #[cfg(since_647)]
            let y = zone_coord.y();
            // Rev-647 zone headers carry the zone index, as in `update_zones`:
            // a tile offset here put every change outside the build area.
            #[cfg(since_647)]
            let (x, z) = (x >> 3, z >> 3);
            if let Some(bytes) = zone.shared_bytes() {
                #[cfg(before_647)]
                self.write(UpdateZonePartialEnclosed { x, z, bytes });
                #[cfg(since_647)]
                self.write(UpdateZonePartialEnclosed { x, z, y, bytes });
            }
            if zone.has_follows_events() {
                #[cfg(before_647)]
                self.write(UpdateZonePartialFollows { x, z });
                #[cfg(since_647)]
                self.write(UpdateZonePartialFollows { x, z, y });
                for message in zone.visible_follows_events(user37) {
                    self.write_zone_message(message);
                }
            }
        }
        true
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

        // The client's input report: sent now, then from a fresh start.
        #[cfg(before_274)]
        if engine().clock.is_multiple_of(TRACKING_TICKS) {
            self.write(FinishTracking);
            self.write(EnableTracking);
        }

        self.watch_acting_on();
        self.write_buffered();
        if self.handle.backlogged() {
            warn!("Over {MAX_UNSENT} bytes left unwritten, closing the connection");
            self.disconnect();
        }
        self.watch.end_tick(engine().clock);
    }

    pub fn cam_moveto(&mut self, x: u8, z: u8, height: u16, rate: u8, rate2: u8) {
        #[cfg(before_647)]
        self.write(CamMoveTo {
            x,
            z,
            height,
            rate,
            rate2,
        });
        #[cfg(since_647)]
        self.write(CamMoveto {
            local_x: x,
            local_z: z,
            height,
            const_speed: rate,
            var_speed: rate2,
        });
    }

    pub fn cam_lookat(&mut self, x: u8, z: u8, height: u16, rate: u8, rate2: u8) {
        #[cfg(before_647)]
        self.write(CamLookAt {
            x,
            z,
            height,
            rate,
            rate2,
        });
        #[cfg(since_647)]
        self.write(CamLookat {
            local_x: x,
            local_z: z,
            height,
            const_speed: rate,
            var_speed: rate2,
        });
    }

    pub fn cam_shake(&mut self, direction: u8, jitter: u8, amplitude: u8, frequency: u8) {
        #[cfg(before_647)]
        self.write(CamShake {
            direction,
            jitter,
            amplitude,
            frequency,
        });
        // 647 adds the wobble's starting phase; a shake starts at the start,
        // as the pre-647 client began every shake at cycle 0.
        #[cfg(since_647)]
        self.write(CamShake {
            direction,
            jitter,
            amplitude,
            frequency,
            cycle: 0,
        });
    }

    /// Flushes queued camera updates and refreshes the player's map view. Runs once per
    /// tick during the output phase.
    ///
    /// 1. Drains the camera queue, converting each update's absolute coordinate to a
    ///    build-area-local coordinate and writing the matching `CamMoveTo` / `CamLookAt`.
    /// 2. On a map-zone (mapsquare) crossing, fires the `[mapzone]` / `[mapzoneexit]`
    ///    triggers.
    /// 3. On a zone crossing, rebuilds the active zone list (delegated to
    ///    [`Player::update_map`]), toggles the multiway indicator before 647, and
    ///    fires the `[zone]` / `[zoneexit]` triggers.
    /// 4. From 647, when the areas painted on the player's tile change, fires the
    ///    `[areaexit]` / `[area]` triggers of the areas that trigger. The multiway
    ///    indicator is `[area,multiway]`'s from then.
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
            #[cfg(before_647)]
            {
                let was_multi = rsmod::is_zone_flagged(
                    old_zone.x(),
                    old_zone.z(),
                    old_zone.y(),
                    ZoneFlag::Multi as u8,
                );
                let now_multi =
                    rsmod::is_zone_flagged(zone.x(), zone.z(), zone.y(), ZoneFlag::Multi as u8);
                if was_multi != now_multi {
                    self.write(SetMultiway { hide: now_multi });
                }
            }
            self.trigger_zone_exit(old_zone.y(), old_zone.x(), old_zone.z());
            self.trigger_zone(coord.y(), zone.x(), zone.z());
        }

        // 4. The areas painted on the tile changed -> area exit/enter triggers.
        #[cfg(since_647)]
        self.trigger_areas(coord);
    }

    /// Fires `[areaexit,...]` for each triggering area the player's tile no
    /// longer has, then `[area,...]` for each it gained.
    #[cfg(since_647)]
    fn trigger_areas(&mut self, coord: CoordGrid) {
        let engine = engine();
        let tile = engine.area_tile(coord);
        let areas = engine.cache.area_map.get(tile.y(), tile.x(), tile.z());
        if areas == self.player.areas {
            return;
        }
        let left = mem::replace(&mut self.player.areas, areas);
        for area in left {
            if area != u16::MAX && !areas.contains(&area) {
                self.trigger_area(ServerTriggerType::AreaExit, area);
            }
        }
        for area in areas {
            if area != u16::MAX && !left.contains(&area) {
                self.trigger_area(ServerTriggerType::Area, area);
            }
        }
    }

    #[cfg(since_647)]
    fn trigger_area(&mut self, trigger: ServerTriggerType, area: u16) {
        let engine = engine();
        if !engine
            .cache
            .areas
            .get_by_id(area)
            .is_some_and(|area| area.trigger)
        {
            return;
        }
        if let Some(script) = engine.script_by_key(trigger, Some(area as i32), None) {
            let _ = self
                .player
                .state
                .queues
                .add(QueuePriority::Engine, script.id, 0, None);
        }
    }

    /// Looks up a coordinate-keyed trigger script by name and, if one is registered,
    /// enqueues it on the engine queue. A no-op when no script matches (e.g. the sentinel
    /// exit triggers fired for the initial `(0, 0, 0)` zone on spawn). The name is
    /// formatted on the stack: this runs on every zone crossing.
    fn enqueue_zone_trigger(&mut self, name: fmt::Arguments<'_>) {
        let mut buf = TriggerName::new();
        if buf.write_fmt(name).is_err() {
            return;
        }
        let script_id = engine().scripts.get_by_name(buf.as_str()).map(|s| s.id);
        if let Some(id) = script_id {
            let _ = self
                .player
                .state
                .queues
                .add(QueuePriority::Engine, id, 0, None);
        }
    }

    /// Fires the `[mapzone,...]` trigger for the mapsquare containing `(x, z)`.
    fn trigger_mapzone(&mut self, x: u16, z: u16) {
        self.enqueue_zone_trigger(format_args!("[mapzone,0_{}_{}]", x >> 6, z >> 6));
    }

    /// Fires the `[mapzoneexit,...]` trigger for the mapsquare containing `(x, z)`.
    fn trigger_mapzone_exit(&mut self, x: u16, z: u16) {
        self.enqueue_zone_trigger(format_args!("[mapzoneexit,0_{}_{}]", x >> 6, z >> 6));
    }

    /// Fires the `[zone,...]` trigger for the zone containing `(x, z)` on `level`.
    fn trigger_zone(&mut self, level: u8, x: u16, z: u16) {
        let mx = x >> 6;
        let mz = z >> 6;
        let lx = ((x & 0x3f) >> 3) << 3;
        let lz = ((z & 0x3f) >> 3) << 3;
        self.enqueue_zone_trigger(format_args!("[zone,{level}_{mx}_{mz}_{lx}_{lz}]"));
    }

    /// Fires the `[zoneexit,...]` trigger for the zone containing `(x, z)` on `level`.
    fn trigger_zone_exit(&mut self, level: u8, x: u16, z: u16) {
        let mx = x >> 6;
        let mz = z >> 6;
        let lx = ((x & 0x3f) >> 3) << 3;
        let lz = ((z & 0x3f) >> 3) << 3;
        self.enqueue_zone_trigger(format_args!("[zoneexit,{level}_{mx}_{mz}_{lx}_{lz}]"));
    }

    /// Sends a camera reset packet to the client, restoring the default camera.
    pub fn cam_reset(&mut self) {
        self.write(CamReset);
    }

    /// Sends the chat filter settings (public, private, trade) to the client.
    /// Rev 647 splits them: public and trade in CHAT_FILTER_SETTINGS, private
    /// in its own packet.
    pub fn chat_filter_settings(&mut self) {
        let public = self.player.public_chat();
        let private = self.player.private_chat();
        let trade = self.player.trade_chat();
        #[cfg(before_647)]
        self.write(ChatFilterSettings {
            public,
            private,
            trade,
        });
        #[cfg(since_647)]
        {
            self.write(ChatFilterSettings { public, trade });
            self.write(ChatFilterSettingsPrivatechat { private });
        }
    }

    /// Sends a chunk of land (ground texture) map data to the client.
    #[cfg(rev = "225")]
    pub fn data_land(&mut self, x: u8, z: u8, off: u16, len: u16, data: &[u8]) {
        self.write(DataLand {
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
        self.write(DataLandDone { x, z });
    }

    /// Sends a chunk of location map data to the client.
    #[cfg(rev = "225")]
    pub fn data_loc(&mut self, x: u8, z: u8, off: u16, len: u16, data: &[u8]) {
        self.write(DataLoc {
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
        self.write(DataLocDone { x, z });
    }

    /// Sends a hint arrow pointing at an NPC.
    ///
    /// # Arguments
    /// * `nid` - The world index of the NPC to highlight.
    pub fn hint_npc(&mut self, nid: u16) {
        #[cfg(before_647)]
        self.write(HintArrow {
            hint: 1,
            arg1: nid,
            arg2: 0,
            arg3: 0,
        });
        #[cfg(since_647)]
        self.write(HintArrow {
            slot: 0,
            icon: 0,
            kind: HintArrowKind::Npc { npc_index: nid },
            model_id: 0xFFFF,
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
        #[cfg(before_647)]
        self.write(HintArrow {
            hint: offset,
            arg1: x,
            arg2: z,
            arg3: height,
        });
        // Rev-647 coord hints carry the level explicitly (the 2004-era packet
        // implied the player's plane). This command keeps the bytes it has
        // always sent: slot 0, icon 0, the tile's centre, range 0;
        // `hint_coord_slot` sends the rest.
        #[cfg(since_647)]
        {
            let _ = offset;
            let level = self.player.pathing.coord.y();
            self.write(HintArrow {
                slot: 0,
                icon: 0,
                kind: HintArrowKind::Coord {
                    offset: 2,
                    x,
                    z,
                    level,
                    height,
                    range: 0,
                },
                model_id: 0xFFFF,
            });
        }
    }

    /// Rev-647: a hint arrow over a tile in one of the client's eight slots,
    /// so arrows from different systems (a quest's, the world map marker's)
    /// stand side by side instead of replacing each other.
    ///
    /// # Arguments
    /// * `slot` - The client's arrow slot, `0`--`7`.
    /// * `icon` - The arrow's colour: an index into the hint_headicons,
    ///   hint_mapmarkers and hint_mapedge sprite sheets.
    /// * `offset` - Where over the tile it hovers: `2` its centre, `3`--`6`
    ///   the middle of its west, east, south or north edge.
    /// * `level`, `x`, `z` - The tile.
    /// * `height` - How high over the tile, in the client's quarter units.
    /// * `range` - How far away the minimap still shows it, in 32-tile units.
    #[cfg(since_647)]
    #[allow(clippy::too_many_arguments)]
    pub fn hint_coord_slot(
        &mut self,
        slot: u8,
        icon: u8,
        offset: u8,
        level: u8,
        x: u16,
        z: u16,
        height: u8,
        range: u16,
        model: Option<u16>,
    ) {
        self.write(HintArrow {
            slot,
            icon,
            kind: HintArrowKind::Coord {
                offset,
                x,
                z,
                level,
                height,
                range,
            },
            model_id: model.unwrap_or(0xFFFF),
        });
    }

    /// Rev-647: a hint arrow over an npc in one of the client's eight slots,
    /// with a model the client draws at the local player's feet, turned toward
    /// the npc, or none.
    #[cfg(since_647)]
    pub fn hint_npc_slot(&mut self, nid: u16, slot: u8, icon: u8, model: Option<u16>) {
        self.write(HintArrow {
            slot,
            icon,
            kind: HintArrowKind::Npc { npc_index: nid },
            model_id: model.unwrap_or(0xFFFF),
        });
    }

    /// Rev-647: clears the hint arrow in one of the client's eight slots,
    /// leaving the others.
    #[cfg(since_647)]
    pub fn hint_stop_slot(&mut self, slot: u8) {
        self.write(HintArrow {
            slot,
            icon: 0,
            kind: HintArrowKind::Stop,
            model_id: 0xFFFF,
        });
    }

    /// Sends a hint arrow pointing at another player.
    ///
    /// # Arguments
    /// * `slot` - The player index (pid) to highlight.
    pub fn hint_player(&mut self, slot: u16) {
        #[cfg(before_647)]
        self.write(HintArrow {
            hint: 10,
            arg1: slot,
            arg2: 0,
            arg3: 0,
        });
        #[cfg(since_647)]
        self.write(HintArrow {
            slot: 0,
            icon: 0,
            kind: HintArrowKind::Player { player_index: slot },
            model_id: 0xFFFF,
        });
    }

    /// Clears any active hint arrow on the client. The wire type `-1` (`0xFF`) tells the
    /// client to remove the arrow.
    pub fn stop_hint(&mut self) {
        #[cfg(before_647)]
        self.write(HintArrow {
            hint: 0xFF,
            arg1: 0,
            arg2: 0,
            arg3: 0,
        });
        #[cfg(since_647)]
        self.write(HintArrow {
            slot: 0,
            icon: 0,
            kind: HintArrowKind::Stop,
            model_id: 0xFFFF,
        });
    }

    /// Sends an interface close packet to the client.
    pub fn if_close(&mut self) {
        #[cfg(before_647)]
        self.write(IfClose {});
        // Rev-647 closes per-slot: IF_CLOSESUB names the parent component hash
        // the sub-interface was mounted on, so a close goes to each slot a
        // modal actually opened in rather than to a single screen-wide modal.
        #[cfg(since_647)]
        for parent in [
            take(&mut self.player.modal_chat_parent),
            take(&mut self.player.modal_main_parent),
            take(&mut self.player.modal_side_parent),
        ] {
            if parent != 0 {
                self.if_close_sub(parent);
            }
        }
    }

    /// Sends a packet to open both a main interface and a side interface.
    pub fn if_open_main_side(&mut self, com: u16, side: u16) {
        #[cfg(before_647)]
        self.write(IfOpenMainSide { com, side });
        #[cfg(since_647)]
        {
            self.if_open_main(com);
            self.if_open_side(side);
        }
    }

    /// Sends a packet to open a main (fullscreen) interface.
    ///
    /// Rev-647 has no screen-wide main modal: the interface is mounted with
    /// IF_OPENSUB on the toplevel's main-screen slot (`mainmodal`,
    /// 548:18 fixed and 746:9 resizable, `void-634`'s `main_screen` with
    /// 746's +1 drift), as a type-0 modal. An interface the content's
    /// `wide_screen_enum` lists mounts on the full-size slot instead (548:8
    /// `mainoverlay_clipped`, 746:8 `mainmodal_fullscreen`): 2011's skill
    /// guide fills it with its own backdrop, which in the main slot showed as
    /// a tinted box.
    pub fn if_open_main(&mut self, com: u16) {
        #[cfg(before_647)]
        self.write(IfOpenMain { com });
        #[cfg(since_647)]
        {
            let layout = &engine().layout;
            let slots = if layout.wide_screen_ifaces.contains(&com) {
                &layout.wide_screen
            } else {
                &layout.main_screen
            };
            let parent = self.toplevel_slot(slots);
            if parent == 0 {
                warn!(
                    "if_open_main({com}): toplevel {} has no main screen",
                    self.player.toplevel
                );
                return;
            }
            self.player.modal_main_parent = parent;
            self.if_open_sub(com, parent, 0);
        }
    }

    /// Sends a packet to open a chat-area interface.
    ///
    /// Rev-647 has no screen-wide chat modal: a dialogue is a *sub-interface*
    /// mounted on a child of the chatbox frame, so it goes out as `IF_OPENSUB`
    /// against that child's component hash. Which child depends on the
    /// interface — most use the default `dialogue_box` slot, while the small
    /// ones (`objbox`, `doubleobjbox`) override it — and the mapping comes from
    /// the content `dialogue_slot_enum` via `layout.dialogue_slot`, so the engine
    /// carries no interface ids of its own.
    ///
    /// The slot actually used is recorded on the player: `IF_CLOSESUB` closes by
    /// parent hash, so closing the default slot while an objbox is open in the
    /// small one leaves the dialogue on screen.
    pub fn if_open_chat(&mut self, com: u16) {
        #[cfg(before_647)]
        self.write(IfOpenChat { com });
        #[cfg(since_647)]
        {
            let parent_hash = engine().layout.dialogue_slot(com);
            if parent_hash == 0 {
                warn!("if_open_chat({com}): no dialogue slot; chatbox interface not found");
                return;
            }
            self.player.modal_chat_parent = parent_hash;
            self.if_open_sub(com, parent_hash, 0);
        }
    }

    /// Sends a packet to open a side-panel interface.
    ///
    /// Rev-647 mounts it with IF_OPENSUB on the toplevel's slot over the side
    /// panels (`sidemodal`, 548:198 fixed and 746:84 resizable:
    /// `void-634`'s `overlay_tab` 197/83 plus the revision drift, both the
    /// 190x261 of a tab), where the bank and shop inventories go.
    ///
    /// As type 1, not a modal: `void-634`'s `overlay_tab` is permanent. The
    /// client's own close (a modal's close button) closes only type-0 subs,
    /// and without re-running the toplevel's scripts (`Class194.method1298`);
    /// the panel is left to the server's IF_CLOSESUB, which re-runs them
    /// (`LightDetailClientOption.method2499`) and so brings the tabs back. A
    /// type-0 panel went with the client's close and left the tabs hidden.
    pub fn if_open_side(&mut self, com: u16) {
        #[cfg(before_647)]
        self.write(IfOpenSide { com });
        #[cfg(since_647)]
        {
            let parent = self.toplevel_slot(&engine().layout.overlay_tab);
            if parent == 0 {
                warn!(
                    "if_open_side({com}): toplevel {} has no side slot",
                    self.player.toplevel
                );
                return;
            }
            self.player.modal_side_parent = parent;
            self.if_open_sub(com, parent, 1);
        }
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
    /// The message count is the player's unread Message Centre messages as of
    /// this login.
    pub fn last_login_info(&mut self) {
        let previous = if self.player.last_login_date == 0 {
            self.player.last_date
        } else {
            self.player.last_login_date
        };
        let days_since_login = ((self.player.last_date - previous) / (60 * 60 * 24)) as u16;
        #[cfg(before_647)]
        self.write(LastLoginInfo {
            ip: 2130706433,
            login: days_since_login,
            recovery: 201,
            messages: self.unread_messages,
            #[cfg(since_244)]
            warn_members_in_non_members: !engine().members && self.member(),
        });
        // Rev-647 only reports the day count; the welcome-screen extras
        // (ip/recovery/messages) are gone from the packet.
        #[cfg(since_647)]
        self.write(LastLoginInfo {
            login: days_since_login as i32,
        });
    }

    /// Sets the animation playing on an interface component.
    pub fn if_setanim(&mut self, com: i32, seq: u16) {
        #[cfg(before_647)]
        self.write(IfSetAnim {
            com: com as u16,
            seq,
        });
        #[cfg(since_647)]
        self.write(IfSetAnim { com, seq });
    }

    /// Sets the color of an interface component.
    pub fn if_setcolour(&mut self, com: i32, colour: u16) {
        #[cfg(before_647)]
        self.write(IfSetColour {
            com: com as u16,
            colour,
        });
        #[cfg(since_647)]
        self.write(IfSetColour { com, colour });
    }

    /// Sets the visibility of an interface component.
    pub fn if_sethide(&mut self, com: i32, hide: bool) {
        #[cfg(before_647)]
        self.write(IfSetHide {
            com: com as u16,
            hide,
        });
        #[cfg(since_647)]
        self.write(IfSetHide { com, hidden: hide });
    }

    /// Sets the model displayed on an interface component.
    pub fn if_setmodel(&mut self, com: i32, model: u16) {
        #[cfg(before_647)]
        self.write(IfSetModel {
            com: com as u16,
            model,
        });
        #[cfg(since_647)]
        self.write(IfSetModel { com, model });
    }

    /// Sets the graphic (a sprite) displayed on an interface component.
    #[cfg(since_647)]
    pub fn if_setgraphic(&mut self, com: i32, graphic: u16) {
        self.write(IfSetGraphic { com, graphic });
    }

    /// Sets an NPC head model on an interface component.
    pub fn if_setnpchead(&mut self, com: i32, npc: u16) {
        #[cfg(before_647)]
        self.write(IfSetNpcHead {
            com: com as u16,
            npc,
        });
        #[cfg(since_647)]
        self.write(IfSetNpcHead { com, npc });
    }

    /// Sets an item model on an interface component with a given zoom scale.
    pub fn if_setobject(&mut self, com: i32, obj: u16, scale: i32) {
        #[cfg(before_647)]
        self.write(IfSetObject {
            com: com as u16,
            obj,
            scale: scale as u16,
        });
        #[cfg(since_647)]
        self.write(IfSetObject { com, obj, scale });
    }

    /// Sets the local player's head model on an interface component.
    pub fn if_setplayerhead(&mut self, com: i32) {
        #[cfg(before_647)]
        self.write(IfSetPlayerHead { com: com as u16 });
        #[cfg(since_647)]
        self.write(IfSetPlayerHead { com });
    }

    /// Sets the position of an interface component.
    pub fn if_setposition(&mut self, com: i32, x: u16, y: u16) {
        #[cfg(before_647)]
        self.write(IfSetPosition {
            com: com as u16,
            x,
            y,
        });
        #[cfg(since_647)]
        self.write(IfSetPosition { com, x, y });
    }

    /// Sets the scrollbar position on an interface component.
    #[cfg(since_245_2)]
    pub fn if_setscrollpos(&mut self, com: i32, y: u16) {
        #[cfg(before_647)]
        self.write(IfSetScrollPos { com: com as u16, y });
        #[cfg(since_647)]
        self.write(IfSetScrollPos { com, y });
    }

    /// Opens an overlay interface over the game view (a minigame's timer, the
    /// wilderness level), or with `u16::MAX` (`null`) closes the one open.
    ///
    /// Rev-647 mounts it with IF_OPENSUB on the toplevel's full-size overlay
    /// slot (`mainoverlay`, 548:7 fixed and 746:6 resizable), as type 1: an
    /// overlay is no modal, and stays while the player walks and talks.
    #[cfg(since_244)]
    pub fn if_openoverlay(&mut self, com: u16) {
        #[cfg(before_647)]
        self.write(IfOpenOverlay { com });
        #[cfg(since_647)]
        {
            let parent = self.toplevel_slot(&engine().layout.overlay);
            if parent == 0 {
                warn!(
                    "if_openoverlay({com}): toplevel {} has no overlay slot",
                    self.player.toplevel
                );
                return;
            }
            if com == u16::MAX {
                self.if_close_sub(parent);
            } else {
                self.if_open_sub(com, parent, 1);
            }
        }
    }

    /// Sets (or clears, when `value` is empty) a right-click option shown to
    /// other players on this player's context menu.
    #[cfg(since_254)]
    pub fn set_player_op(&mut self, op: u8, value: &str, primary: u8) {
        self.write(SetPlayerOp { op, value, primary });
    }

    /// Tells the client the loading state of its friend list (`1` connecting,
    /// `2` loaded/online).
    #[cfg(since_254)]
    pub fn friendlist_loaded(&mut self, status: u8) {
        #[cfg(before_647)]
        self.write(FriendListLoaded { status });
        // Rev-647 FRIENDLIST_LOADED carries no payload and only signals the
        // loaded state; the "connecting" (1) state has no 647 equivalent.
        #[cfg(since_647)]
        if status == 2 {
            self.write(FriendlistLoaded);
        }
    }

    /// Sets the client minimap state (`0` normal, `1` disable click, `2`
    /// blacked out).
    #[cfg(since_274)]
    pub fn minimap_toggle(&mut self, minimap_type: u8) {
        self.write(MinimapToggle { minimap_type });
    }

    /// Recolors an interface component model, remapping `src` color to `dst`.
    #[cfg(before_245_2)]
    pub fn if_setrecol(&mut self, com: u16, src: u16, dst: u16) {
        self.write(IfSetRecol { com, src, dst });
    }

    /// Assigns an interface component to a tab slot on the client.
    pub fn if_settab(&mut self, com: u16, tab: u8) {
        #[cfg(before_647)]
        self.write(IfSetTab { com, tab });
        // TODO(647): rev-647 has no tab packets — tabs are laid out via
        // IF_OPENSUB by content (interface milestone).
        #[cfg(since_647)]
        let _ = (com, tab);
    }

    /// Switches the client's currently selected (active) tab. 2011's client
    /// has no tab packet: it keeps its selected tab in the `if_switchside_varc`
    /// varc, in the same order as the tabs.
    pub fn if_settabactive(&mut self, tab: u8) {
        #[cfg(before_647)]
        self.write(IfSetTabActive { tab });
        #[cfg(since_647)]
        self.varc_transmit(engine().layout.switch_side, tab as i32);
    }

    /// Sets the text content of an interface component.
    pub fn if_settext(&mut self, com: i32, text: &str) {
        #[cfg(before_647)]
        self.write(IfSetText {
            com: com as u16,
            text,
        });
        #[cfg(since_647)]
        self.write(IfSetText { com, text });
    }

    /// Sends the logout packet and marks the player as having received it.
    ///
    /// # Side Effects
    /// * Sets `player.logout_sent = true`.
    pub fn logout(&mut self) {
        self.watch_end();
        self.player.logout_sent = true;
        self.write(Logout);
    }

    /// Notes how the session is ending, for how fast the player comes back.
    fn watch_end(&mut self) {
        let engine = engine();
        let ms = self.watch.ms();
        let silent = engine.clock.saturating_sub(self.player.last_response) >= TIMEOUT_NO_RESPONSE;
        let (end, at) = if engine.is_shutdown() {
            (End::Reboot, ms)
        } else if let Some(lost) = self.player.disconnected_at {
            let since = engine.clock.saturating_sub(lost) * 600;
            (End::Disconnect, ms.saturating_sub(since))
        } else if silent {
            (End::Disconnect, ms)
        } else if self.player.logout_idle_requested {
            (End::Idle, ms)
        } else if self.player.logout_requested {
            (End::Logout, ms)
        } else {
            (End::Kick, ms)
        };
        self.watch.anticheat.ended(end, at);
    }

    /// When a mute lasting `d` from now ends. Capped at 100 years, so a mute
    /// set far out by hand can't overflow the clock.
    pub fn mute_deadline(d: Duration) -> Instant {
        const CENTURY: Duration = Duration::from_secs(100 * 365 * 24 * 3600);
        Instant::now() + d.min(CENTURY)
    }

    /// Whether the player is muted, and so may not public or private chat.
    pub fn is_muted(&self) -> bool {
        self.muted_until.is_some_and(|until| Instant::now() < until)
    }

    /// Bans the player for [`AUTO_BAN_HOURS`] and logs them out, for sending
    /// what only a modified client can. The ban reaches the database before
    /// the logout save, so logging straight back in is refused.
    pub fn auto_ban(&mut self, trigger: AutoBan) {
        counters::add(&COUNTERS.auto_bans[trigger as usize], 1);
        self.watch.anticheat.auto_banned = Some(counters::AUTO_BAN_TRIGGERS[trigger as usize]);
        if let Some(tx) = &engine().db_tx {
            let _ = tx.send(DbRequest::Ban {
                user37: self.player.uid.username37(),
                hours: AUTO_BAN_HOURS,
            });
        }
        self.logout();
    }

    /// Closes the connection, as if the socket had dropped. The player stays
    /// for the reconnect window under the usual logout rules, so this is no
    /// way out of combat.
    fn disconnect(&mut self) {
        self.handle.close();
        self.player.disconnected_at.get_or_insert(engine().clock);
    }

    /// Records a menu action the player chose, for the anti-cheat checks and
    /// the replay's timeline.
    pub fn watch_op(&mut self, op: Op) {
        if !self.watch.checking() {
            return;
        }
        let clock = engine().clock;
        let base = self.map_base();
        self.watch.op(op, clock, base);
        let on = match op {
            Op::Npc { nid, .. } => Some(Target::Npc(nid)),
            Op::Player { pid, .. } => Some(Target::Player(pid)),
            Op::Loc { id, x, z, .. } => Some(Target::Loc { id, x, z }),
            Op::Obj { id, x, z, .. } => Some(Target::Obj { id, x, z }),
            _ => None,
        };
        if let Some(on) = on {
            self.watch
                .anticheat
                .target(on, &self.player.build_area, clock);
        }
        let menu = self.needs_menu(op) && !self.one_mouse_button();
        if menu {
            let ms = self.watch.ms();
            self.watch.anticheat.menu_op(ms);
        }
        self.watch_behaviour(op, on, menu);
        if !self.watch.recording() {
            return;
        }
        let mut note = self.watch.note_buf();
        self.op_note(op, &mut note);
        self.watch.note_string(clock, note);
    }

    /// Feeds an action to the checks on how the player behaves over time:
    /// how fast they react, how much they repeat themselves, where in its
    /// slot they click an obj.
    fn watch_behaviour(&mut self, op: Op, on: Option<Target>, menu: bool) {
        let ms = self.watch.packet_ms();
        let appeared = match op {
            Op::Obj { id, x, z, .. } => {
                let at = CoordGrid::new(x, self.player.pathing.coord.y(), z);
                engine()
                    .obj_appeared
                    .get(&(at, id))
                    .map(|&when| self.watch.ms_at(when))
            }
            _ => None,
        };
        let slot = match op {
            Op::Held { slot, com, .. } | Op::InvButton { slot, com, .. } => {
                self.slot_on_screen(com, slot)
            }
            _ => None,
        };
        let anticheat = &mut self.watch.anticheat;
        anticheat.acted(ms, appeared);
        anticheat.acting_on = on.filter(|t| matches!(t, Target::Loc { .. } | Target::Npc(_)));
        match op {
            Op::Npc { op, nid } => anticheat.repeat(("npc", op, nid)),
            Op::Loc { op, id, x, z } => anticheat.repeat(("loc", op, id, x, z)),
            Op::Obj { op, id, x, z } => anticheat.repeat(("obj", op, id, x, z)),
            Op::Player { op, .. } => anticheat.repeat(("player", op)),
            Op::Held { op, obj, slot, com } => anticheat.repeat(("held", op, obj, slot, com)),
            Op::InvButton { op, obj, slot, com } => {
                anticheat.repeat(("button", op, obj, slot, com))
            }
            Op::Other => {}
        }
        if let Some(slot) = slot {
            anticheat.slot_action(ms, slot, menu);
        }
    }

    /// Where on the client's screen an inventory's `slot` is drawn (its top
    /// left), from where the interface it's in is open. `None` in a layer
    /// that scrolls (the bank), whose slots move with it.
    fn slot_on_screen(&self, com: u16, slot: u16) -> Option<(i16, i16)> {
        // Where the client draws its main, chat and side areas.
        #[cfg(rev = "225")]
        const AREAS: [(i16, i16); 3] = [(8, 11), (22, 375), (562, 231)];
        #[cfg(since_244)]
        const AREAS: [(i16, i16); 3] = [(4, 4), (17, 357), (553, 205)];
        let interfaces = engine().interfaces();
        let inv = interfaces.get_by_id(com).filter(|i| !i.scrolls)?;
        let (x, y) = inv.slot_at(slot)?;
        let root = u16::try_from(inv.root_layer).ok();
        let (left, top) = if root.is_some() && root == self.player.modal_main {
            AREAS[0]
        } else if root.is_some() && root == self.player.modal_chat {
            AREAS[1]
        } else {
            AREAS[2]
        };
        Some((left + x, top + y))
    }

    /// At the end of each tick: whether the loc or NPC the player last acted
    /// on went (a rock ran out, a tree fell, an NPC died), for how fast they
    /// react to it.
    fn watch_acting_on(&mut self) {
        let Some(on) = self.watch.anticheat.acting_on else {
            return;
        };
        let engine = engine();
        let here = match on {
            Target::Loc { id, x, z } => engine
                .zones
                .loc_at(x, self.player.pathing.coord.y(), z, id)
                .is_some(),
            Target::Npc(nid) => engine.get_npc(nid).is_some_and(|n| n.npc.active),
            _ => true,
        };
        if !here {
            let ms = self.watch.ms();
            self.watch.anticheat.gone(ms);
        }
    }

    /// Whether `op` is an option a left click doesn't pick, so the client had
    /// to open its menu for it (see [`menu`]). Objs a free world shows as
    /// members' and banknotes are passed over.
    fn needs_menu(&self, op: Op) -> bool {
        let engine = engine();
        let obj = |id: u16| {
            engine
                .objs()
                .get_by_id(id)
                .filter(|t| (engine.members || !t.members) && t.certtemplate.is_none())
        };
        let inv_top = |com: u16, id: u16| {
            let interface = engine.interfaces().get_by_id(com)?;
            let obj = obj(id)?;
            Some(menu::inv_top(
                interface.iop.as_deref(),
                interface.operable,
                interface.usable,
                obj.iop.as_deref(),
            ))
        };
        match op {
            Op::Npc { op, nid } => engine
                .get_npc(nid)
                .and_then(|n| self.shown_npc(n.npc.uid.id()))
                .and_then(|id| engine.npcs().get_by_id(id))
                .is_some_and(|t| {
                    let level = t.vislevel.map_or(-1, i32::from);
                    !menu::npc(t.op.as_deref(), op, level, self.player.combat_level.into())
                }),
            // The web client doesn't turn locs into others (from 289), so
            // what it offers on one that turns isn't judged.
            Op::Loc { op, id, .. } => self
                .shown_loc(id)
                .filter(|&shown| shown == id || !self.watch.anticheat.device.web)
                .and_then(|id| engine.locs().get_by_id(id))
                .is_some_and(|t| !menu::loc(t.op.as_deref(), op)),
            Op::Obj { op, id, .. } => {
                obj(id).is_some_and(|t| !menu::ground_obj(t.op.as_deref(), op))
            }
            Op::Held { op, obj, com, .. } => {
                inv_top(com, obj).is_some_and(|top| top != InvOp::Obj(usize::from(op) - 1))
            }
            Op::InvButton { op, obj, com, .. } => {
                inv_top(com, obj).is_some_and(|top| top != InvOp::Interface(usize::from(op) - 1))
            }
            Op::Player { .. } | Op::Other => false,
        }
    }

    /// Whether the player set the game to one mouse button: a left click
    /// then opens the menu.
    fn one_mouse_button(&self) -> bool {
        engine()
            .varps()
            .get_by_debugname("option_mouse")
            .is_some_and(|varp| self.player.vars.get(varp.id).as_int() == 1)
    }

    /// Notes something the player did for the replay's timeline.
    pub fn watch_note(&mut self, note: &str) {
        self.watch.note(engine().clock, note);
    }

    /// Records a "use" or spell action, e.g. "Use Bones on Altar".
    pub fn watch_use(&mut self, with: Using, on: Target) {
        if !self.watch.checking() {
            return;
        }
        self.watch_op(Op::Other);
        self.watch.anticheat.repeat(("use", with, on));
        self.watch
            .anticheat
            .target(on, &self.player.build_area, engine().clock);
        if !self.watch.recording() {
            return;
        }
        let engine = engine();
        let obj = |id: u16| {
            engine
                .objs()
                .get_by_id(id)
                .and_then(|t| t.name.as_deref())
                .unwrap_or("?")
        };
        let (verb, with) = match with {
            Using::Item(id) => ("Use", Cow::Borrowed(obj(id))),
            Using::Spell(com) => ("Cast", self.component_name(com)),
        };
        let on = match on {
            Target::Npc(nid) => Cow::Borrowed(
                engine
                    .get_npc(nid)
                    .and_then(|n| engine.npcs().get_by_id(n.npc.uid.id()))
                    .and_then(|t| t.name.as_deref())
                    .unwrap_or("?"),
            ),
            Target::Loc { id, .. } => Cow::Borrowed(
                engine
                    .locs()
                    .get_by_id(id)
                    .and_then(|t| t.name.as_deref())
                    .unwrap_or("?"),
            ),
            Target::Obj { id, .. } | Target::Item(id) => Cow::Borrowed(obj(id)),
            Target::Player(pid) => engine
                .get_player(pid)
                .map_or(Cow::Borrowed("?"), |p| Cow::Owned(p.uid().username())),
        };
        self.watch
            .note_fmt(engine.clock, format_args!("{verb} {with} on {on}"));
    }

    /// Records a click on interface component `com`.
    pub fn watch_button(&mut self, com: u16) {
        self.watch_op(Op::Other);
        if !self.watch.recording() {
            return;
        }
        let name = self.component_name(com.into());
        self.watch
            .note_fmt(engine().clock, format_args!("Click {name}"));
    }

    /// The loc `id` shows as to this player: from 289 a loc can turn into
    /// another by a variable, and the client shows that one's name and options
    /// (while clicks on it carry `id`). `None` when it turns into nothing,
    /// which the client offers no options on.
    #[cfg(since_289)]
    pub fn shown_loc(&self, id: u16) -> Option<u16> {
        match engine().locs().get_by_id(id) {
            Some(ty) => ty.multi_loc(self.multivar(ty.multivar)),
            None => Some(id),
        }
    }

    /// What the variable a transform is picked by holds: -1 without one, which
    /// picks the transform's default.
    #[cfg(since_289)]
    fn multivar(&self, var: Option<MultiVar>) -> i32 {
        let value = match var {
            Some(MultiVar::Varbit(var)) => engine()
                .varbits()
                .get_by_id(var)
                .map(|var| self.varbit(var)),
            Some(MultiVar::Varp(var)) => Some(self.player.vars.get(var).as_int()),
            None => None,
        };
        value.unwrap_or(-1)
    }

    #[cfg(before_289)]
    pub fn shown_loc(&self, id: u16) -> Option<u16> {
        Some(id)
    }

    /// The npc type `id` shows as to this player: a 647 npc can be a
    /// multi-npc that the client draws as another by a variable, with that
    /// one's name and options (2011's Osman, npc 5282, is Osman by varbit
    /// 3280). `None` when it shows as nothing.
    #[cfg(since_647)]
    pub fn shown_npc(&self, id: u16) -> Option<u16> {
        match engine().npcs().get_by_id(id) {
            Some(ty) => ty.multi_npc(self.multivar(ty.multivar)),
            None => Some(id),
        }
    }

    #[cfg(before_647)]
    pub fn shown_npc(&self, id: u16) -> Option<u16> {
        Some(id)
    }

    fn map_base(&self) -> MapBase {
        let origin = self.player.build_area.origin;
        MapBase {
            x: CoordGrid::zone_origin(origin.x()),
            z: CoordGrid::zone_origin(origin.z()),
        }
    }

    /// A minimap click, for the anti-cheat to check what the client sent
    /// with it against the path and where the world has the player.
    #[cfg(before_647)]
    pub fn watch_minimap(&mut self, path: &[u32], trailer: &MinimapTrailer) {
        let base = self.map_base();
        let at = self.player.pathing.coord;
        self.watch
            .anticheat
            .minimap(path, trailer, base, (at.x(), at.z()));
    }

    pub fn watch_walk(&mut self, path: &[u32], minimap: bool) {
        if let Some(&to) = path.last() {
            let (x, z) = unpack_coord(to);
            self.watch.anticheat.walked(x, z);
        }
        self.watch_op(Op::Other);
        if !self.watch.recording() {
            return;
        }
        if let Some(&to) = path.last() {
            let (x, z) = unpack_coord(to);
            let on = if minimap { " (minimap)" } else { "" };
            self.watch
                .note_fmt(engine().clock, format_args!("Walk to {x}, {z}{on}"));
        }
    }

    /// An interface component's name, e.g. `magic:wind_strike`.
    pub(crate) fn component_name(&self, com: ComId) -> Cow<'static, str> {
        #[cfg(before_647)]
        let name = engine()
            .interfaces()
            .get_by_id(com as u16)
            .and_then(|i| i.com_name.as_deref())
            .map_or_else(|| Cow::Owned(format!("component {com}")), Cow::Borrowed);
        #[cfg(since_647)]
        let name = engine().layout.component_name(com);
        name
    }

    /// Writes how the player saw `op` in their menu, e.g. "Attack Goblin",
    /// to `out`. Nothing for [`Op::Other`].
    fn op_note(&self, op: Op, out: &mut String) {
        let engine = engine();
        fn named(
            out: &mut String,
            ops: Option<&[Option<Box<str>>]>,
            op: u8,
            name: Option<&str>,
            kind: &str,
        ) {
            match ops.and_then(|ops| ops.get(usize::from(op).checked_sub(1)?)?.as_deref()) {
                Some(verb) => out.push_str(verb),
                None => _ = write!(out, "{kind} op {op}"),
            }
            out.push(' ');
            out.push_str(name.unwrap_or("?"));
        }
        match op {
            Op::Npc { op, nid } => {
                let ty = engine
                    .get_npc(nid)
                    .and_then(|n| engine.npcs().get_by_id(n.npc.uid.id()));
                named(
                    out,
                    ty.and_then(|t| t.op.as_deref()),
                    op,
                    ty.and_then(|t| t.name.as_deref()),
                    "Npc",
                );
            }
            Op::Loc { op, id, x, z } => {
                let ty = self
                    .shown_loc(id)
                    .and_then(|id| engine.locs().get_by_id(id));
                named(
                    out,
                    ty.and_then(|t| t.op.as_deref()),
                    op,
                    ty.and_then(|t| t.name.as_deref()),
                    "Loc",
                );
                _ = write!(out, " at {x}, {z}");
            }
            Op::Obj { op, id, x, z } => {
                let ty = engine.objs().get_by_id(id);
                named(
                    out,
                    ty.and_then(|t| t.op.as_deref()),
                    op,
                    ty.and_then(|t| t.name.as_deref()),
                    "Obj",
                );
                _ = write!(out, " at {x}, {z}");
            }
            Op::Player { op, pid } => {
                let name = engine.get_player(pid).map(|p| p.uid().username());
                _ = write!(out, "Player op {op} on {}", name.as_deref().unwrap_or("?"));
            }
            Op::Held { op, obj, .. } => {
                let ty = engine.objs().get_by_id(obj);
                named(
                    out,
                    ty.and_then(|t| t.iop.as_deref()),
                    op,
                    ty.and_then(|t| t.name.as_deref()),
                    "Item",
                );
            }
            // A shop's or bank's own options, named by the interface.
            Op::InvButton { op, obj, com, .. } => named(
                out,
                engine
                    .interfaces()
                    .get_by_id(com)
                    .and_then(|i| i.iop.as_deref()),
                op,
                engine.objs().get_by_id(obj).and_then(|t| t.name.as_deref()),
                "Button",
            ),
            Op::Other => {}
        }
    }

    /// This tick's [`LogBatch`], sent with [`DbRequest::Logs`], and where the
    /// player is (x, z, y). `None` when nothing is logged: without a database,
    /// or for a bot.
    fn log_batch(&self) -> Option<(&'static mut LogBatch, i16, i16, i16)> {
        let engine = engine_mut();
        if engine.db_tx.is_none() || self.player.bot {
            return None;
        }
        let coord = self.player.pathing.coord;
        Some((
            &mut engine.logs,
            coord.x() as i16,
            coord.z() as i16,
            i16::from(coord.y()),
        ))
    }

    /// Logs chat the player sent (unfiltered), to `to37` if private.
    pub fn log_chat(&self, to37: Option<u64>, message: String) {
        if let Some((logs, x, z, y)) = self.log_batch() {
            logs.chat.push(ChatLog {
                name_hash: self.player.uid.username37(),
                to_name_hash: to37,
                x,
                z,
                y,
                message,
            });
        }
    }

    /// Logs a script's `session_log` event.
    pub fn log_session(&self, event_type: i32, event: &str) {
        if let Some((logs, x, z, y)) = self.log_batch() {
            logs.sessions.push(SessionLog {
                name_hash: self.player.uid.username37(),
                x,
                z,
                y,
                event_type: event_type as i16,
                event: event.to_owned(),
            });
        }
    }

    /// Logs a script's `wealth_event`.
    pub fn log_wealth(&self, event_type: i32, item: &str, count: i32, value: i32) {
        if let Some((logs, x, z, y)) = self.log_batch() {
            logs.wealth.push(WealthEvent {
                name_hash: self.player.uid.username37(),
                x,
                z,
                y,
                event_type: event_type as i16,
                item: item.to_owned(),
                count,
                value,
                counterpart: None,
            });
        }
    }

    /// Logs items passing to another player (`to37`) or through the floor
    /// at `coord`, as one of the engine's own wealth events
    /// ([`rs_db::transfer`]), valued at the cache's price.
    pub fn log_transfer(
        &self,
        event_type: i16,
        coord: CoordGrid,
        obj: u16,
        count: i32,
        to37: Option<u64>,
    ) {
        let Some((logs, ..)) = self.log_batch() else {
            return;
        };
        let Some(obj_type) = engine().objs().get_by_id(obj) else {
            return;
        };
        logs.wealth.push(WealthEvent {
            name_hash: self.player.uid.username37(),
            x: coord.x() as i16,
            z: coord.z() as i16,
            y: i16::from(coord.y()),
            event_type,
            item: obj_type.debugname.to_string(),
            count,
            value: (i64::from(obj_type.cost) * i64::from(count)).clamp(0, i64::from(i32::MAX))
                as i32,
            counterpart: to37,
        });
    }

    /// Sends a game message to the player's chatbox.
    pub fn message_game(&mut self, text: &str) {
        #[cfg(before_647)]
        self.write(MessageGame { text });
        #[cfg(since_647)]
        self.message_game_typed(0, text);
    }

    /// Rev-647: a game message on a given chat channel, MESSAGE_GAME's type --
    /// 0 for the game channel, 11 for the clan chat's (`void-634` `ChatType`).
    #[cfg(since_647)]
    pub fn message_game_typed(&mut self, msg_type: i32, text: &str) {
        self.write(MessageGame {
            kind: msg_type,
            text,
        });
    }

    /// Plays a MIDI jingle sound effect on the client.
    #[cfg(rev = "225")]
    pub fn midi_jingle(&mut self, delay: u16, bytes: &[u8]) {
        self.write(MidiJingle { delay, bytes });
    }

    /// Plays a MIDI jingle sound effect on the client.
    #[cfg(since_244)]
    pub fn midi_jingle(&mut self, id: u16, delay: u16) {
        #[cfg(before_647)]
        self.write(MidiJingle { id, delay });
        // Rev-647 adds a volume byte (255 = full, mirroring the 647
        // reference server's default) and widens the delay/duration word.
        #[cfg(since_647)]
        self.write(MidiJingle {
            song_id: id,
            duration: delay as u32,
            volume: 255,
        });
    }

    /// Starts playing a MIDI song on the client
    #[cfg(rev = "225")]
    pub fn midi_song(&mut self, name: &str, crc: i32, len: i32) {
        self.write(MidiSong { name, crc, len });
    }

    /// Starts playing a MIDI song on the client
    #[cfg(since_244)]
    pub fn midi_song(&mut self, id: u16) {
        #[cfg(before_647)]
        self.write(MidiSong { id });
        // Rev-647 MIDI_SONG carries a volume (0 loads the track but plays
        // nothing) and a delay; full volume, at once.
        #[cfg(since_647)]
        self.write(MidiSong {
            id,
            volume: 255,
            delay: 0,
        });
    }

    /// Rev-647: opens a toplevel (root) interface and remembers it so later
    /// component-hash lookups (main-screen mounts, tab slots) resolve
    /// against the right layout row.
    ///
    /// # Arguments
    /// * `interface` - The toplevel interface.
    /// * `mode` - IF_OPENTOP's mode byte: `0`, or `2` for the client to
    ///   rebuild its scene behind a "Loading - please wait." screen.
    #[cfg(since_647)]
    pub fn if_open_top(&mut self, interface: u16, mode: u8) {
        self.player.toplevel = interface;
        self.write(IfOpentop { interface, mode });
    }

    /// Rev-647: mounts a sub-interface at a parent component hash, and
    /// records it as the client will (`Player::sub_opened`). Modal tracking
    /// lives in the open paths that choose the slot, so permanent subs (tabs,
    /// orbs, chatbox) are never mistaken for modals.
    #[cfg(since_647)]
    pub fn if_open_sub(&mut self, interface: u16, com: i32, walkable: u8) {
        self.player.sub_opened(com, interface, walkable);
        self.write(IfOpensub {
            interface,
            com,
            walkable,
        });
    }

    /// Rev-647: closes whatever sub-interface is mounted at the parent
    /// component hash.
    #[cfg(since_647)]
    pub fn if_close_sub(&mut self, com: i32) {
        self.player.sub_closed(com);
        self.write(IfClosesub { com });
    }

    /// Rev-647: moves the sub-interface mounted at `from` onto `to`, as a
    /// window-mode switch carries the gameframe from one toplevel to the other.
    #[cfg(since_647)]
    pub fn if_move_sub(&mut self, com: i32, com2: i32) {
        self.player.sub_moved(com, com2);
        self.write(IfMovesub { com, com2 });
    }

    /// Rev-647: the hash of a gameframe slot on the current toplevel, or 0 if
    /// the toplevel has no such slot.
    #[cfg(since_647)]
    fn toplevel_slot(&self, slots: &HashMap<u16, u16>) -> i32 {
        let top = self.player.toplevel;
        slots
            .get(&top)
            .map_or(0, |&com| ((top as i32) << 16) | com as i32)
    }

    /// Rev-647: sets the client-side event mask for a component slot range.
    #[cfg(since_647)]
    pub fn if_set_events(&mut self, com: i32, slot1: u16, slot2: u16, settings: i32) {
        let from = if slot1 == u16::MAX { -1 } else { slot1 as i32 };
        let to = if slot2 == u16::MAX { -1 } else { slot2 as i32 };
        for slot in from..=to {
            self.player
                .if_events
                .insert((com, slot as u16), settings as u32);
        }
        self.write(IfSetEvents {
            com,
            slot1,
            slot2,
            settings,
        });
    }

    /// Rev-647: transmits a client variable, picking the small or large
    /// packet by value range. The value is mirrored on the player so
    /// server-side consumers (`applydesign`) can read the design preview back.
    #[cfg(since_647)]
    pub fn varc_transmit(&mut self, id: u16, val: i32) {
        let varcs = engine().varcs();
        let varc = varcs.get_by_id(id, false).unwrap();
        self.player.varcs[varcs.index(id, false)] = VarValue::from_int(varc.var_type, val);
        if (-128..=127).contains(&val) {
            self.write(VarcSmall { id, val: val as u8 });
        } else {
            self.write(VarcLarge { id, val });
        }
    }

    /// Rev-647: transmits a string client variable.
    #[cfg(since_647)]
    pub fn varc_str_transmit(&mut self, id: u16, val: &str) {
        let index = engine().varcs().index(id, true);
        self.player.varcs[index] = VarValue::String(val.into());
        // CLIENT_SETVARCSTR_SMALL's frame size is one byte: a longer string (Unstable
        // Foundations' run hint) wrapped it, the client read the stream out of step and dropped
        // the connection ("Connection lost", in-client runs ufrest10 and ufrest13).
        if VarcStrSmall::sizeof_str(val) > u8::MAX as usize {
            self.write(VarcStrLarge { id, val });
        } else {
            self.write(VarcStrSmall { id, val });
        }
    }

    /// Rev-647: invokes a clientscript (cs2) with no parameters.
    #[cfg(since_647)]
    pub fn run_client_script(&mut self, script_id: i32) {
        self.write(RunClientScript {
            script_id,
            params: Vec::new(),
        });
    }

    /// Rev-647 `runclientscript*`: invokes a CS2 script with typed int/string
    /// parameters.
    #[cfg(since_647)]
    pub fn run_client_script_with_params(&mut self, script_id: i32, args: Vec<ScriptArgument>) {
        let params = args
            .into_iter()
            .map(|a| match a {
                ScriptArgument::Int(v) => ScriptParam::Int(v),
                ScriptArgument::String(s) => ScriptParam::Str(s),
            })
            .collect();
        self.write(RunClientScript { script_id, params });
    }

    /// Rev-647 public chat: `MESSAGE_PUBLIC` for each player this client
    /// tracks, itself included, that spoke this tick. The client drops chat
    /// from a player it doesn't have, so this follows the tick's player info.
    #[cfg(since_647)]
    pub fn update_chats(&mut self, renderer: &PlayerRenderer) {
        for (w, &spoke) in renderer.chatted().iter().enumerate() {
            let mut bits = spoke & self.player.build_area.locals[w];
            while bits != 0 {
                let pid = ((w << 6) | bits.trailing_zeros() as usize) as u16;
                bits &= bits - 1;
                self.write(message_public::MessagePublic {
                    pid,
                    chat: renderer.chat(pid),
                });
            }
        }
    }

    /// Sends the pre-encoded NPC info update payload to the client.
    pub fn npc_info(&mut self, bytes: &[u8]) {
        self.write(NpcInfo { bytes });
    }

    /// Sends the pre-encoded player info update payload to the client.
    pub fn player_info(&mut self, bytes: &[u8]) {
        self.write(PlayerInfo { bytes });
    }

    /// [`player_info`](Self::player_info) for a payload in two parts, the
    /// bit section and the update blocks, as `PlayerInfo::encode` builds it.
    pub fn player_info_parts(&mut self, parts: [&[u8]; 2]) {
        if self.capture.is_some() {
            self.player_info(&parts.concat());
            return;
        }
        self.write_varshort_parts(ServerProt::PlayerInfo as u8, parts);
    }

    /// [`npc_info`](Self::npc_info) for a payload in two parts, the bit
    /// section and the update blocks, as `NpcInfo::encode` builds it.
    pub fn npc_info_parts(&mut self, parts: [&[u8]; 2]) {
        if self.capture.is_some() {
            self.npc_info(&parts.concat());
            return;
        }
        self.write_varshort_parts(ServerProt::NpcInfo as u8, parts);
    }

    /// [`write_immediate`](Self::write_immediate) for a var-short message
    /// whose body is `parts` in order. The parts go straight into the gathered
    /// output, so each byte is copied once, not joined and then copied
    /// through the write queue.
    fn write_varshort_parts(&mut self, prot: u8, parts: [&[u8]; 2]) {
        let size = parts[0].len() + parts[1].len();
        if 1 + 2 + size > MAX_PACKET_SIZE {
            return;
        }
        self.write_varshort_unchecked(prot, parts);
    }

    /// [`write_varshort_parts`](Self::write_varshort_parts) without the size
    /// check, for `::readerstall`'s body the size of the client's whole packet
    /// buffer.
    pub(crate) fn write_varshort_unchecked(&mut self, prot: u8, parts: [&[u8]; 2]) {
        let handle = &mut self.handle;
        let opcode = (prot as u32 + handle.isaac_encode.next_int()) as u8;
        let rest = handle.gather_varshort(opcode, parts);
        let engine = engine();
        self.watch.server(engine.clock, prot, rest);
        self.watch.remember(prot, 2, rest);
        if !engine.gather_output {
            self.handle.flush();
        }
    }

    /// Opens the count dialog (number input) on the client. Rev 647 has no
    /// packet for it: the `meslayer_mode7` clientscript shows the prompt,
    /// 289's client's, in the chat box and answers RESUME_P_COUNTDIALOG.
    pub fn p_countdialog(&mut self) {
        #[cfg(before_647)]
        self.write(PCountDialog);
        #[cfg(since_647)]
        self.write(RunClientScript {
            script_id: engine().layout.count_dialog,
            params: vec![ScriptParam::Str("Enter amount:".into())],
        });
    }

    pub(crate) fn needs_rebuild(&self) -> bool {
        let area = &self.player.build_area;
        let coord = self.player.pathing.coord;
        #[cfg(since_647)]
        if area.rebuild || (coord.x() >= REGION_X) != area.dynamic {
            return true;
        }
        area.needs_rebuild(&coord)
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
        if !self.needs_rebuild() && !reconnecting {
            return;
        }
        let coord = self.player.pathing.coord;
        #[cfg(since_647)]
        let dynamic = coord.x() >= REGION_X;
        self.player.build_area.rebuild_normal(&coord);
        let base = self.map_base();
        self.watch.anticheat.rebuilt(base, engine().clock);
        let zone_x = self.player.pathing.coord.zone_x();
        let zone_z = self.player.pathing.coord.zone_z();
        #[cfg(rev = "225")]
        {
            let cache = engine().cache;
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
            self.write(RebuildNormal {
                zone_x,
                zone_z,
                mapsquares: mapsquares.into_iter().collect(),
                crcs: crcs.into_iter().collect(),
            });
        }
        #[cfg(all(since_244, before_647))]
        self.write(RebuildNormal { zone_x, zone_z });
        // Rev-647: the rebuild carries the PlayerInfo init bit-stream on the
        // first login (self position + 18-bit packed regions per slot) and
        // the XTEA key blocks for every mapsquare covering the build area.
        #[cfg(since_647)]
        {
            self.player.build_area.rebuild = false;
            if dynamic != self.player.build_area.dynamic {
                self.forget_npcs();
                self.player.build_area.dynamic = dynamic;
            }
            if dynamic {
                self.rebuild_region(zone_x, zone_z, coord);
                return;
            }
            let first_login = self.player.pending_login;
            let player_regions = if first_login {
                engine().player_renderer.regions().to_vec()
            } else {
                Vec::new()
            };
            let xtea_keys = XteaKeyStore::global()
                .map(|s| s.snapshot_build_area(coord.x(), coord.z()))
                .unwrap_or_default();
            self.write(RebuildNormal {
                self_index: self.player.uid.pid(),
                player_x: coord.x(),
                player_z: coord.z(),
                player_plane: coord.y(),
                login: first_login,
                player_regions,
                xtea_keys,
            });
            if first_login {
                self.player.pending_login = false;
            }
        }
    }

    /// Sends the zones copied onto the 13x13 zones around `zone_x`, `zone_z`,
    /// on every level, with the XTEA keys of the mapsquares they come from in
    /// the order the client first meets them. A rebuild between this and
    /// REBUILD_NORMAL empties the client's npc list.
    #[cfg(since_647)]
    fn rebuild_region(&mut self, zone_x: u16, zone_z: u16, coord: CoordGrid) {
        let regions = &engine().regions;
        let mut templates = Vec::with_capacity((13 * 13) << 2);
        let mut squares: Vec<(u16, u16)> = Vec::new();
        for level in 0..4 {
            for x in zone_x - 6..=zone_x + 6 {
                for z in zone_z - 6..=zone_z + 6 {
                    let template = regions.template_at(x, level, z);
                    if let Some(square) = template.map(template_square)
                        && !squares.contains(&square)
                    {
                        squares.push(square);
                    }
                    templates.push(template);
                }
            }
        }
        let xtea_keys = XteaKeyStore::global()
            .map(|store| {
                squares
                    .iter()
                    .map(|&(mx, mz)| store.get(mx as u32, mz as u32))
                    .collect()
            })
            .unwrap_or_default();
        self.write(RebuildRegion {
            player_x: coord.x(),
            player_z: coord.z(),
            size: 0,
            map_type: 2,
            force: true,
            templates,
            xtea_keys,
        });
    }

    /// Stops tracking every npc, as the client's npc list was emptied.
    #[cfg(since_647)]
    fn forget_npcs(&mut self) {
        let npcs = &mut engine_mut().npc_list;
        for &nid in self.player.build_area.npcs.iter() {
            if let Some(npc) = npcs.get_mut(nid) {
                npc.npc.observers = npc.npc.observers.saturating_sub(1);
            }
        }
        self.player.build_area.npcs.clear();
    }

    /// Sends an animation reset packet to the client, clearing all playing
    /// entity animations.
    pub fn reset_anims(&mut self) {
        #[cfg(before_647)]
        self.write(ResetAnims);
        // TODO(647): rev-647 has no RESET_ANIMS packet modelled yet.
    }

    /// Tells the client to clear its cached varp values, forcing a full
    /// resync on the next varp transmit.
    pub fn reset_client_varcache(&mut self) {
        #[cfg(before_647)]
        self.write(ResetClientVarCache);
        // TODO(647): rev-647 has no RESET_CLIENT_VARCACHE packet modelled yet.
    }

    /// Plays a synthesised sound effect on the client.
    pub fn synth_sound(&mut self, synth: u16, loops: u8, delay: u16) {
        #[cfg(before_647)]
        self.write(SynthSound {
            synth,
            loops,
            delay,
        });
        #[cfg(since_647)]
        self.write(SynthSound {
            synth,
            loops,
            delay,
            volume: 255,
            speed: 256,
        });
    }

    /// Plays a voice-over clip on the client's voice channel. 65535 is no clip.
    #[cfg(since_647)]
    pub fn speech_sound(&mut self, vorbis: u16, loops: u8, delay: u16, volume: u8) {
        self.write(VorbisSpeechSound {
            vorbis,
            loops,
            delay,
            volume,
        });
    }

    /// Stops every voice-over clip the client is playing or has queued.
    #[cfg(since_647)]
    pub fn speech_stop(&mut self) {
        self.write(VorbisSpeechStop);
    }

    /// Clears the destination map flag (red minimap marker) on the client.
    pub fn unset_map_flag(&mut self) {
        #[cfg(before_647)]
        self.write(UnsetMapFlag);
        // Rev-647 clears the flag by setting it to the (255, 255) sentinel.
        #[cfg(since_647)]
        self.write(SetMapFlag { x: 255, z: 255 });
    }

    /// Sends a full inventory update to the client for the given component.
    #[cfg(before_647)]
    pub fn update_inv_full(&mut self, com: u16, objs: &[Option<(u16, i32)>]) {
        // prevent a client crash by capping to inv size || interface size
        let size = engine()
            .interfaces()
            .get_by_id(com)
            .map(|c| c.width as usize * c.height as usize)
            .unwrap_or(objs.len())
            .min(objs.len());
        self.write(UpdateInvFull {
            com,
            objs: &objs[..size],
        });
    }

    /// Sends a full inventory update to the client. Rev-647 keys client
    /// inventories by inv id — the interface binding comes from the cache
    /// component defs, so no component travels on the wire.
    #[cfg(since_647)]
    pub fn update_inv_full(&mut self, inv_id: u16, objs: &[Option<(u16, i32)>]) {
        self.send_inv_full(inv_id, false, objs);
    }

    /// Sends another player's inventory whole, which the 647 client keeps
    /// apart from the player's own copy of that inv (trade's other offer).
    #[cfg(since_647)]
    pub fn update_inv_other_full(&mut self, inv_id: u16, objs: &[Option<(u16, i32)>]) {
        self.send_inv_full(inv_id, true, objs);
    }

    #[cfg(since_647)]
    fn send_inv_full(&mut self, inv_id: u16, other: bool, objs: &[Option<(u16, i32)>]) {
        let slots: Vec<Option<InvItem>> = objs
            .iter()
            .map(|slot| {
                slot.map(|(id, num)| InvItem {
                    id,
                    count: num.max(0) as u32,
                })
            })
            .collect();
        self.write(UpdateInvFull {
            inv_id,
            other,
            capacity: slots.len() as u16,
            slots,
        });
    }

    /// Sends a partial inventory update to the client.
    ///
    /// Each entry is `(slot, item)` for one changed slot.
    pub fn update_inv_partial(&mut self, id: u16, objs: &[(u16, Option<(u16, i32)>)]) {
        self.write(UpdateInvPartial {
            #[cfg(before_647)]
            com: id,
            #[cfg(since_647)]
            inv_id: id,
            #[cfg(since_647)]
            other: false,
            objs,
        });
    }

    /// Tells the client to stop expecting inventory updates for a component.
    pub fn update_inv_stop_transmit(&mut self, com: ComId) {
        #[cfg(before_647)]
        self.write(UpdateInvStopTransmit { com: com as u16 });
        // TODO(647): port with the inventory milestone — no rev-647
        // stop-transmit packet is modelled yet.
        #[cfg(since_647)]
        let _ = com;
    }

    /// Sends the player's server-side player ID to the client.
    pub fn update_pid(&mut self, pid: u16) {
        #[cfg(rev = "225")]
        self.write(UpdatePid { pid });
        #[cfg(all(since_244, before_647))]
        self.write(UpdatePid {
            pid,
            members: self.member(),
        });
        // Rev-647 has no UPDATE_PID packet — the slot index travels in the
        // rebuild/GPI init block instead (map/login milestone).
        #[cfg(since_647)]
        let _ = pid;
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
        self.write(UpdateStat {
            stat: stat as u8,
            exp: self.player.stats.xp[stat] / 10,
            lvl: self.player.stats.levels[stat] as u8,
        });
    }

    /// Sends a run energy update packet to the client.
    ///
    /// The energy value is divided by 100 to convert from internal
    /// representation to the client's 0-100 percentage scale.
    pub fn update_runenergy(&mut self, energy: u16) {
        self.write(UpdateRunEnergy {
            energy: (energy / 100) as u8,
        });
    }

    /// Transmits a varp (player variable) value to the client, automatically
    /// choosing the small (1-byte) or large (4-byte) encoding based on the
    /// value range.
    pub fn varp_transmit(&mut self, id: u16, val: i32) {
        if (-128..=127).contains(&val) {
            self.varp_small(id, val as u8);
        } else {
            self.varp_large(id, val);
        }
    }

    /// Sends a varp update using the 4-byte (large) encoding.
    pub fn varp_large(&mut self, id: u16, val: i32) {
        self.write(VarpLarge { id, val });
    }

    /// Sends a varp update using the 1-byte (small) encoding.
    pub fn varp_small(&mut self, id: u16, val: u8) {
        self.write(VarpSmall { id, val });
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
        let transmits = take(&mut self.player.inv_transmits);

        // A `runweight` inventory changed (recompute & maybe send weight).
        let mut runweight_changed = false;
        // A player inventory was first-seen this tick (force a weight send so the
        // client has the correct value, e.g. right after logging in).
        let mut first_seen_weight = false;

        let invs = engine().invs();

        for (inv_id, coms) in &transmits {
            let inv_type = invs.get_by_id(*inv_id);
            let is_shared = inv_type.is_some_and(|t| t.scope == InvScope::Shared);
            let runweight_inv = inv_type.is_some_and(|t| t.runweight);

            let inv = if is_shared {
                shared_invs.get(inv_id)
            } else {
                self.player.invs.get(inv_id)
            };

            let Some(inv) = inv else { continue };
            let has_dirty = !inv.dirty_slots.is_empty();
            // Before 647 first sight is per component; from 647 it is per inv
            // (below), so the per-component scratch payload is never built.
            #[cfg(before_647)]
            let any_first_seen = coms
                .iter()
                .any(|&c| !self.player.inv_first_seen.contains(&(c as u16)));
            #[cfg(since_647)]
            let any_first_seen = false;

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
            // Rev-647 clients key inventories by inv id, and the interface
            // binding lives in the cache component defs — so send one full
            // update per inv on first sight and whenever a slot changed,
            // regardless of how many components transmit it.
            #[cfg(since_647)]
            {
                let _ = coms;
                let first = !self.player.inv_first_seen.contains(inv_id);
                if first || has_dirty {
                    let full: Vec<Option<(u16, i32)>> = inv
                        .slots
                        .iter()
                        .map(|i| i.map(|it| (it.obj, it.num as i32)))
                        .collect();
                    self.update_inv_full(*inv_id, &full);
                    if first {
                        self.player.inv_first_seen.insert(*inv_id);
                        any_full = true;
                    }
                }
            }
            #[cfg(before_647)]
            for &com in coms {
                let com = com as u16;
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
            #[cfg(before_647)]
            self.write(UpdateRunWeight {
                kg: (self.player.runweight / 1000) as u16,
            });
            // TODO(647): rev-647 has no UPDATE_RUNWEIGHT packet — weight is
            // surfaced client-side (interface milestone).
        }
    }

    /// Mirrors other players' inventories onto this player's interface components.
    ///
    /// Processes every `invother_transmit` listener: resolves the source player by
    /// script-uid, reads the requested inventory, and transmits it to the bound
    /// component. The first transmit for a component is a full update; subsequent
    /// ticks send only the source inventory's changed slots (its dirty set survives
    /// until the cleanup phase, so every viewer this tick sees the same changes).
    /// From 647 it goes by inv id with the client's "other" flag, whole on first
    /// sight and on any change.
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
    pub fn update_other_invs(&mut self, players: OtherPlayers<'_>) {
        if self.player.inv_other_transmits.is_empty() {
            return;
        }

        #[cfg(before_647)]
        thread_local! {
            static FULL: RefCell<FullInvUpdate> = RefCell::new(Vec::with_capacity(48));
            static PARTIAL: RefCell<PartialInvUpdate> = RefCell::new(Vec::with_capacity(48));
        }

        // Take the map out so `&mut self` methods (update_inv_full) can be called
        // while iterating, then restore it. Nothing in the loop mutates the map.
        let transmits = take(&mut self.player.inv_other_transmits);
        // From 647 one update goes per inv, however many components show it.
        #[cfg(since_647)]
        let mut sent: Vec<u16> = Vec::new();

        for (&com, &(uid, inv_id)) in &transmits {
            let Some(source) = players.get((uid & 0x7FF) as u16) else {
                continue;
            };
            // Guard against a reused player slot: the live uid must still match.
            if source.player.uid.packed() as i32 != uid {
                continue;
            }
            let Some(inv) = source.player.invs.get(&inv_id) else {
                continue;
            };

            // Rev-647 clients key another player's inv by its inv id and the
            // "other" flag, apart from their own copy of that inv: send it
            // whole on first sight and whenever a slot changed, as the
            // player's own invs go.
            #[cfg(since_647)]
            {
                let _ = com;
                let first = !self.player.inv_other_first_seen.contains(&inv_id);
                if sent.contains(&inv_id) || (!first && inv.dirty_slots.is_empty()) {
                    continue;
                }
                let full: Vec<Option<(u16, i32)>> = inv
                    .slots
                    .iter()
                    .map(|i| i.map(|it| (it.obj, it.num as i32)))
                    .collect();
                self.update_inv_other_full(inv_id, &full);
                self.player.inv_other_first_seen.insert(inv_id);
                sent.push(inv_id);
            }
            #[cfg(before_647)]
            {
                let com = com as u16;
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
        let invs = engine().invs();
        let objs = engine().objs();
        let mut weight = 0;
        for (inv_id, inv) in &self.player.invs {
            let Some(inv_type) = invs.get_by_id(*inv_id) else {
                continue;
            };
            if !inv_type.runweight {
                continue;
            }
            for item in inv.slots.iter().flatten() {
                let Some(obj_type) = objs.get_by_id(item.obj) else {
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
        if let Some(font) = engine().fonts().get(Font::P12) {
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
        let varps = engine().varps();
        for id in 0..self.player.vars.len() {
            let Some(varp) = varps.get_by_id(id as u16) else {
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
        #[cfg(before_647)]
        self.write(TutOpen { com });
        // TODO(647): rev-647 has no TUT_OPEN packet (interface milestone).
        self.player.open_tutorial(com);
    }

    /// Flashes a tutorial tab on the client to draw the player's attention.
    ///
    /// # Arguments
    /// * `tab` - The tab index to flash.
    pub fn tut_flash(&mut self, tab: u8) {
        #[cfg(before_647)]
        self.write(TutFlash { tab });
        // TODO(647): rev-647 has no TUT_FLASH packet (interface milestone).
        #[cfg(since_647)]
        let _ = tab;
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
                (
                    ServerTriggerType::IfClose,
                    Some(modal_tutorial as i32),
                    None,
                ),
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
            #[cfg(before_647)]
            self.write(TutOpen { com: u16::MAX });
            // TODO(647): rev-647 has no TUT_OPEN packet (interface milestone).
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

    /// Sets the appearance "skill level" (RuneScript `SET_SKILL_LEVEL`) and
    /// rebuilds the appearance so the new value is reflected to other players.
    #[cfg(since_274)]
    pub fn set_skill_level(&mut self, level: u16) {
        self.player.skill_level = level;
        if let Some(appearance) = self.player.info.appearance {
            self.buildappearance(appearance);
        }
    }

    /// Disguises the player as the given npc type (RuneScript `P_TRANSMOGRIFY`),
    /// or clears the disguise when `npc` is `None`, then rebuilds the appearance
    /// so the change is reflected to other players.
    #[cfg(since_254)]
    pub fn transmogrify(&mut self, npc: Option<u16>) {
        self.player.transmog = npc;
        if let Some(appearance) = self.player.info.appearance {
            self.buildappearance(appearance);
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
        let amount = amount as u16;
        let current = self.player.stats.levels[PlayerStat::Hitpoints as usize];
        let taken = if current.saturating_sub(amount) == 0 {
            self.player.stats.levels[PlayerStat::Hitpoints as usize] = 0;
            current
        } else {
            self.player.stats.levels[PlayerStat::Hitpoints as usize] =
                current.saturating_sub(amount);
            amount
        };
        let remaining = self.player.stats.levels[PlayerStat::Hitpoints as usize] as u8;
        let base = self.player.stats.base_levels[PlayerStat::Hitpoints as usize] as u8;

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

    /// Paints a 647 hitsplat without touching a stat.
    ///
    /// 2011's hitpoints are life points, and content owns them (varbit 7198,
    /// ten to a hitpoint: 2011 wiki, Constitution), so content takes the hit
    /// off, works out the health bar's fill, and hands the engine only what to
    /// draw.
    ///
    /// # Arguments
    /// * `amount` - The hit in life points, which passes 255.
    /// * `hitmark` - 2011's hitmark config (config group 46).
    /// * `soak` - Life points armour soaked off the hit (2011's damage
    ///   soaking), painted in a blue splat beside it; 0 for none.
    /// * `healthbar` - The overhead bar's fill, 0-255.
    /// * `source` - The pid of the player who dealt it, if a player did: they
    ///   see it bright, as this player does; everyone else darker.
    #[cfg(since_647)]
    pub fn hit(&mut self, amount: u16, hitmark: u8, soak: u16, healthbar: u8, source: Option<u16>) {
        let soak = (soak > 0).then_some(soak);
        if self.player.info.apply_damage2(
            amount,
            hitmark,
            healthbar,
            u8::MAX,
            PlayerInfoProt::Damage2 as u16,
        ) {
            self.player.info.damage2_soak = soak;
            self.player.info.damage2_source = source;
            return;
        }
        self.player.info.apply_damage(
            amount,
            hitmark,
            healthbar,
            u8::MAX,
            PlayerInfoProt::Damage as u16,
        );
        self.player.info.damage_soak = soak;
        self.player.info.damage_source = source;
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

    /// Generates and caches this player's appearance body (rs-protocol's
    /// [`Appearance`]): equipment, body kits, colours, animations, name and
    /// levels.
    ///
    /// # Arguments
    /// * `clock` - The current game tick, stored as the appearance timestamp.
    pub fn generateappearance(&mut self, clock: u32) {
        thread_local! {
            static SCRATCH: RefCell<Packet> = RefCell::new(Packet::new(256));
        }

        let appearance: Box<[u8]> = SCRATCH.with_borrow_mut(|buf| {
            buf.pos = 0;
            let info = &self.player.info;
            Appearance {
                gender: self.player.gender,
                headicons: self.player.headicons,
                look: self.look(),
                colours: &self.player.colours,
                #[cfg(before_647)]
                anims: [
                    info.readyanim.unwrap(),
                    info.turnanim.unwrap(),
                    info.walkanim.unwrap(),
                    info.walkanim_b.unwrap(),
                    info.walkanim_l.unwrap(),
                    info.walkanim_r.unwrap(),
                    info.runanim.unwrap(),
                ],
                #[cfg(before_647)]
                username37: self.uid().username37(),
                #[cfg(since_647)]
                bas: info.readyanim.unwrap_or(DEFAULT_BAS),
                #[cfg(since_647)]
                name: &self.player.shown_name(),
                combat_level: self.player.combat_level,
                #[cfg(since_274)]
                skill_level: self.player.skill_level,
            }
            .encode(buf);
            Box::from(&buf.data[..buf.pos])
        });

        self.player.info.last_appearance = Some(clock);
        self.player.info.last_appearance_info = Some(appearance);
    }

    /// What the player looks like: what they wear, or the npc they turned
    /// into.
    fn look(&self) -> Look {
        #[cfg(since_254)]
        if let Some(npc) = self.player.transmog {
            return Look::Npc(npc);
        }
        Look::Worn(self.worn())
    }

    /// What the player wears in each wear position: an item, else their body
    /// kit. Items hide the additional slots named by their wearpos2/wearpos3.
    fn worn(&self) -> [Wear; WEAR_SLOTS] {
        let worn = self
            .player
            .invs
            .get(&self.player.info.appearance.unwrap_or(0));
        let mut skipped_slots = [false; WEAR_SLOTS];
        if let Some(worn) = worn {
            let objs = engine().objs();
            for item in worn.slots.iter().flatten() {
                if let Some(obj) = objs.get_by_id(item.obj) {
                    if let Some(wp2) = obj.wearpos2.filter(|&w| (w as usize) < WEAR_SLOTS) {
                        skipped_slots[wp2 as usize] = true;
                    }
                    if let Some(wp3) = obj.wearpos3.filter(|&w| (w as usize) < WEAR_SLOTS) {
                        skipped_slots[wp3 as usize] = true;
                    }
                }
            }
        }

        let mut slots = [Wear::None; WEAR_SLOTS];
        for (slot, wear) in slots.iter_mut().enumerate() {
            if skipped_slots[slot] {
                continue;
            }
            if let Some(item) = worn.and_then(|inv| inv.get(slot as u16)) {
                #[cfg(before_647)]
                {
                    *wear = Wear::Obj(item.obj);
                }
                #[cfg(since_647)]
                if let Some(wearid) = engine()
                    .objs()
                    .get_by_id(item.obj)
                    .and_then(|obj| obj.wearid)
                {
                    *wear = Wear::Obj(wearid);
                }
            } else if let Some(kit) = self.body_kit(slot) {
                *wear = Wear::Kit(kit);
            }
        }
        slots
    }

    /// The body kit shown in wear position `slot` when nothing is worn there:
    /// none for a slot without a body part or a hidden part.
    fn body_kit(&self, slot: usize) -> Option<u16> {
        let part = BODY_WEARPOS.iter().position(|&wearpos| wearpos == slot)?;
        u16::try_from(self.player.body[part]).ok()
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
        let seqs = engine().seqs();
        let cur_pri = self
            .player
            .info
            .anim_id
            .and_then(|a| seqs.get_by_id(a))
            .map(|s| s.priority as u16);
        let new_pri = id
            .and_then(|a| seqs.get_by_id(a))
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
        let Some(varp) = engine().varps().get_by_debugname(name) else {
            return false;
        };
        let id = varp.id;
        let transmit = varp.transmit;
        let value = VarValue::from_int(varp.var_type, value);
        self.set_varp(id, value, transmit);
        true
    }

    /// Reads a varbit: its bit range out of the base varp.
    #[cfg(since_254)]
    pub fn varbit(&self, varbit: &VarbitType) -> i32 {
        let base = self.player.vars.get(varbit.basevar).as_int();
        getbit_range(base, varbit.start_bit as i32, varbit.end_bit as i32)
    }

    /// Writes a varbit into its bit range of the base varp, leaving the other
    /// bits alone. A value outside the range stores 0, like the reference
    /// engine's `setVarBit`. The base varp's `transmit` flag decides whether
    /// the client is sent the change.
    #[cfg(since_254)]
    pub fn set_varbit(&mut self, varbit: &VarbitType, value: i32) {
        let Some(varp) = engine().varps().get_by_id(varbit.basevar) else {
            return;
        };
        let (start, end) = (varbit.start_bit as i32, varbit.end_bit as i32);
        let value = if fits_bit_range(value, start, end) {
            value
        } else {
            0
        };
        let current = self.player.vars.get(varbit.basevar).as_int();
        let new_value = setbit_range_toint(current, value, start, end);
        self.set_varp(
            varbit.basevar,
            VarValue::from_int(varp.var_type, new_value),
            varp.transmit,
        );
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
    /// the client for all zones in the build area. `changed` holds the zones
    /// with events this tick.
    fn update_zones(&mut self, zones: &ZoneMap, changed: &FxHashSet<ZoneCoordGrid>, clock: u32);

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

/// Requests turned down that a real client makes too: a second click on a
/// dialog's "continue" after it closed, an option the content has no script
/// for. (A mistyped cheat command is the player's own, and isn't counted
/// either.)
const UNREMARKABLE: [&str; 2] = ["Trigger not found", "Player does not have an active"];

/// How many bytes the read queue holds before the next message waits a tick.
const READ_QUEUE_LIMIT: usize = MAX_PACKET_SIZE;

impl EnginePlayer for ActivePlayer {
    /// Drains the network inbox into a read queue, then processes packets
    /// in a loop until per-category rate limits are reached or no whole
    /// packet is left.
    ///
    /// Messages that would exceed the [`READ_QUEUE_LIMIT`] are held as
    /// pending for the next tick.
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
            if handle.read_queue.len() + msg.1.len() > READ_QUEUE_LIMIT {
                handle.pending_msg = Some(msg);
                break;
            }
            handle.queue(msg.0, msg.1);
        }

        self.client_limit = 0;
        self.user_limit = 0;
        self.restricted_limit = 0;

        while self.client_limit < ClientProtCategory::ClientEvent as u8
            && self.user_limit < ClientProtCategory::UserEvent as u8
            && self.restricted_limit < ClientProtCategory::RestrictedEvent as u8
            && self.read().is_some()
        {}

        received
    }

    /// Reads a single client protocol message from the read queue, decodes
    /// it, and dispatches it to the appropriate handler.
    ///
    /// Decrypts the opcode byte using the ISAAC cipher, determines the
    /// packet length from the frame type (fixed, var-byte, var-short),
    /// and routes to the matching `ClientProt` handler. Then counts the
    /// packet against the rate limits.
    ///
    /// A packet whose payload hasn't all arrived is left in the queue, its
    /// opcode kept on the handle for the next tick. An unknown opcode, or
    /// a length the read queue can never hold, closes the connection: the
    /// rest of the stream can't be read after it. Every revision's table
    /// holds every opcode its client sends.
    ///
    /// A var-frame payload shorter than the decoder's declared minimum is
    /// drained and dropped rather than decoded.
    ///
    /// # Returns
    /// `Some(())` if a message was processed, `None` if no whole packet
    /// is left or the connection was closed.
    ///
    /// # Side Effects
    /// * Increments `client_limit`, `user_limit`, or `restricted_limit`.
    /// * May send game messages or log errors on handler failure.
    ///
    /// # Call Stack
    /// **Called by:** [`decode`](Self::decode)
    fn read(&mut self) -> Option<()> {
        let handle = &mut self.handle;
        let opcode = match handle.opcode {
            Some(opcode) => opcode,
            None => {
                let byte = handle.read_queue.pop_front()?;
                let opcode = byte.wrapping_sub(handle.isaac_decode.next_int() as u8);
                handle.opcode = Some(opcode);
                opcode
            }
        };

        let Ok(prot) = ClientProt::try_from(opcode) else {
            warn!("Unknown opcode {opcode}, closing the connection");
            self.disconnect();
            return None;
        };
        let info = prot.info();

        let queue = &self.handle.read_queue;
        let (head, len) = match info.frame {
            ClientProtFrame::VarByte { .. } => (1, *queue.front()? as usize),
            ClientProtFrame::VarShort { .. } => {
                let hi = *queue.front()? as usize;
                let lo = *queue.get(1)? as usize;
                (2, (hi << 8) | lo)
            }
            ClientProtFrame::Fixed(len) => (0, len as usize),
        };

        if head + len > READ_QUEUE_LIMIT {
            warn!("{prot:?} of {len} bytes, closing the connection");
            self.disconnect();
            return None;
        }
        if queue.len() < head + len {
            return None;
        }

        let handle = &mut self.handle;
        handle.opcode = None;
        handle.read_queue.drain(..head);
        let mut buf = Packet::from(take(&mut handle.read_buf));
        buf.data.clear();
        buf.data.extend(handle.read_queue.drain(..len));
        let malformed = !prot.validate(&buf.data);

        if let Some(at) = self.handle.arrived() {
            self.watch.arrived(at);
        }
        self.watch.client(engine().clock, &prot, opcode, &buf.data);

        let independent = match prot {
            ClientProt::MoveGameClick
            | ClientProt::MoveMinimapClick
            | ClientProt::MessagePublic
            | ClientProt::NoTimeout => true,
            #[cfg(before_647)]
            ClientProt::MoveOpClick => true,
            #[cfg(any(rev = "225", since_254))]
            ClientProt::EventCameraPosition => true,
            #[cfg(since_254)]
            ClientProt::EventAppletFocus
            | ClientProt::EventMouseClick
            | ClientProt::EventMouseMove => true,
            _ => false,
        };
        if !independent {
            engine_mut().flush_input_paths();
        }

        #[rustfmt::skip]
        let success = if malformed {
            debug!("Malformed {prot:?}: {} byte(s)", buf.len());
            self.watch.anticheat.malformed();
            false
        } else {
            let len = buf.len();
            let result = match prot {
                #[cfg(before_647)]
                ClientProt::AnticheatCycleLogic1 => AnticheatCycleLogic1::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatCycleLogic2 => AnticheatCycleLogic2::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatCycleLogic3 => AnticheatCycleLogic3::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatCycleLogic4 => AnticheatCycleLogic4::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatCycleLogic5 => AnticheatCycleLogic5::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatCycleLogic6 => AnticheatCycleLogic6::decode(&mut buf, len).handle(self),
                #[cfg(all(since_254, before_647))]
                ClientProt::AnticheatCycleLogic7 => AnticheatCycleLogic7::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic1 => AnticheatOpLogic1::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic2 => AnticheatOpLogic2::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic3 => AnticheatOpLogic3::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic4 => AnticheatOpLogic4::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic5 => AnticheatOpLogic5::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic6 => AnticheatOpLogic6::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic7 => AnticheatOpLogic7::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic8 => AnticheatOpLogic8::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::AnticheatOpLogic9 => AnticheatOpLogic9::decode(&mut buf, len).handle(self),
                ClientProt::ChatSetMode => ChatSetMode::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::SetChatfiltersettings => SetChatfiltersettings::decode(&mut buf, len).handle(self),
                ClientProt::ClientCheat => ClientCheat::decode(&mut buf, len).handle(self),
                ClientProt::CloseModal => CloseModal::decode(&mut buf, len).handle(self),
                #[cfg(any(rev = "225", since_254))]
                ClientProt::EventCameraPosition => EventCameraPosition::decode(&mut buf, len).handle(self),
                #[cfg(since_254)]
                ClientProt::EventAppletFocus => EventAppletFocus::decode(&mut buf, len).handle(self),
                #[cfg(since_254)]
                ClientProt::EventMouseClick => EventMouseClick::decode(&mut buf, len).handle(self),
                #[cfg(since_254)]
                ClientProt::EventMouseMove => EventMouseMove::decode(&mut buf, len).handle(self),
                #[cfg(before_274)]
                ClientProt::EventTracking => EventTracking::decode(&mut buf, len).handle(self),
                #[cfg(since_254)]
                ClientProt::MapBuildComplete => MapBuildComplete::decode(&mut buf, len).handle(self),
                ClientProt::FriendListAdd => FriendListAdd::decode(&mut buf, len).handle(self),
                ClientProt::FriendListDel => FriendListDel::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::IdkSaveDesign => IdkSaveDesign::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::IdleTimer => IdleTimer::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::IfButton => IfButton::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton1 => IfButton1::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton2 => IfButton2::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton3 => IfButton3::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton4 => IfButton4::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton5 => IfButton5::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton6 => IfButton6::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton7 => IfButton7::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton8 => IfButton8::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton9 => IfButton9::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButton10 => IfButton10::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButtonD => IfButtonD::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::IfButtonT => IfButtonT::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpNpc6 => OpNpc6::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpLoc6 => OpLoc6::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpObj6 => OpObj6::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::WindowStatus => WindowStatus::decode(&mut buf, len).handle(self),
                ClientProt::IgnoreListAdd => IgnoreListAdd::decode(&mut buf, len).handle(self),
                ClientProt::IgnoreListDel => IgnoreListDel::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::InvButton1 => InvButton1::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::InvButton2 => InvButton2::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::InvButton3 => InvButton3::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::InvButton4 => InvButton4::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::InvButton5 => InvButton5::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::InvButtonD => InvButtonD::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::MessagePrivate => MessagePrivate::decode(&mut buf, len).handle(self),
                ClientProt::MessagePublic => MessagePublic::decode(&mut buf, len).handle(self),
                ClientProt::MoveGameClick => MoveGameClick::decode(&mut buf, len).handle(self),
                ClientProt::MoveMinimapClick => MoveMinimapClick::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::MoveOpClick => MoveOpClick::decode(&mut buf, len).handle(self),
                ClientProt::NoTimeout => NoTimeout::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpHeld1 => OpHeld1::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpHeld2 => OpHeld2::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpHeld3 => OpHeld3::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpHeld4 => OpHeld4::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpHeld5 => OpHeld5::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpHeldT => OpHeldT::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpHeldU => OpHeldU::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc1 => OpLoc1::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc2 => OpLoc2::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc3 => OpLoc3::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc4 => OpLoc4::decode(&mut buf, len).handle(self),
                ClientProt::OpLoc5 => OpLoc5::decode(&mut buf, len).handle(self),
                ClientProt::OpLocT => OpLocT::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpLocU => OpLocU::decode(&mut buf, len).handle(self),
                ClientProt::OpObj1 => OpObj1::decode(&mut buf, len).handle(self),
                ClientProt::OpObj2 => OpObj2::decode(&mut buf, len).handle(self),
                ClientProt::OpObj3 => OpObj3::decode(&mut buf, len).handle(self),
                ClientProt::OpObj4 => OpObj4::decode(&mut buf, len).handle(self),
                ClientProt::OpObj5 => OpObj5::decode(&mut buf, len).handle(self),
                ClientProt::OpObjT => OpObjT::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpObjU => OpObjU::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc1 => OpNpc1::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc2 => OpNpc2::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc3 => OpNpc3::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc4 => OpNpc4::decode(&mut buf, len).handle(self),
                ClientProt::OpNpc5 => OpNpc5::decode(&mut buf, len).handle(self),
                ClientProt::OpNpcT => OpNpcT::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpNpcU => OpNpcU::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer1 => OpPlayer1::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer2 => OpPlayer2::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer3 => OpPlayer3::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayer4 => OpPlayer4::decode(&mut buf, len).handle(self),
                #[cfg(since_254)]
                ClientProt::OpPlayer5 => OpPlayer5::decode(&mut buf, len).handle(self),
                ClientProt::OpPlayerT => OpPlayerT::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::OpPlayerU => OpPlayerU::decode(&mut buf, len).handle(self),
                #[cfg(rev = "225")]
                ClientProt::RebuildGetMaps => RebuildGetMaps::decode(&mut buf, len).handle(self),
                ClientProt::ResumePCountDialog => ResumePCountDialog::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ResumePStringDialog => {
                    ResumePStringDialog::decode(&mut buf, len).handle(self)
                }
                ClientProt::ResumePauseButton => ResumePauseButton::decode(&mut buf, len).handle(self),
                ClientProt::SendSnapshot => SendSnapshot::decode(&mut buf, len).handle(self),
                #[cfg(before_647)]
                ClientProt::TutClickSide => TutClickSide::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::EventKeyboard => EventKeyboard::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ClientDetailoptionsStatus => ClientDetailoptionsStatus::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::WorldLatency => WorldLatency::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::MapBuildLoading => MapBuildLoading::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::TransmitvarVerifyId => TransmitvarVerifyId::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::VideoEnd => VideoEnd::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ReflectionCheckReply => ReflectionCheckReply::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::FaceSquare => FaceSquare::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpPlayer6 => OpPlayer6::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpPlayer7 => OpPlayer7::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpPlayer8 => OpPlayer8::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpPlayer9 => OpPlayer9::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::OpPlayer10 => OpPlayer10::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ResumePNameDialog => ResumePNameDialog::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ResumePObjDialog => ResumePObjDialog::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ResumePHslDialog => ResumePHslDialog::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ResumePClanforumqfcDialog => ResumePClanforumqfcDialog::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::MessagePrivate => MessagePrivate::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::MessageQuickchatPublic => MessageQuickchatPublic::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::MessageQuickchatPrivate => MessageQuickchatPrivate::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::FriendSetRank => FriendSetRank::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ClanJoinchatLeavechat => ClanJoinchatLeavechat::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ClanKickUser => ClanKickUser::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ClanchannelKickUser => ClanchannelKickUser::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::WorldlistFetch => WorldlistFetch::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ClickWorldMap => ClickWorldMap::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::ApCoordT => ApCoordT::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::SoundSongPreloaded => SoundSongPreloaded::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::UrlRequest => UrlRequest::decode(&mut buf, len).handle(self),
                #[cfg(since_647)]
                ClientProt::CreateCheckName => CreateCheckName::decode(&mut buf, len).handle(self),
            };
            match result {
                Ok(_) => true,
                Err(err) => {
                    #[cfg(debug_assertions)]
                    self.message_game_wrapped(&err.to_string());
                    // The client's own doing: the anticheat counts it, and a
                    // log line each would let a client flood the log.
                    if let ScriptError::Client(why) = &err {
                        debug!("{prot:?}: {why}");
                        if !matches!(prot, ClientProt::ClientCheat)
                            && !UNREMARKABLE.iter().any(|ok| why.starts_with(ok))
                        {
                            self.watch.anticheat.rejected();
                        }
                    } else {
                        error!("{err}");
                    }
                    false
                }
            }
        };

        // A turned-down user event counts as a client event, as in the
        // reference, so rejections stop at that limit.
        match info.category {
            ClientProtCategory::UserEvent if success => self.user_limit += 1,
            ClientProtCategory::RestrictedEvent => self.restricted_limit += 1,
            _ => self.client_limit += 1,
        }

        self.handle.read_buf = buf.data;
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
        // Rev-647: the [login,_] script never calls buildappearance, so seed
        // the Appearance mask here (mirrors the reference server's
        // buildappearance_default) — without it the first GPI add carries no
        // appearance block and the client renders the player null (black
        // screen / renderer NPE on any player-model interface component).
        // The stored move type goes out from `Player::sync_move_types` on the
        // first info phase, as it does whenever it changes.
        #[cfg(since_647)]
        {
            self.player.info.masks |= PlayerInfoProt::Appearance as u16;
        }
        self.rebuild_normal(false);
        self.chat_filter_settings();
        #[cfg(since_254)]
        if engine().ether_tx.is_some() {
            self.friendlist_loaded(1);
        } else {
            self.friendlist_loaded(2);
        }
        self.if_close();
        self.update_pid(self.player.uid.pid());
        self.reset_client_varcache();
        self.sync_varps();
        for i in 0..self.player.stats.len() {
            self.update_stat(i);
        }
        self.update_runenergy(self.player.runenergy);
        self.reset_anims();
        // The client's own input report, for the replay and the checks.
        #[cfg(before_274)]
        self.write(EnableTracking);
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
        #[cfg(since_647)]
        {
            self.player.client_move_type = None;
        }
        self.player.build_area.clear(true);
        self.rebuild_normal(true);
        self.close_modal(true)?;
        self.sync_tabs();
        self.player.inv_first_seen.clear();
        #[cfg(since_647)]
        self.player.inv_other_first_seen.clear();
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
            #[cfg(since_647)]
            ZoneMessage::LocPrefetch(m) => self.write(m.clone()),
            ZoneMessage::MapAnim(m) => self.write(m.clone()),
            ZoneMessage::MapProjAnim(m) => self.write(m.clone()),
            #[cfg(since_647)]
            ZoneMessage::MapProjAnimHalfsq(m) => self.write(m.clone()),
            #[cfg(since_289)]
            ZoneMessage::SoundArea(m) => self.write(m.clone()),
            #[cfg(since_647)]
            ZoneMessage::SoundVorbisArea(m) => self.write(m.clone()),
            #[cfg(since_647)]
            ZoneMessage::TextCoord(m) => self.write(m.clone()),
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
    /// * `changed` - The zones with events this tick.
    /// * `clock` - The current game tick, used for object visibility checks.
    ///
    /// # Side Effects
    /// * Marks zones as loaded in the player's build area.
    /// * Sends zone update packets to the client.
    fn update_zones(&mut self, zones: &ZoneMap, changed: &FxHashSet<ZoneCoordGrid>, clock: u32) {
        if self.update_settled_zones(zones, changed) {
            return;
        }
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
            // Rev-647 zone-update headers carry the level explicitly.
            #[cfg(since_647)]
            let y = zone_coord.y();
            // Rev-647 zone headers carry the zone INDEX, not the tile offset the
            // 2004-era protocol sends: the client rebuilds the base with
            // `g1s() << 3` (see `MapAreaConfig` UPDATE_ZONE_*_FOLLOWS /
            // _ENCLOSED). Sending the raw tile offset made it shift 0..96 up to
            // 0..768 -- far outside the 104-tile build area -- so every zone
            // event (dropped objs, loc changes) landed off-map and never drew.
            #[cfg(since_647)]
            let (x, z) = (x >> 3, z >> 3);
            let newly_loaded =
                !self.player.build_area.loaded_zones[..loaded_before].contains(&zone_coord);

            // Read-only lookup. A zone that was never instantiated has no
            // objs/locs/events, so it yields exactly what the old `zone_mut`
            // path produced (a full-clear when newly loaded) -- without
            // creating and leaving an empty zone in the map.
            match zones.zone(zone_coord.x(), zone_coord.y(), zone_coord.z()) {
                Some(zone) => {
                    if newly_loaded {
                        #[cfg(before_647)]
                        self.write(UpdateZoneFullFollows { x, z });
                        #[cfg(since_647)]
                        self.write(UpdateZoneFullFollows { x, z, y });

                        let mut wrote_follows = false;

                        for obj in zone.visible_objs(user37, clock) {
                            if !wrote_follows {
                                #[cfg(before_647)]
                                self.write(UpdateZonePartialFollows { x, z });
                                #[cfg(since_647)]
                                self.write(UpdateZonePartialFollows { x, z, y });
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
                                    #[cfg(before_647)]
                                    self.write(UpdateZonePartialFollows { x, z });
                                    #[cfg(since_647)]
                                    self.write(UpdateZonePartialFollows { x, z, y });
                                    wrote_follows = true;
                                }
                                self.write(LocDel { coord, shape_angle });
                            } else if loc.lifetime() == EntityLifeTime::Despawn || loc.is_changed()
                            {
                                if !wrote_follows {
                                    #[cfg(before_647)]
                                    self.write(UpdateZonePartialFollows { x, z });
                                    #[cfg(since_647)]
                                    self.write(UpdateZonePartialFollows { x, z, y });
                                    wrote_follows = true;
                                }
                                self.write(LocAddChange {
                                    coord,
                                    shape_angle,
                                    id: loc.id(),
                                });
                            }
                        }

                        #[cfg(since_647)]
                        for merge in zone.loc_merges() {
                            if !wrote_follows {
                                self.write(UpdateZonePartialFollows { x, z, y });
                                wrote_follows = true;
                            }
                            self.write(merge.clone());
                        }
                    } else {
                        if let Some(bytes) = zone.shared_bytes() {
                            #[cfg(before_647)]
                            self.write(UpdateZonePartialEnclosed { x, z, bytes });
                            #[cfg(since_647)]
                            self.write(UpdateZonePartialEnclosed { x, z, y, bytes });
                        }

                        if zone.has_follows_events() {
                            #[cfg(before_647)]
                            self.write(UpdateZonePartialFollows { x, z });
                            #[cfg(since_647)]
                            self.write(UpdateZonePartialFollows { x, z, y });
                            for message in zone.visible_follows_events(user37) {
                                self.write_zone_message(message);
                            }
                        }
                    }
                }
                None => {
                    if newly_loaded {
                        #[cfg(before_647)]
                        self.write(UpdateZoneFullFollows { x, z });
                        #[cfg(since_647)]
                        self.write(UpdateZoneFullFollows { x, z, y });
                    }
                }
            }

            if !self.player.build_area.loaded_zones.contains(&zone_coord) {
                self.player.build_area.loaded_zones.push(zone_coord);
                self.watch
                    .anticheat
                    .zone_sent(zone_coord.x(), zone_coord.z());
            }
        }

        let build_area = &mut self.player.build_area;
        debug_assert!(build_area.active_zones.len() <= 64);
        let mut unloaded = 0;
        for (i, zone) in build_area.active_zones.iter().enumerate() {
            if !build_area.loaded_zones.contains(zone) {
                unloaded |= 1 << i;
            }
        }
        build_area.zones_settled = Some(unloaded);
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
                (ServerTriggerType::IfClose, Some(modal_main as i32), None),
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
            let cleared = self.player.clear_interface_listeners(modal, root_layer);
            for com in cleared {
                self.clear_inv_transmits(com);
            }
            self.player.modal_main = None;
        }

        if let Some(modal_chat) = self.player.modal_chat {
            if let Err(e) = engine_mut().run_script_by_trigger(
                (ServerTriggerType::IfClose, Some(modal_chat as i32), None),
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
            let cleared = self.player.clear_interface_listeners(modal, root_layer);
            for com in cleared {
                self.clear_inv_transmits(com);
            }
            self.player.modal_chat = None;
        }

        if let Some(modal_side) = self.player.modal_side {
            if let Err(e) = engine_mut().run_script_by_trigger(
                (ServerTriggerType::IfClose, Some(modal_side as i32), None),
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
            let cleared = self.player.clear_interface_listeners(modal, root_layer);
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
    /// If the target zone is not allocated, reports an invalid teleport
    /// instead of teleporting. If the destination is on a different level,
    /// a jump flag is set.
    ///
    /// # Arguments
    /// * `coord` - The destination coordinate.
    ///
    /// # Side Effects
    /// * Sets `player.pathing.coord` and `player.pathing.tele = true`.
    /// * Sets `player.jump = true` when changing levels.
    /// * Moves the player's zone presence and collision there now.
    /// * Rev-647: going from the map into region space, keeps the tile it
    ///   left as the player's way back.
    fn tele(&mut self, coord: CoordGrid) {
        #[cfg(since_647)]
        let from = self.player.pathing.coord;
        match self.player.pathing.teleport(coord) {
            None => self.invalid_teleport(),
            Some((look_x, look_z)) => {
                self.cancel_input_path();
                #[cfg(since_647)]
                if coord.x() >= REGION_X && from.x() < REGION_X {
                    self.player.region_entry = Some(from);
                }
                self.player.info.focus(
                    FocusKind::Player,
                    CoordGrid::fine(look_x, self.player.pathing.size),
                    CoordGrid::fine(look_z, self.player.pathing.size),
                    false,
                );
                let block_walk = self.player.block_walk;
                Engine::refresh_presence(
                    &mut engine_mut().zones,
                    &mut self.player.pathing,
                    EntityId::Player(self.player.uid.pid()),
                    block_walk,
                );
            }
        }
    }
}

/// A zone trigger's script name, formatted on the stack. The longest,
/// `[zoneexit,3_255_255_56_56]`, is 26 bytes.
struct TriggerName {
    buf: [u8; 32],
    len: usize,
}

impl TriggerName {
    const fn new() -> Self {
        Self {
            buf: [0; 32],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        str::from_utf8(&self.buf[..self.len]).unwrap_or_default()
    }
}

impl fmt::Write for TriggerName {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        let Some(dst) = self.buf.get_mut(self.len..end) else {
            return Err(fmt::Error);
        };
        dst.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
