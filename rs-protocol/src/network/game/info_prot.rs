#[cfg(before_530)]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum PlayerInfoProt {
    Appearance = 0x1,
    Anim = 0x2,
    FaceEntity = 0x4,
    Say = 0x8,
    Damage = 0x10,
    FaceCoord = 0x20,
    Chat = 0x40,
    BigInfo = 0x80,
    SpotAnim = 0x100,
    ExactMove = 0x200,
    #[cfg(since_244)]
    Damage2 = 0x400,
}

#[cfg(rev = "530")]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum PlayerInfoProt {
    Damage = 0x1,
    FaceEntity = 0x2,
    Appearance = 0x4,
    Anim = 0x8,
    BigInfo = 0x10,
    Say = 0x20,
    FaceCoord = 0x40,
    Chat = 0x80,
    SpotAnim = 0x100,
    Damage2 = 0x200,
    ExactMove = 0x400,
    ModelAnims = 0x800,
}

#[cfg(all(since_647, before_742))]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum PlayerInfoProt {
    FaceCoord = 0x1,
    Damage = 0x2,
    Appearance = 0x4,
    MoveSpeed = 0x8,
    FaceEntity = 0x10,
    BigInfo = 0x20,
    SpotAnim = 0x40,
    Anim = 0x80,
    Say = 0x200,
    Chat = 0x400,
    TempMoveSpeed = 0x800,
    Damage2 = 0x1000,
    ExactMove = 0x4000,
    BigInfo2 = 0x8000,
}

#[cfg(since_742)]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum PlayerInfoProt {
    SpotAnim = 0x1,
    FaceCoord = 0x2,
    Damage = 0x4,
    BigInfo = 0x8,
    FaceEntity = 0x10,
    Appearance = 0x20,
    Anim = 0x40,
    MoveSpeed = 0x80,
    ExactMove = 0x200,
    Say = 0x400,
    Chat = 0x800,
    Damage2 = 0x1000,
    BigInfo2 = 0x2000,
    TempMoveSpeed = 0x8000,
}

impl PlayerInfoProt {
    /// How many blocks [`to_index`](Self::to_index) orders.
    #[cfg(rev = "225")]
    pub const COUNT: usize = 9;
    #[cfg(all(since_244, before_530))]
    pub const COUNT: usize = 10;
    #[cfg(all(since_530, before_647))]
    pub const COUNT: usize = 11;
    #[cfg(since_647)]
    pub const COUNT: usize = 10;

    /// The mask header for the blocks in `masks`, and how many of its bytes
    /// are sent.
    #[cfg(before_647)]
    pub const fn header(masks: u16) -> ([u8; 3], usize) {
        short_header(masks, Self::BigInfo as u16)
    }

    #[cfg(since_647)]
    pub const fn header(masks: u16) -> ([u8; 3], usize) {
        mask_header(
            Self::mask(masks),
            Self::BigInfo as u32,
            Self::BigInfo2 as u32,
        )
    }

    /// The blocks in `masks` as the 647 client reads them: the engine's two
    /// damage slots are one hit block, and chat goes out as `MESSAGE_PUBLIC`.
    #[cfg(since_647)]
    const fn mask(masks: u16) -> u32 {
        let wire = masks & !(Self::Damage2 as u16 | Self::Chat as u16);
        if masks & Self::Damage2 as u16 != 0 {
            (wire | Self::Damage as u16) as u32
        } else {
            wire as u32
        }
    }

    /// Where the block sits in the order the client reads them.
    #[cfg(before_530)]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            PlayerInfoProt::Appearance => 0,
            PlayerInfoProt::Anim => 1,
            PlayerInfoProt::FaceEntity => 2,
            PlayerInfoProt::Say => 3,
            PlayerInfoProt::Damage => 4,
            PlayerInfoProt::FaceCoord => 5,
            PlayerInfoProt::Chat => 6,
            PlayerInfoProt::SpotAnim => 7,
            PlayerInfoProt::ExactMove => 8,
            #[cfg(since_244)]
            PlayerInfoProt::Damage2 => 9,
            PlayerInfoProt::BigInfo => 255, // unused
        }
    }

    #[cfg(all(since_530, before_647))]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            Self::Chat => 0,
            Self::Damage => 1,
            Self::Anim => 2,
            Self::Appearance => 3,
            Self::FaceEntity => 4,
            Self::ExactMove => 5,
            Self::Say => 6,
            Self::Damage2 => 7,
            Self::ModelAnims => 8,
            Self::SpotAnim => 9,
            Self::FaceCoord => 10,
            Self::BigInfo => 255,
        }
    }

    #[cfg(all(since_647, before_742))]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            Self::FaceCoord => 0,
            Self::TempMoveSpeed => 1,
            Self::Anim => 2,
            Self::MoveSpeed => 3,
            Self::ExactMove => 4,
            Self::Damage | Self::Damage2 => 5,
            Self::FaceEntity => 6,
            Self::SpotAnim => 7,
            Self::Say => 8,
            Self::Appearance => 9,
            Self::BigInfo | Self::BigInfo2 | Self::Chat => 255,
        }
    }

    #[cfg(since_742)]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            Self::MoveSpeed => 0,
            Self::Damage | Self::Damage2 => 1,
            Self::Anim => 2,
            Self::TempMoveSpeed => 3,
            Self::Say => 4,
            Self::FaceCoord => 5,
            Self::FaceEntity => 6,
            Self::Appearance => 7,
            Self::SpotAnim => 8,
            Self::ExactMove => 9,
            Self::BigInfo | Self::BigInfo2 | Self::Chat => 255,
        }
    }
}

