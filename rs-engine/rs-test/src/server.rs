//! Server messages read the way each rev's client reads them: the inverse of
//! the rev's encoder in rs-protocol. 530's interface packets carry a verify
//! short, which is skipped.

use rs_io::Packet;
use rs_protocol::network::game::server_prot::ServerProt;
use rs_ver::STRING_TERMINATOR;

/// The payloads of every `prot` among `packets`, in order.
pub fn of(packets: &[(u16, Vec<u8>)], prot: ServerProt) -> impl Iterator<Item = &[u8]> {
    let opcode = prot as u16;
    packets
        .iter()
        .filter(move |(op, _)| *op == opcode)
        .map(|(_, p)| p.as_slice())
}

fn read<T>(packets: &[(u16, Vec<u8>)], prot: ServerProt, f: impl Fn(&mut Packet) -> T) -> Vec<T> {
    of(packets, prot)
        .map(|p| f(&mut Packet::from(p.to_vec())))
        .collect()
}

/// Every IF_SETTEXT as (com, text).
pub fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, String)> {
    read(packets, ServerProt::IfSetText, |b| {
        #[cfg(all(since_530, before_647))]
        let com = b.g4_alt3();
        #[cfg(since_647)]
        let com = b.g4s();
        (com, b.gjstr(STRING_TERMINATOR))
    })
}

/// Every IF_SETHIDE as (com, hidden).
pub fn hides(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool)> {
    read(packets, ServerProt::IfSetHide, |b| {
        #[cfg(all(since_530, before_647))]
        let (hidden, _, com) = (b.g1_alt2(), b.g2(), b.g4_alt1());
        #[cfg(all(since_647, before_742))]
        let (com, hidden) = (b.g4_alt3(), b.g1_alt2());
        #[cfg(since_742)]
        let (hidden, com) = (b.g1_alt1(), b.g4_alt2());
        (com, hidden != 0)
    })
}

/// Every IF_SETEVENTS as (com, first slot, last slot, events).
pub fn events(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, u16, u16, i32)> {
    read(packets, ServerProt::IfSetEvents, |b| {
        #[cfg(all(since_530, before_647))]
        let (_, to, com, from, events) =
            (b.g2_alt1(), b.g2_alt1(), b.g4s(), b.g2_alt2(), b.g4_alt2());
        #[cfg(all(since_647, before_742))]
        let (from, events, com, to) = (b.g2_alt2(), b.g4_alt3(), b.g4_alt3(), b.g2_alt3());
        #[cfg(since_742)]
        let (from, to, com, events) = (b.g2_alt3(), b.g2_alt1(), b.g4s(), b.g4s());
        (com, from, to, events)
    })
}

/// Every IF_SETOBJECT as (com, obj).
pub fn objects(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, i32)> {
    read(packets, ServerProt::IfSetObject, |b| {
        #[cfg(all(since_530, before_647))]
        let (_, com, obj) = (b.g4s(), b.g4_alt3(), b.g2_alt3());
        #[cfg(all(since_647, before_742))]
        let (com, _, obj) = (b.g4_alt1(), b.g4_alt3(), b.g2());
        #[cfg(since_742)]
        let (_, obj, com) = (b.g4s(), b.g2_alt1(), b.g4_alt3());
        (com, obj as i32)
    })
}

/// Every IF_SETMODEL as (com, model).
pub fn models(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, i32)> {
    read(packets, ServerProt::IfSetModel, |b| {
        #[cfg(all(since_530, before_647))]
        let (com, _, model) = (b.g4_alt1(), b.g2_alt3(), b.g2_alt2() as i32);
        #[cfg(all(since_647, before_742))]
        let (model, com) = (b.g2_alt3() as i32, b.g4s());
        #[cfg(since_742)]
        let (model, com) = (b.g4_alt1(), b.g4_alt1());
        (com, model)
    })
}

