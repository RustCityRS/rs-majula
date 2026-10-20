//! What a client holds on to: the last packet of each kind that sets lasting
//! state (an interface component's text, a varp, a stat, a tab, an open
//! interface, the camera...), and the latest chat messages. A keyframe sends
//! them again, in the order they were last sent, so a client starting from
//! it shows what the player's did.
//!
//! The world itself doesn't keep most of this (it sends an interface's text
//! and forgets it), so it's remembered here, from the packets themselves.

use rs_protocol::network::game::server_prot::ServerProt;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::sync::OnceLock;

/// How many chat messages a keyframe brings back.
const MESSAGES: usize = 100;

/// What to keep of a packet.
#[derive(Clone, Copy)]
enum Keep {
    No,
    One,
    Key { group: u8, at: u8, len: u8 },
    Message,
}

fn table() -> &'static [Keep; 256] {
    static TABLE: OnceLock<[Keep; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [Keep::No; 256];
        let key = |prot: u8, at: u8, len: u8| Keep::Key {
            group: prot,
            at,
            len,
        };
        let varps = ServerProt::VarpSmall as u8;
        let mut set = |prot: u8, keep: Keep| table[prot as usize] = keep;
        // An interface component's state, by component.
        for prot in [
            ServerProt::IfSetText as u8,
            ServerProt::IfSetColour as u8,
            ServerProt::IfSetHide as u8,
            ServerProt::IfSetModel as u8,
            ServerProt::IfSetAnim as u8,
            ServerProt::IfSetObject as u8,
            ServerProt::IfSetNpcHead as u8,
            ServerProt::IfSetPlayerHead as u8,
            ServerProt::IfSetPosition as u8,
        ] {
            set(prot, key(prot, 0, 2));
        }
        #[cfg(since_245_2)]
        set(
            ServerProt::IfSetScrollPos as u8,
            key(ServerProt::IfSetScrollPos as u8, 0, 2),
        );
        #[cfg(before_245_2)]
        set(
            ServerProt::IfSetRecol as u8,
            key(ServerProt::IfSetRecol as u8, 0, 2),
        );
        // Sidebar tabs, by tab (after the component).
        set(
            ServerProt::IfSetTab as u8,
            key(ServerProt::IfSetTab as u8, 2, 1),
        );
        set(ServerProt::VarpSmall as u8, key(varps, 0, 2));
        set(ServerProt::VarpLarge as u8, key(varps, 0, 2));
        set(
            ServerProt::UpdateStat as u8,
            key(ServerProt::UpdateStat as u8, 0, 1),
        );
        set(
            ServerProt::UpdateFriendList as u8,
            key(ServerProt::UpdateFriendList as u8, 0, 8),
        );
        #[cfg(since_254)]
        set(
            ServerProt::SetPlayerOp as u8,
            key(ServerProt::SetPlayerOp as u8, 0, 1),
        );
        for prot in [
            ServerProt::IfOpenMain as u8,
            ServerProt::IfOpenSide as u8,
            ServerProt::IfOpenChat as u8,
            ServerProt::IfOpenMainSide as u8,
            ServerProt::IfClose as u8,
            ServerProt::TutOpen as u8,
            ServerProt::TutFlash as u8,
            ServerProt::UpdatePid as u8,
            ServerProt::UpdateRunEnergy as u8,
            ServerProt::UpdateRunWeight as u8,
            ServerProt::ChatFilterSettings as u8,
            ServerProt::UpdateIgnoreList as u8,
            ServerProt::SetMultiway as u8,
            ServerProt::HintArrow as u8,
            ServerProt::MidiSong as u8,
            ServerProt::CamReset as u8,
            ServerProt::CamLookAt as u8,
            ServerProt::CamMoveTo as u8,
        ] {
            set(prot, Keep::One);
        }
        set(ServerProt::IfSetTabActive as u8, Keep::One);
        #[cfg(since_254)]
        set(ServerProt::FriendListLoaded as u8, Keep::One);
        #[cfg(since_274)]
        set(ServerProt::MinimapToggle as u8, Keep::One);
        set(ServerProt::MessageGame as u8, Keep::Message);
        set(ServerProt::MessagePrivate as u8, Keep::Message);
        table
    })
}

#[derive(Default)]
pub struct ClientState {
    seq: u64,
    /// (group, key) to (when last sent, the packet).
    packets: FxHashMap<(u8, u64), (u64, Vec<u8>)>,
    messages: VecDeque<Vec<u8>>,
}

impl ClientState {
    /// A packet the client was sent: `opcode`, then `rest` (a `frame`-byte
    /// length, then the body).
    pub fn remember(&mut self, opcode: u8, frame: usize, rest: &[u8]) {
        let keep = table()[opcode as usize];
        let packet = || {
            let mut packet = Vec::with_capacity(rest.len() + 1);
            packet.push(opcode);
            packet.extend_from_slice(rest);
            packet
        };
        let key = match keep {
            Keep::No => return,
            Keep::Message => {
                if self.messages.len() == MESSAGES {
                    self.messages.pop_front();
                }
                self.messages.push_back(packet());
                return;
            }
            Keep::One => (opcode, 0),
            Keep::Key { group, at, len } => {
                let body = rest.get(frame..).unwrap_or_default();
                let Some(bytes) = body.get(at as usize..(at + len) as usize) else {
                    return;
                };
                (
                    group,
                    bytes.iter().fold(0u64, |k, &b| (k << 8) | u64::from(b)),
                )
            }
        };
        self.seq += 1;
        self.packets.insert(key, (self.seq, packet()));
    }

    /// Everything remembered, in the order it was last sent, then the chat.
    pub fn packets(&self) -> Vec<&[u8]> {
        let mut kept: Vec<&(u64, Vec<u8>)> = self.packets.values().collect();
        kept.sort_unstable_by_key(|(seq, _)| *seq);
        kept.into_iter()
            .map(|(_, packet)| packet.as_slice())
            .chain(self.messages.iter().map(Vec::as_slice))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_last_of_each() {
        let mut state = ClientState::default();
        let text = ServerProt::IfSetText as u8;
        let varp = ServerProt::VarpSmall as u8;
        let varp_large = ServerProt::VarpLarge as u8;
        let pid = ServerProt::UpdatePid as u8;
        // IF_SETTEXT is a var-short: 2 length bytes, then com and text.
        state.remember(text, 2, &[0, 4, 0, 10, b'a', 10]);
        state.remember(varp, 0, &[0, 7, 1]);
        state.remember(text, 2, &[0, 4, 0, 11, b'b', 10]);
        state.remember(pid, 0, &[0, 5]);
        state.remember(text, 2, &[0, 4, 0, 10, b'c', 10]);
        // A large write of the same varp replaces the small one.
        state.remember(varp_large, 0, &[0, 7, 0, 0, 1, 0]);
        let packets = state.packets();
        assert_eq!(
            packets,
            [
                &[text, 0, 4, 0, 11, b'b', 10][..],
                &[pid, 0, 5][..],
                &[text, 0, 4, 0, 10, b'c', 10][..],
                &[varp_large, 0, 7, 0, 0, 1, 0][..],
            ]
        );
    }
}
