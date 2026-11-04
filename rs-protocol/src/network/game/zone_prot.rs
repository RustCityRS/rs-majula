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
}

#[cfg(rev = "647")]
#[repr(u8)]
#[derive(Debug)]
pub enum ZoneProt {
    LocMerge = 0,
    ObjDel = 1,
    SoundArea = 2,
    LocAnim = 3,
    ObjCount = 4,
    LocPrefetch = 5,
    MapAnim = 6,
    ObjReveal = 7,
    LocDel = 8,
    TextCoord = 9,
    SoundVorbisArea = 10,
    ObjAdd = 11,
    MapProjAnimHalfsq = 12,
    LocAddChange = 13,
    MapProjAnim = 14,
}