#[cfg(before_530)]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum NpcInfoProt {
    #[cfg(since_244)]
    Damage2 = 0x1,
    Anim = 0x2,
    FaceEntity = 0x4,
    Say = 0x8,
    Damage = 0x10,
    ChangeType = 0x20,
    SpotAnim = 0x40,
    FaceCoord = 0x80,
}

#[cfg(rev = "530")]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum NpcInfoProt {
    ChangeType = 0x1,
    Damage2 = 0x2,
    FaceEntity = 0x4,
    BigInfo = 0x8,
    Anim = 0x10,
    Say = 0x20,
    Damage = 0x40,
    SpotAnim = 0x80,
    ModelAnims = 0x100,
    FaceCoord = 0x200,
}

#[cfg(all(since_647, before_742))]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum NpcInfoProt {
    Damage = 0x1,
    ChangeType = 0x2,
    FaceCoord = 0x4,
    BigInfo = 0x8,
    FaceEntity = 0x10,
    SpotAnim = 0x20,
    Say = 0x40,
    Anim = 0x80,
    BigInfo2 = 0x100,
    Damage2 = 0x200,
}

#[cfg(since_742)]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum NpcInfoProt {
    Anim = 0x1,
    FaceCoord = 0x2,
    BigInfo = 0x4,
    Damage = 0x8,
    Say = 0x10,
    FaceEntity = 0x20,
    SpotAnim = 0x40,
    ChangeType = 0x80,
    Damage2 = 0x200,
    BigInfo2 = 0x400,
}

impl NpcInfoProt {
    /// How many blocks [`to_index`](Self::to_index) orders.
    #[cfg(rev = "225")]
    pub const COUNT: usize = 7;
    #[cfg(all(since_244, before_530))]
    pub const COUNT: usize = 8;
    #[cfg(all(since_530, before_647))]
    pub const COUNT: usize = 9;
    #[cfg(since_647)]
    pub const COUNT: usize = 7;

    /// The mask header for the blocks in `masks`, and how many of its bytes
    /// are sent.
    #[cfg(before_530)]
    pub const fn header(masks: u16) -> ([u8; 3], usize) {
        ([masks as u8, 0, 0], 1)
    }

    #[cfg(all(since_530, before_647))]
    pub const fn header(masks: u16) -> ([u8; 3], usize) {
        short_header(masks, Self::BigInfo as u16)
    }

    #[cfg(since_647)]
    pub const fn header(masks: u16) -> ([u8; 3], usize) {
        mask_header(
            Self::mask(masks),
            Self::BigInfo as u32,
            Self::BigInfo2 as u32,
        )
    }

    /// Combines the engine's two damage slots into the 647 hit block.
    #[cfg(since_647)]
    const fn mask(masks: u16) -> u32 {
        let wire = masks & !(Self::Damage2 as u16);
        if masks & Self::Damage2 as u16 != 0 {
            (wire | Self::Damage as u16) as u32
        } else {
            wire as u32
        }
    }

    /// Where the block sits in the order the client reads them.
    #[cfg(before_530)]
    #[inline]
    pub const fn to_index(self) -> usize {
        let offset = cfg!(since_244) as usize;
        match self {
            NpcInfoProt::Anim => offset,
            NpcInfoProt::FaceEntity => offset + 1,
            NpcInfoProt::Say => offset + 2,
            NpcInfoProt::Damage => offset + 3,
            NpcInfoProt::ChangeType => offset + 4,
            NpcInfoProt::SpotAnim => offset + 5,
            NpcInfoProt::FaceCoord => offset + 6,
            #[cfg(since_244)]
            NpcInfoProt::Damage2 => 0,
        }
    }

