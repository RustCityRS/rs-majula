//! `ServerGameZoneMessage` — a `ServerProtMessage` with an associated
//! zone coordinate. Mirrors 2004scape's abstract class at
//! `engine/src/network/game/server/ServerGameZoneMessage.ts`.
//!
//! Zone messages (`LocAddChange`, `ObjAdd`, `MapAnim`, …) carry a
//! `coord` identifying their zone so the world's zone-flush path can
//! encode each message once and slice the resulting bytes into every
//! subscribed player's outgoing buffer.

use crate::network::game::server_prot_message::ServerProtMessage;

/// Trait for zone-scoped outbound messages. Implementors store a
/// `coord` (packed zone position) and are encoded via the paired
/// `ServerGameZoneMessageEncoder`.
pub trait ServerGameZoneMessage: ServerProtMessage {
    /// Packed zone coordinate where this event happened.
    fn coord(&self) -> u32;
}
