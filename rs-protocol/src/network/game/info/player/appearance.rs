use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;

/// The wear positions an appearance lists.
pub const WEAR_SLOTS: usize = 12;

/// A wear position showing nothing.
const EMPTY: u8 = 0;

/// Added to a body kit id.
const KIT: u16 = 0x100;

/// Added to the obj (before 647) or wear model (647) shown in a slot.
#[cfg(before_647)]
const OBJ: u16 = 0x200;
#[cfg(since_647)]
const OBJ: u16 = 0x8000;

/// In the first slot: the player looks like the npc type that follows.
#[cfg(since_254)]
const NPC: u16 = u16::MAX;

/// After the slots: which of them carry recolours or retextures.
#[cfg(since_647)]
const NO_RECOLOURS: u16 = 0;

/// After a transmog npc: the team cape the player wears.
#[cfg(since_647)]
const NO_TEAM: u8 = 0;

/// The first byte's flags (`PlayerEntity.method4076`).
#[cfg(since_647)]
const FEMALE: u8 = 0x1;
#[cfg(since_647)]
const SKILL_LEVEL: u8 = 0x4;

/// The byte after the flags, which the client reads and never uses.
#[cfg(since_647)]
const UNUSED: u8 = u8::MAX;

/// No `headicons_pk` or `headicons_prayer` icon.
#[cfg(since_647)]
const NO_ICON: u8 = u8::MAX;

/// The headicon bit scripts set for a skull (`headicon.constant`), and the
/// `headicons_pk` sprite 2011 draws for it.
#[cfg(since_647)]
const HEADICON_SKULL: u8 = 1 << 0;
#[cfg(since_647)]
const PK_SKULL: u8 = 0;

/// The headicon bits scripts set (`headicon.constant`) that 2011 draws as
/// prayer icons, and their `headicons_prayer` sprites.
#[cfg(since_647)]
const HEADICON_PROTECT_MELEE: u8 = 1 << 3;
#[cfg(since_647)]
const HEADICON_PROTECT_RANGE: u8 = 1 << 4;
#[cfg(since_647)]
const HEADICON_PROTECT_MAGIC: u8 = 1 << 5;
#[cfg(since_647)]
const PRAYER_PROTECT_MELEE: u8 = 0;
#[cfg(since_647)]
const PRAYER_PROTECT_RANGE: u8 = 1;
#[cfg(since_647)]
const PRAYER_PROTECT_MAGIC: u8 = 2;

/// Not hidden from other players' minimaps and menus.
#[cfg(since_647)]
const VISIBLE: u8 = 0;

/// Without a skill level: no summoning on top of the combat level, and no
/// PvP level range.
#[cfg(since_647)]
const NO_SUMMONING_LEVEL: u8 = 0;
#[cfg(since_647)]
const NO_PVP_RANGE: u8 = u8::MAX;

/// No idle, crawl, walk and run ambient sounds.
#[cfg(since_647)]
const NO_SOUNDS: u8 = 0;

pub struct PlayerInfoAppearance<'a> {
    pub bytes: &'a [u8],
}

impl InfoMessage for PlayerInfoAppearance<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        #[cfg(before_647)]
        buf.p1(self.bytes.len() as u8);
        #[cfg(since_647)]
        buf.p1_alt3(self.bytes.len() as u8);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        1 + self.bytes.len()
    }
}

#[derive(Clone, Copy)]
pub enum Wear {
    None,
    Kit(u16),
    Obj(u16),
}

pub enum Look {
    Worn([Wear; WEAR_SLOTS]),
    #[cfg(since_254)]
    Npc(u16),
}

pub struct Appearance<'a> {
    pub gender: u8,
    pub headicons: u8,
    pub look: Look,
    pub colours: &'a [u8; 5],
    #[cfg(before_647)]
    pub anims: [u16; 7],
    #[cfg(before_647)]
    pub username37: u64,
    #[cfg(since_647)]
    pub bas: u16,
    #[cfg(since_647)]
    pub name: &'a str,
    pub combat_level: u8,
    #[cfg(since_274)]
    pub skill_level: u16,
}

impl Appearance<'_> {
    /// The appearance block's body, which the engine encodes once per change.
    #[cfg(before_647)]
    pub fn encode(&self, buf: &mut Packet) {
        buf.p1(self.gender);
        buf.p1(self.headicons);
        self.look.encode(buf);
        for &colour in self.colours {
            buf.p1(colour);
        }
        for anim in self.anims {
            buf.p2(anim);
        }
        buf.p8(self.username37 as i64);
        buf.p1(self.combat_level);
        #[cfg(since_274)]
        buf.p2(self.skill_level);
    }

    #[cfg(since_647)]
    pub fn encode(&self, buf: &mut Packet) {
        let skill_level = if self.skill_level != 0 {
            SKILL_LEVEL
        } else {
            0
        };
        buf.p1((self.gender & FEMALE) | skill_level);
        buf.p1(UNUSED);
        buf.p1(self.skull());
        buf.p1(self.prayer());
        buf.p1(VISIBLE);
        self.look.encode(buf);
        for &colour in self.colours {
            buf.p1(colour);
        }
        buf.p2(self.bas);
        buf.pjstr(self.name, STRING_TERMINATOR);
        buf.p1(self.combat_level);
        if self.skill_level != 0 {
            buf.p2(self.skill_level);
        } else {
            buf.p1(NO_SUMMONING_LEVEL);
            buf.p1(NO_PVP_RANGE);
        }
        buf.p1(NO_SOUNDS);
    }

    /// The skull the 2011 client draws for the headicon bits.
    #[cfg(since_647)]
    fn skull(&self) -> u8 {
        if self.headicons & HEADICON_SKULL != 0 {
            PK_SKULL
        } else {
            NO_ICON
        }
    }

    /// The prayer icon the 2011 client draws for the headicon bits.
    #[cfg(since_647)]
    fn prayer(&self) -> u8 {
        if self.headicons & HEADICON_PROTECT_MAGIC != 0 {
            PRAYER_PROTECT_MAGIC
        } else if self.headicons & HEADICON_PROTECT_RANGE != 0 {
            PRAYER_PROTECT_RANGE
        } else if self.headicons & HEADICON_PROTECT_MELEE != 0 {
            PRAYER_PROTECT_MELEE
        } else {
            NO_ICON
        }
    }
}

impl Look {
    fn encode(&self, buf: &mut Packet) {
        match self {
            Look::Worn(slots) => {
                for slot in slots {
                    match *slot {
                        Wear::None => buf.p1(EMPTY),
                        Wear::Kit(kit) => buf.p2(KIT + kit),
                        Wear::Obj(obj) => buf.p2(OBJ + obj),
                    }
                }
                #[cfg(since_647)]
                buf.p2(NO_RECOLOURS);
            }
            #[cfg(since_254)]
            Look::Npc(npc) => {
                buf.p2(NPC);
                buf.p2(*npc);
                #[cfg(since_647)]
                buf.p1(NO_TEAM);
            }
        }
    }
}
