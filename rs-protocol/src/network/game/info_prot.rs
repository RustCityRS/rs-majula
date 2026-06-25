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

impl PlayerInfoProt {
    #[inline]
    pub const fn to_index(self) -> usize {
        // the ordering here does not matter.
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
}

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

impl NpcInfoProt {
    #[inline]
    pub const fn to_index(self) -> usize {
        // the ordering here does not matter.
        match self {
            NpcInfoProt::Anim => 0,
            NpcInfoProt::FaceEntity => 1,
            NpcInfoProt::Say => 2,
            NpcInfoProt::Damage => 3,
            NpcInfoProt::ChangeType => 4,
            NpcInfoProt::SpotAnim => 5,
            NpcInfoProt::FaceCoord => 6,
            #[cfg(since_244)]
            NpcInfoProt::Damage2 => 7,
        }
    }
}
