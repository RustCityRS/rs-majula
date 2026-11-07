#[cfg(before_647)]
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

#[cfg(since_647)]
#[repr(u16)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum PlayerInfoProt {
    FaceCoord = 0x1,
    Damage = 0x2,
    Appearance = 0x4,
    MoveType = 0x8,
    FaceEntity = 0x10,
    BigInfo = 0x20,
    SpotAnim = 0x40,
    Anim = 0x80,
    Say = 0x200,
    TempMoveType = 0x800,
    Damage2 = 0x1000,
    ExactMove = 0x4000,
    BigInfo2 = 0x8000,
}

impl PlayerInfoProt {
    #[cfg(since_647)]
    pub const fn header(wire: u32) -> ([u8; 3], usize) {
        mask_header(wire, Self::BigInfo as u32, Self::BigInfo2 as u32)
    }

    /// Combines the engine's two damage slots into the 647 hit block.
    #[cfg(since_647)]
    pub const fn mask(masks: u16) -> u32 {
        let wire = masks & !(Self::Damage2 as u16);
        if masks & Self::Damage2 as u16 != 0 {
            (wire | Self::Damage as u16) as u32
        } else {
            wire as u32
        }
    }

    #[cfg(before_647)]
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
            #[cfg(since_244)]
            PlayerInfoProt::Damage2 => 8,
            PlayerInfoProt::BigInfo => 255,   // unused
            PlayerInfoProt::ExactMove => 255, // unused
        }
    }

    #[cfg(since_647)]
    #[inline]
    pub const fn to_index(self) -> usize {
        match self {
            Self::FaceCoord => 0,
            Self::TempMoveType => 1,
            Self::Anim => 2,
            Self::MoveType => 3,
            Self::ExactMove => 4,
            Self::Damage | Self::Damage2 => 5,
            Self::FaceEntity => 6,
            Self::SpotAnim => 7,
            Self::Say => 8,
            Self::Appearance => 9,
            Self::BigInfo | Self::BigInfo2 => 255,
        }
    }
}

#[cfg(before_647)]
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

#[cfg(since_647)]
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

impl NpcInfoProt {
    #[cfg(since_647)]
    pub const fn header(wire: u32) -> ([u8; 3], usize) {
        mask_header(wire, Self::BigInfo as u32, Self::BigInfo2 as u32)
    }

    /// Combines the engine's two damage slots into the 647 hit block.
    #[cfg(since_647)]
    pub const fn mask(masks: u16) -> u32 {
        let wire = masks & !(Self::Damage2 as u16);
        if masks & Self::Damage2 as u16 != 0 {
            (wire | Self::Damage as u16) as u32
        } else {
            wire as u32
        }
    }

    #[cfg(before_647)]
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

    #[cfg(since_647)]
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
    fn extended_masks_match_the_client_readers() {
        for (wire, player, npc) in [
            (0x80, ([0x80, 0, 0], 1), ([0x80, 0, 0], 1)),
            (0x4000, ([0x20, 0x40, 0], 2), ([0x8, 0x40, 0], 2)),
            (0x10000, ([0x20, 0x80, 1], 3), ([0x8, 1, 1], 3)),
        ] {
            assert_eq!(PlayerInfoProt::header(wire), player);
            assert_eq!(NpcInfoProt::header(wire), npc);
        }
    }
}