/// Every IF_SETANIM as (com, seq).
pub fn anims(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, i32)> {
    read(packets, ServerProt::IfSetAnim, |b| {
        #[cfg(all(since_530, before_647))]
        let (com, seq) = (b.g4_alt3(), b.g2_alt1() as i32);
        #[cfg(all(since_647, before_742))]
        let (com, seq) = (b.g4_alt1(), b.g2_alt1() as i32);
        #[cfg(since_742)]
        let (com, seq) = (b.g4_alt1(), b.g4s());
        (com, seq)
    })
}

/// Every IF_SETGRAPHIC as (com, graphic).
#[cfg(since_647)]
pub fn graphics(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, i32)> {
    read(packets, ServerProt::IfSetGraphic, |b| {
        #[cfg(before_742)]
        let (com, graphic) = (b.g4_alt1(), b.g2_alt2() as i32);
        #[cfg(since_742)]
        let (graphic, com) = (b.g4_alt2(), b.g4s());
        (com, graphic)
    })
}

/// Every IF_SETPOSITION as (com, x, y).
pub fn positions(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, i16, i16)> {
    read(packets, ServerProt::IfSetPosition, |b| {
        #[cfg(all(since_530, before_647))]
        let (_, com, x, y) = (b.g2_alt2(), b.g4_alt1(), b.g2(), b.g2_alt2());
        #[cfg(all(since_647, before_742))]
        let (com, x, y) = (b.g4_alt3(), b.g2_alt3(), b.g2_alt3());
        #[cfg(since_742)]
        let (com, y, x) = (b.g4s(), b.g2_alt3(), b.g2_alt1());
        (com, x as i16, y as i16)
    })
}

/// Every IF_OPENTOP as (interface, mode).
pub fn tops(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, u8)> {
    read(packets, ServerProt::IfOpentop, |b| {
        #[cfg(all(since_530, before_647))]
        let (top, mode) = (b.g2_alt3(), b.g1_alt1());
        #[cfg(all(since_647, before_742))]
        let (top, mode) = (b.g2_alt1(), b.g1_alt1());
        #[cfg(since_742)]
        let (mode, _, top) = (b.g1(), b.g4_alt1(), b.g2_alt1());
        (top, mode)
    })
}

/// Every IF_OPENSUB as (interface, com).
pub fn subs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    read(packets, ServerProt::IfOpensub, |b| {
        #[cfg(all(since_530, before_647))]
        let (_, com, _, sub) = (b.g1(), b.g4_alt3(), b.g2_alt2(), b.g2());
        #[cfg(all(since_647, before_742))]
        let (sub, com) = (b.g2_alt3(), b.g4_alt3());
        #[cfg(since_742)]
        let (_, _, com, _, sub) = (b.g4s(), b.g4_alt3(), b.g4_alt2(), b.g1_alt3(), b.g2());
        (sub, com)
    })
}

/// Every IF_CLOSESUB's parent com.
pub fn closes(packets: &[(u16, Vec<u8>)]) -> Vec<i32> {
    read(packets, ServerProt::IfClosesub, |b| {
        #[cfg(all(since_530, before_647))]
        let (_, com) = (b.g2(), b.g4s());
        #[cfg(all(since_647, before_742))]
        let (com, _) = (b.g4_alt3(), ());
        #[cfg(since_742)]
        let (com, _) = (b.g4_alt2(), ());
        com
    })
}

/// A RUNCLIENTSCRIPT argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    Int(i32),
    Str(String),
}

/// Every RUNCLIENTSCRIPT as (script, args in the script's order).
pub fn scripts(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, Vec<Arg>)> {
    read(packets, ServerProt::RunClientScript, |b| {
        #[cfg(all(since_530, before_647))]
        b.g2();
        let types = b.gjstr(STRING_TERMINATOR);
        let mut args: Vec<Arg> = types
            .chars()
            .rev()
            .map(|t| match t {
                's' => Arg::Str(b.gjstr(STRING_TERMINATOR)),
                _ => Arg::Int(b.g4s()),
            })
            .collect();
        args.reverse();
        (b.g4s(), args)
    })
}

