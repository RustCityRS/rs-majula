#[cfg(before_647)]
use crate::network::game::server_prot::ServerProt;

#[cfg(before_647)]
#[repr(u8)]
#[derive(Debug)]
pub enum ZoneProt {
    ObjAdd = ServerProt::ObjAdd as u8,
    ObjDel = ServerProt::ObjDel as u8,
    ObjCount = ServerProt::ObjCount as u8,
    ObjReveal = ServerProt::ObjReveal as u8,
    LocAddChange = ServerProt::LocAddChange as u8,
    LocDel = ServerProt::LocDel as u8,
    LocAnim = ServerProt::LocAnim as u8,
    LocMerge = ServerProt::LocMerge as u8,
    MapAnim = ServerProt::MapAnim as u8,
    MapProjAnim = ServerProt::MapProjAnim as u8,
    #[cfg(since_289)]
    SoundArea = ServerProt::SoundArea as u8,
    #[cfg(since_530)]
    MapProjAnimHalfsq = ServerProt::MapProjAnimHalfsq as u8,
    #[cfg(since_530)]
    ServerProt530_104 = ServerProt::ServerProt530_104 as u8,
}

#[cfg(rev = "647")]
#[repr(u8)]
#[derive(Debug)]
pub enum ZoneProt {
    LocMerge = 0,
    ObjDel = 1,
    SoundVorbisArea = 2,
    LocAnim = 3,
    ObjCount = 4,
    LocPrefetch = 5,
    MapAnim = 6,
    ObjReveal = 7,
    LocDel = 8,
    TextCoord = 9,
    SoundArea = 10,
    ObjAdd = 11,
    MapProjAnimHalfsq = 12,
    LocAddChange = 13,
    MapProjAnim = 14,
}

#[cfg(rev = "742")]
#[repr(u8)]
#[derive(Debug)]
pub enum ZoneProt {
    SoundVorbisArea = 0,
    LocDel = 1,
    ObjAdd = 2,
    ObjReveal = 3,
    LocAnim = 4,
    MapProjAnim = 5,
    LocAddChange = 6,
    MapProjAnimHalfsq = 7,
    SoundArea = 8,
    ObjDel = 9,
    LocPrefetch = 10,
    LocMerge = 11,
    MapAnim = 12,
    TextCoord = 13,
    ObjCount = 14,
}
