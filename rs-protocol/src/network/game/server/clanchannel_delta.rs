#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;

#[cfg(since_647)]
const NO_HASH: u8 = 255;

#[cfg(since_647)]
pub enum ClanChannelDeltaOp {
    AddUser {
        display_name: String,
        world: u16,
        rank: i8,
    },
    UpdateUserDetails {
        slot: u16,
        rank: i8,
        world: u16,
        display_name: String,
    },
    DeleteUser {
        slot: u16,
    },
    UpdateClanChannelBaseSettings {
        name: Option<String>,
        rank_talk: i8,
        rank_kick: i8,
    },
}

#[cfg(since_647)]
impl ClanChannelDeltaOp {
    fn encode(&self, buf: &mut Packet) {
        match self {
            Self::AddUser {
                display_name,
                world,
                rank,
            } => {
                buf.p1(1);
                buf.p1(NO_HASH);
                buf.pjstr(display_name, STRING_TERMINATOR);
                buf.p2(*world);
                buf.p1(*rank as u8);
                buf.p8(0);
            }
            Self::UpdateUserDetails {
                slot,
                rank,
                world,
                display_name,
            } => {
                buf.p1(2);
                buf.p2(*slot);
                buf.p1(*rank as u8);
                buf.p2(*world);
                buf.p8(0);
                buf.pjstr(display_name, STRING_TERMINATOR);
            }
            Self::DeleteUser { slot } => {
                buf.p1(3);
                buf.p2(*slot);
                buf.p1(0);
                buf.p1(NO_HASH);
            }
            Self::UpdateClanChannelBaseSettings {
                name,
                rank_talk,
                rank_kick,
            } => {
                buf.p1(4);
                let Some(name) = name else {
                    buf.p1(0);
                    return;
                };
                buf.pjstr(name, STRING_TERMINATOR);
                buf.p1(0);
                buf.p1(*rank_talk as u8);
                buf.p1(*rank_kick as u8);
            }
        }
    }

    fn sizeof(&self) -> usize {
        size_of::<u8>()
            + match self {
                Self::AddUser { display_name, .. } => {
                    1 + Packet::sizejstr(display_name) + 2 + 1 + 8
                }
                Self::UpdateUserDetails { display_name, .. } => {
                    2 + 1 + 2 + 8 + Packet::sizejstr(display_name)
                }
                Self::DeleteUser { .. } => 2 + 1 + 1,
                Self::UpdateClanChannelBaseSettings { name: None, .. } => 1,
                Self::UpdateClanChannelBaseSettings {
                    name: Some(name), ..
                } => Packet::sizejstr(name) + 3,
            }
    }
}

#[cfg(since_647)]
#[server_prot(ClanchannelDelta, Buffered, VarShort)]
pub struct ClanchannelDelta {
    pub affined: bool,
    pub clan_hash: i64,
    pub update_num: i64,
    pub ops: Vec<ClanChannelDeltaOp>,
}

#[cfg(since_647)]
impl ServerProtMessage for ClanchannelDelta {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.affined as u8);
        buf.p8(self.clan_hash);
        buf.p8(self.update_num);
        for op in &self.ops {
            op.encode(buf);
        }
        buf.p1(0);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.affined)
            + size_of_val(&self.clan_hash)
            + size_of_val(&self.update_num)
            + self
                .ops
                .iter()
                .map(ClanChannelDeltaOp::sizeof)
                .sum::<usize>()
            + size_of::<u8>()
    }
}