/// Every VARC_SMALL and VARC_LARGE as (varc, value), in the order sent.
pub fn varcs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    let small = ServerProt::VarcSmall as u16;
    let large = ServerProt::VarcLarge as u16;
    packets
        .iter()
        .filter_map(|(op, p)| {
            let b = &mut Packet::from(p.clone());
            match *op {
                op if op == small => {
                    #[cfg(all(since_530, before_647))]
                    let (_, val, id) = (b.g2_alt1(), b.g1_alt2() as i32, b.g2_alt3());
                    #[cfg(all(since_647, before_742))]
                    let (val, id) = (b.g1_alt2() as i8 as i32, b.g2_alt1());
                    #[cfg(since_742)]
                    let (val, id) = (b.g1_alt3() as i8 as i32, b.g2_alt3());
                    Some((id, val))
                }
                op if op == large => {
                    #[cfg(all(since_530, before_647))]
                    let (_, val, id) = (b.g2_alt3(), b.g4s(), b.g2_alt2());
                    #[cfg(all(since_647, before_742))]
                    let (val, id) = (b.g4_alt1(), b.g2_alt3());
                    #[cfg(since_742)]
                    let (val, id) = (b.g4s(), b.g2_alt2());
                    Some((id, val))
                }
                _ => None,
            }
        })
        .collect()
}

/// Every VARP_SMALL and VARP_LARGE as (varp, value), in the order sent.
pub fn varps(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, i32)> {
    let small = ServerProt::VarpSmall as u16;
    let large = ServerProt::VarpLarge as u16;
    packets
        .iter()
        .filter_map(|(op, p)| {
            let b = &mut Packet::from(p.clone());
            match *op {
                op if op == small => {
                    #[cfg(all(since_530, before_647))]
                    let (id, val) = (b.g2_alt2(), b.g1_alt2());
                    #[cfg(all(since_647, before_742))]
                    let (val, id) = (b.g1_alt3(), b.g2());
                    #[cfg(since_742)]
                    let (val, id) = (b.g1_alt1(), b.g2_alt1());
                    Some((id, val as i8 as i32))
                }
                op if op == large => {
                    #[cfg(all(since_530, before_647))]
                    let (val, id) = (b.g4s(), b.g2_alt2());
                    #[cfg(all(since_647, before_742))]
                    let (id, val) = (b.g2(), b.g4_alt2());
                    #[cfg(since_742)]
                    let (val, id) = (b.g4_alt1(), b.g2_alt2());
                    Some((id, val))
                }
                _ => None,
            }
        })
        .collect()
}

/// Every VARCSTR_SMALL as (varc, value).
pub fn varcstrs(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, String)> {
    read(packets, ServerProt::VarcStrSmall, |b| {
        #[cfg(all(since_530, before_647))]
        let (id, _, val) = (b.g2_alt1(), b.g2_alt2(), b.gjstr(STRING_TERMINATOR));
        #[cfg(all(since_647, before_742))]
        let (val, id) = (b.gjstr(STRING_TERMINATOR), b.g2_alt3());
        #[cfg(since_742)]
        let (id, val) = (b.g2(), b.gjstr(STRING_TERMINATOR));
        (id, val)
    })
}

/// An UPDATE_INV_FULL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inv {
    pub inv: i32,
    pub other: bool,
    /// Each slot's (obj, count).
    pub slots: Vec<Option<(i32, i32)>>,
}

