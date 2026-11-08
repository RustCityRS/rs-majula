use rs_io::Packet;
use rs_protocol::network::game::server::ServerProtMessage;
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
use rs_protocol::network::game::server::loc_anim::LocAnim;
use rs_protocol::network::game::server::loc_del::LocDel;
#[cfg(before_647)]
use rs_protocol::network::game::server::loc_merge::LocMerge;
#[cfg(since_647)]
use rs_protocol::network::game::server::loc_prefetch::LocPrefetch;
use rs_protocol::network::game::server::map_anim::MapAnim;
use rs_protocol::network::game::server::map_projanim::MapProjAnim;
#[cfg(since_647)]
use rs_protocol::network::game::server::map_projanim_halfsq::MapProjAnimHalfsq;
use rs_protocol::network::game::server::obj_add::ObjAdd;
use rs_protocol::network::game::server::obj_count::ObjCount;
use rs_protocol::network::game::server::obj_del::ObjDel;
use rs_protocol::network::game::server::obj_reveal::ObjReveal;
#[cfg(since_289)]
use rs_protocol::network::game::server::sound_area::SoundArea;
#[cfg(since_647)]
use rs_protocol::network::game::server::sound_vorbis_area::SoundVorbisArea;
#[cfg(since_647)]
use rs_protocol::network::game::server::text_coord::TextCoord;
use rs_protocol::network::game::zone_prot::ZoneProt;

/// Generates the [`ZoneMessage`] enum and its mechanical `encode_zone` /
/// `sizeof_zone` matches from a single variant list.
///
/// Each variant wraps a single newtype payload, and both generated matches
/// dispatch uniformly over the variants, so the only per-variant information is
/// the variant name and its payload type.
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
                    $($(#[$meta])* Self::$variant(m) => {
                        buf.p1(ZoneProt::$variant as u8);
                        m.encode(buf);
                    },)+
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

zone_messages! {
    ObjAdd(ObjAdd),
    ObjDel(ObjDel),
    ObjCount(ObjCount),
    ObjReveal(ObjReveal),
    LocAddChange(LocAddChange),
    LocDel(LocDel),
    LocAnim(LocAnim),
    #[cfg(before_647)]
    LocMerge(LocMerge),
    #[cfg(since_647)]
    LocPrefetch(LocPrefetch),
    MapAnim(MapAnim),
    MapProjAnim(MapProjAnim),
    #[cfg(since_647)]
    MapProjAnimHalfsq(MapProjAnimHalfsq),
    #[cfg(since_289)]
    SoundArea(SoundArea),
    #[cfg(since_647)]
    SoundVorbisArea(SoundVorbisArea),
    #[cfg(since_647)]
    TextCoord(TextCoord),
}