    #[cfg(all(since_530, before_647))]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            Self::Damage => 0,
            Self::Damage2 => 1,
            Self::Anim => 2,
            Self::FaceEntity => 3,
            Self::SpotAnim => 4,
            Self::ChangeType => 5,
            Self::Say => 6,
            Self::ModelAnims => 7,
            Self::FaceCoord => 8,
            Self::BigInfo => 255,
        }
    }

    #[cfg(all(since_647, before_742))]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            Self::ChangeType => 0,
            Self::FaceCoord => 1,
            Self::SpotAnim => 2,
            Self::FaceEntity => 3,
            Self::Say => 4,
            Self::Damage | Self::Damage2 => 5,
            Self::Anim => 6,
            Self::BigInfo | Self::BigInfo2 => 255,
        }
    }

    #[cfg(since_742)]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            Self::SpotAnim => 0,
            Self::Say => 1,
            Self::Anim => 2,
            Self::Damage | Self::Damage2 => 3,
            Self::FaceCoord => 4,
            Self::ChangeType => 5,
            Self::FaceEntity => 6,
            Self::BigInfo | Self::BigInfo2 => 255,
        }
    }
}

/// One mask byte, or two with `big` set in the first.
#[cfg(before_647)]
const fn short_header(masks: u16, big: u16) -> ([u8; 3], usize) {
    if masks > 0xff {
        let wire = masks | big;
        ([wire as u8, (wire >> 8) as u8, 0], 2)
    } else {
        ([masks as u8, 0, 0], 1)
    }
}

#[cfg(since_647)]
const fn mask_header(mut wire: u32, big: u32, big2: u32) -> ([u8; 3], usize) {
    let len = if wire >> 16 != 0 {
        wire |= big | big2;
        3
    } else if wire >> 8 != 0 {
        wire |= big;
        2
    } else {
        1
    };
    ([wire as u8, (wire >> 8) as u8, (wire >> 16) as u8], len)
}

#[cfg(all(test, since_647))]
mod tests {
    use super::*;

    #[test]
    #[cfg(before_742)]
    fn extended_masks_match_the_client_readers() {
        for (wire, player, npc) in [
            (0x80, ([0x80, 0, 0], 1), ([0x80, 0, 0], 1)),
            (0x4000, ([0x20, 0x40, 0], 2), ([0x8, 0x40, 0], 2)),
            (0x10000, ([0x20, 0x80, 1], 3), ([0x8, 1, 1], 3)),
        ] {
            let big = (PlayerInfoProt::BigInfo, PlayerInfoProt::BigInfo2);
            assert_eq!(mask_header(wire, big.0 as u32, big.1 as u32), player);
            let big = (NpcInfoProt::BigInfo, NpcInfoProt::BigInfo2);
            assert_eq!(mask_header(wire, big.0 as u32, big.1 as u32), npc);
        }
        assert_eq!(
            PlayerInfoProt::header(PlayerInfoProt::Damage2 as u16),
            ([0x2, 0, 0], 1),
            "the second hit goes in the hit block"
        );
    }

    #[test]
    #[cfg(since_742)]
    fn extended_masks_match_the_742_client_readers() {
        for (wire, player, npc) in [
            (0x80, ([0x80, 0, 0], 1), ([0x80, 0, 0], 1)),
            (0x8000, ([0x8, 0x80, 0], 2), ([0x4, 0x80, 0], 2)),
            (0x10000, ([0x8, 0x20, 1], 3), ([0x4, 0x4, 1], 3)),
        ] {
            let big = (PlayerInfoProt::BigInfo, PlayerInfoProt::BigInfo2);
            assert_eq!(mask_header(wire, big.0 as u32, big.1 as u32), player);
            let big = (NpcInfoProt::BigInfo, NpcInfoProt::BigInfo2);
            assert_eq!(mask_header(wire, big.0 as u32, big.1 as u32), npc);
        }
        assert_eq!(
            PlayerInfoProt::header(PlayerInfoProt::Damage2 as u16 | PlayerInfoProt::Chat as u16),
            ([0x4, 0, 0], 1),
            "the second hit goes in the hit block, chat in MESSAGE_PUBLIC"
        );
        assert_eq!(
            NpcInfoProt::header(NpcInfoProt::Damage2 as u16),
            ([0x8, 0, 0], 1),
            "the second hit goes in the hit block"
        );
    }
}