/// Every UPDATE_INV_FULL.
pub fn invs(packets: &[(u16, Vec<u8>)]) -> Vec<Inv> {
    read(packets, ServerProt::UpdateInvFull, |b| {
        #[cfg(all(since_530, before_647))]
        let (other, inv, capacity) = (b.g4s() == -70001, b.g2(), b.g2());
        #[cfg(since_647)]
        let (inv, other, capacity) = (b.g2(), b.g1() != 0, b.g2());
        let slots = (0..capacity)
            .map(|_| {
                #[cfg(all(since_530, before_647))]
                let (count, obj) = {
                    let count = match b.g1_alt3() {
                        255 => b.g4s(),
                        n => n as i32,
                    };
                    (count, b.g2())
                };
                #[cfg(all(since_647, before_742))]
                let (count, obj) = {
                    let count = match b.g1() {
                        255 => b.g4_alt1(),
                        n => n as i32,
                    };
                    let hi = b.g1() as u16;
                    (count, (hi << 8) | b.g1_alt1() as u16)
                };
                #[cfg(since_742)]
                let (count, obj) = {
                    let count = match b.g1() {
                        255 => b.g4s(),
                        n => n as i32,
                    };
                    (count, b.g2_alt3())
                };
                (obj != 0).then(|| (obj as i32 - 1, count))
            })
            .collect();
        Inv {
            inv: inv as i32,
            other,
            slots,
        }
    })
}

/// Every UPDATE_INV_PARTIAL as (inv, other, [(slot, (obj, count))]).
#[allow(clippy::type_complexity)]
pub fn inv_changes(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, bool, Vec<(i32, Option<(i32, i32)>)>)> {
    of(packets, ServerProt::UpdateInvPartial)
        .map(|p| {
            let b = &mut Packet::from(p.to_vec());
            #[cfg(all(since_530, before_647))]
            let (other, inv) = (b.g4s() == -70001, b.g2());
            #[cfg(since_647)]
            let (inv, other) = (b.g2(), b.g1() != 0);
            let mut changes = Vec::new();
            while b.pos < p.len() {
                let slot = b.gsmart1or2();
                let obj = b.g2();
                let change = (obj != 0).then(|| {
                    let count = match b.g1() {
                        255 => b.g4s(),
                        n => n as i32,
                    };
                    (obj as i32 - 1, count)
                });
                changes.push((slot, change));
            }
            (inv as i32, other, changes)
        })
        .collect()
}

/// Every MESSAGE_GAME as (type, the name it is from, text). 530's carry only
/// text, as type 0.
pub fn messages(packets: &[(u16, Vec<u8>)]) -> Vec<(i32, Option<String>, String)> {
    read(packets, ServerProt::MessageGame, |b| {
        if cfg!(since_647) {
            let kind = if b.data[b.pos] < 128 {
                b.g1() as i32
            } else {
                (b.g2() & 0x7FFF) as i32
            };
            b.g4s();
            let name = (b.g1() != 0).then(|| b.gjstr(STRING_TERMINATOR));
            (kind, name, b.gjstr(STRING_TERMINATOR))
        } else {
            (0, None, b.gjstr(STRING_TERMINATOR))
        }
    })
}

/// Every MIDI_SONG's song.
pub fn songs(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    read(packets, ServerProt::MidiSong, |b| {
        if cfg!(since_742) {
            b.g2()
        } else if cfg!(since_647) {
            b.g1_alt3();
            b.g2_alt3()
        } else {
            b.g2_alt3()
        }
    })
}

/// Every MIDI_JINGLE's jingle.
pub fn jingles(packets: &[(u16, Vec<u8>)]) -> Vec<u16> {
    read(packets, ServerProt::MidiJingle, |b| {
        if cfg!(since_742) {
            b.g1_alt2();
            b.g2_alt3()
        } else if cfg!(since_647) {
            b.g2_alt3()
        } else {
            b.g2_alt1();
            b.g1();
            b.g2_alt1()
        }
    })
}

/// Every SYNTH_SOUND as (synth, delay).
pub fn synths(packets: &[(u16, Vec<u8>)]) -> Vec<(u16, u16)> {
    read(packets, ServerProt::SynthSound, |b| {
        let (synth, _, delay) = (b.g2(), b.g1(), b.g2());
        (synth, delay)
    })
}

/// Every MINIMAP_TOGGLE's state.
pub fn minimap_states(packets: &[(u16, Vec<u8>)]) -> Vec<u8> {
    read(packets, ServerProt::MinimapToggle, |b| b.g1())
}

