#[cfg(before_647)]
use rs_io::Packet;
#[cfg(before_647)]
use rs_protocol::network::game::server::ServerProtMessage;
#[cfg(before_647)]
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
#[cfg(before_647)]
use rs_protocol::network::game::server::loc_anim::LocAnim;
#[cfg(before_647)]
use rs_protocol::network::game::server::loc_del::LocDel;
#[cfg(before_647)]
use rs_protocol::network::game::server::loc_merge::LocMerge;
#[cfg(before_647)]
use rs_protocol::network::game::server::map_anim::MapAnim;
#[cfg(before_647)]
use rs_protocol::network::game::server::map_projanim::MapProjAnim;
#[cfg(before_647)]
use rs_protocol::network::game::server::obj_add::ObjAdd;
#[cfg(before_647)]
use rs_protocol::network::game::server::obj_count::ObjCount;
#[cfg(before_647)]
use rs_protocol::network::game::server::obj_del::ObjDel;
#[cfg(before_647)]
use rs_protocol::network::game::server::obj_reveal::ObjReveal;
#[cfg(all(since_289, before_647))]
use rs_protocol::network::game::server::sound_area::SoundArea;

/// Rev-647 zone messages are not direct server prots: they are sub-opcode
/// framed payloads multiplexed inside `UPDATE_ZONE_*` packets. The enum,
/// its payload structs, and the `zone_prot` sub-opcode table all live in
/// rs-protocol; re-export the enum so the rest of the engine keeps using
/// `rs_zone::ZoneMessage` uniformly across revisions.
#[cfg(since_647)]
pub use rs_protocol::network::game::server::zone_message::ZoneMessage;

/// Generates the [`ZoneMessage`] enum and its mechanical `encode_zone` /
/// `sizeof_zone` matches from a single variant list.
///
/// Each variant wraps a single newtype payload, and both generated matches
/// dispatch uniformly over the variants, so the only per-variant information is
/// the variant name and its payload type.
#[cfg(before_647)]
macro_rules! zone_messages {
    ($($(#[$meta:meta])* $variant:ident($payload:ty)),+ $(,)?) => {
        /// A protocol message payload representing a zone update to be sent to clients.
        ///
        /// Each variant wraps a specific server protocol message struct. These are
        /// queued as part of [`ZoneEvent`](crate::zone_event::ZoneEvent)s and serialized
        /// into network packets during the zone output phase of each engine tick.
        #[derive(Debug, Clone)]
        pub enum ZoneMessage {
            $($(#[$meta])* $variant($payload),)+
        }

        impl ZoneMessage {
            /// Serializes this zone message into a packet buffer, prefixed with the protocol opcode.
            ///
            /// Writes a 1-byte protocol identifier followed by the message-specific payload.
            /// Used by [`Zone::compute_shared`](crate::zone::Zone::compute_shared) to build
            /// the shared byte buffer for enclosed events.
            ///
            /// # Arguments
            ///
            /// * `buf` -- The packet buffer to write into. Must have sufficient capacity
            ///   (see [`sizeof_zone`](Self::sizeof_zone)).
            ///
            /// # Side Effects
            ///
            /// Advances `buf.pos` by `1 + message.sizeof()` bytes.
            pub fn encode_zone(&self, buf: &mut Packet) {
                match self {
                    $($(#[$meta])* Self::$variant(m) => encode(m, buf),)+
                }
            }

            /// Returns the total serialized size of this zone message in bytes.
            ///
            /// Includes the 1-byte protocol opcode prefix plus the message-specific payload size.
            /// Used to pre-allocate the shared byte buffer in
            /// [`Zone::compute_shared`](crate::zone::Zone::compute_shared).
            ///
            /// # Returns
            ///
            /// The total byte count: `1 + message.sizeof()`.
            pub fn sizeof_zone(&self) -> usize {
                1 + match self {
                    $($(#[$meta])* Self::$variant(m) => m.sizeof(),)+
                }
            }
        }
    };
}

#[cfg(before_647)]
zone_messages! {
    ObjAdd(ObjAdd),
    ObjDel(ObjDel),
    ObjCount(ObjCount),
    ObjReveal(ObjReveal),
    LocAddChange(LocAddChange),
    LocDel(LocDel),
    LocAnim(LocAnim),
    LocMerge(LocMerge),
    MapAnim(MapAnim),
    MapProjAnim(MapProjAnim),
    #[cfg(since_289)]
    SoundArea(SoundArea),
}

/// Writes a protocol opcode byte followed by the encoded message payload into `buf`.
///
/// # Arguments
///
/// * `message` -- The server protocol message to encode.
/// * `buf` -- The packet buffer to write into.
///
/// # Side Effects
///
/// Advances `buf.pos` by `1 + message.sizeof()` bytes.
#[cfg(before_647)]
fn encode<M: ServerProtMessage>(message: &M, buf: &mut Packet) {
    buf.p1(M::PROT as u8);
    message.encode(buf);
}