/// Every URL_OPEN's url.
pub fn urls(packets: &[(u16, Vec<u8>)]) -> Vec<String> {
    of(packets, ServerProt::UrlOpen)
        .map(|p| {
            #[cfg(since_742)]
            let p = &p[1..p.len() - 1];
            String::from_utf8_lossy(p).into_owned()
        })
        .collect()
}

/// A CAM_MOVETO as (local x, local z, height, const speed, var speed).
pub fn cam_movetos(packets: &[(u16, Vec<u8>)]) -> Vec<(u8, u8, u16, u8, u8)> {
    read(packets, ServerProt::CamMoveto, |b| {
        #[cfg(all(since_530, before_647))]
        let (_, x, z, height, cs, vs) = (b.g2(), b.g1(), b.g1(), b.g2(), b.g1(), b.g1());
        #[cfg(all(since_647, before_742))]
        let (x, z, vs, height, cs) = (b.g1_alt1(), b.g1_alt2(), b.g1_alt1(), b.g2(), b.g1_alt1());
        #[cfg(since_742)]
        let (x, z, height, cs, vs) = (b.g1_alt2(), b.g1_alt3(), b.g2_alt1(), b.g1(), b.g1_alt3());
        (x, z, height, cs, vs)
    })
}

/// A CAM_LOOKAT as (local x, local z, height, const speed, var speed).
pub fn cam_lookats(packets: &[(u16, Vec<u8>)]) -> Vec<(u8, u8, u16, u8, u8)> {
    read(packets, ServerProt::CamLookat, |b| {
        #[cfg(all(since_530, before_647))]
        let (_, x, z, height, cs, vs) = (b.g2(), b.g1(), b.g1(), b.g2(), b.g1(), b.g1());
        #[cfg(all(since_647, before_742))]
        let (x, height, z, cs, vs) = (
            b.g1_alt2(),
            b.g2_alt3(),
            b.g1_alt3(),
            b.g1_alt2(),
            b.g1_alt3(),
        );
        #[cfg(since_742)]
        let (height, x, vs, cs, z) = (b.g2_alt1(), b.g1(), b.g1_alt2(), b.g1_alt1(), b.g1_alt3());
        (x, z, height, cs, vs)
    })
}

/// The zone index (x, z) in the build area of every UPDATE_ZONE_PARTIAL_FOLLOWS and
/// UPDATE_ZONE_PARTIAL_ENCLOSED, in the order sent. Before 647 the header is the zone's tile
/// offset.
pub fn zone_partials(packets: &[(u16, Vec<u8>)]) -> Vec<(u8, u8)> {
    let follows = ServerProt::UpdateZonePartialFollows as u16;
    let enclosed = ServerProt::UpdateZonePartialEnclosed as u16;
    packets
        .iter()
        .filter_map(|(op, p)| {
            let b = &mut Packet::from(p.clone());
            match *op {
                op if op == follows => {
                    #[cfg(all(since_530, before_647))]
                    let (x, z) = (b.g1_alt2() >> 3, b.g1() >> 3);
                    #[cfg(all(since_647, before_742))]
                    let (_, x, z) = (b.g1_alt2(), b.g1(), b.g1_alt3());
                    #[cfg(since_742)]
                    let (z, x) = (b.g1_alt1(), b.g1_alt1());
                    Some((x, z))
                }
                op if op == enclosed => {
                    #[cfg(all(since_530, before_647))]
                    let (z, x) = (b.g1_alt1() >> 3, b.g1_alt3() >> 3);
                    #[cfg(all(since_647, before_742))]
                    let (x, _, z) = (b.g1(), b.g1_alt2(), b.g1());
                    #[cfg(since_742)]
                    let (_, x, z) = (b.g1(), b.g1_alt1(), b.g1_alt3());
                    Some((x, z))
                }
                _ => None,
            }
        })
        .collect()
}
