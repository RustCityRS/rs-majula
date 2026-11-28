use rs_io::Packet;
use rs_io::crc::getcrc;
use rs_ver::STRING_TERMINATOR;

#[cfg(before_647)]
const VERSION: u8 = 1;
#[cfg(since_647)]
const VERSION: u8 = 2;

/// A world's activity when it has none. The clients sort "-" after every
/// activity, and from 647 they show the location for an activity of one
/// character or less.
const NO_ACTIVITY: &str = "-";

/// The members bit of a world's flags. The clients also read 0x2 and 0x4
/// (the Quick Chat and PvP columns on 530, sort keys from 647), 0x8 (the
/// LootShare column), 0x10 (an activity world) and, from 647, 0x1000 (left
/// out of the world switcher).
const MEMBERS: i32 = 0x1;

#[derive(Clone)]
pub struct WorldEntry {
    pub id: u16,
    pub address: String,
    pub members: bool,
    pub players: u16,
    pub location: String,
    pub country: u16,
    pub activity: Option<String>,
}

/// A world list encoded for both replies: the full list, and the player
/// counts alone for a client that already holds the list's checksum. The
/// checksum is never 0, which a client that has never had the list sends:
/// the player counts alone would crash it.
pub struct WorldList {
    checksum: i32,
    full: Vec<u8>,
    counts: Vec<u8>,
}

impl WorldList {
    pub fn new(worlds: &[WorldEntry]) -> Self {
        let first = worlds.first().map_or(0, |w| w.id);
        let last = worlds.last().map_or(0, |w| w.id);
        let mut locations: Vec<(u16, &str)> = Vec::new();
        let indices: Vec<u8> = worlds
            .iter()
            .map(|w| {
                let location = (w.country, w.location.as_str());
                match locations.iter().position(|&l| l == location) {
                    Some(i) => i as u8,
                    None if locations.len() <= usize::from(u8::MAX) => {
                        locations.push(location);
                        (locations.len() - 1) as u8
                    }
                    None => 0,
                }
            })
            .collect();
        let mut buf = Packet::new(
            32 + locations
                .iter()
                .map(|(_, name)| 4 + name.len())
                .sum::<usize>()
                + worlds
                    .iter()
                    .map(|w| 20 + w.address.len() + w.activity.as_ref().map_or(1, String::len))
                    .sum::<usize>(),
        );
        buf.p1(VERSION);
        buf.p1(1);
        buf.psmart1or2(locations.len() as i32);
        for &(country, name) in &locations {
            buf.psmart1or2(country.into());
            buf.pjstr2(name, STRING_TERMINATOR);
        }
        buf.psmart1or2(first.into());
        buf.psmart1or2(last.into());
        buf.psmart1or2(worlds.len() as i32);
        for (w, &location) in worlds.iter().zip(&indices) {
            buf.psmart1or2((w.id - first).into());
            buf.p1(location);
            buf.p4(if w.members { MEMBERS } else { 0 });
            buf.pjstr2(
                w.activity.as_deref().unwrap_or(NO_ACTIVITY),
                STRING_TERMINATOR,
            );
            buf.pjstr2(&w.address, STRING_TERMINATOR);
        }
        let checksum = match getcrc(&buf.data, 2, buf.pos) {
            0 => 1,
            crc => crc,
        };
        buf.p4(checksum);
        let start = buf.pos;
        for w in worlds {
            buf.psmart1or2((w.id - first).into());
            buf.p2(w.players);
        }
        let mut counts = Packet::new(2 + buf.pos - start);
        counts.p1(VERSION);
        counts.p1(0);
        counts.pdata(&buf.data, start, buf.pos - start);
        buf.data.truncate(buf.pos);
        WorldList {
            checksum,
            full: buf.data,
            counts: counts.data,
        }
    }

    /// The list for a client that holds the list with `checksum`: the
    /// player counts alone when it is this list's, else the full list.
    pub fn bytes(&self, checksum: i32) -> &[u8] {
        if checksum == self.checksum {
            &self.counts
        } else {
            &self.full
        }
    }

    /// The reply to 530's world list service: status 0, then the list after
    /// its p2 length.
    #[cfg(before_647)]
    pub fn service_reply(&self, checksum: i32) -> Vec<u8> {
        let list = self.bytes(checksum);
        let mut buf = Packet::new(3 + list.len());
        buf.p1(0);
        buf.p2(list.len() as u16);
        buf.pdata(list, 0, list.len());
        buf.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worlds(count: u16) -> Vec<WorldEntry> {
        (1..=count)
            .map(|id| WorldEntry {
                id,
                address: format!("world{id}.example"),
                players: id,
                members: true,
                location: "RuneScape".into(),
                country: 225,
                activity: None,
            })
            .collect()
    }

    fn gjstr2(buf: &mut Packet) -> String {
        assert_eq!(buf.g1(), 0);
        buf.gjstr(0)
    }

    #[test]
    fn a_client_holding_the_list_gets_only_the_player_counts() {
        let list = WorldList::new(&worlds(3));
        assert_ne!(list.checksum, 0);
        assert_eq!(
            list.bytes(list.checksum),
            [VERSION, 0, 0, 0, 1, 1, 0, 2, 2, 0, 3]
        );
        let full = list.bytes(0);
        assert_eq!(full[..2], [VERSION, 1]);
        assert!(full.ends_with(&[0, 0, 1, 1, 0, 2, 2, 0, 3]));
        let at = full.len() - 9 - 4;
        assert_eq!(full[at..at + 4], list.checksum.to_be_bytes());
        assert_eq!(list.checksum, getcrc(full, 2, at));
        assert_ne!(WorldList::new(&worlds(2)).checksum, list.checksum);
    }

    #[test]
    fn worlds_share_a_location_entry_and_carry_their_own_activity() {
        let mut worlds = worlds(3);
        worlds[1].activity = Some("Clan Wars".into());
        worlds[2].location = "Canada".into();
        worlds[2].country = 38;
        let mut buf = Packet::from(WorldList::new(&worlds).bytes(0).to_vec());
        assert_eq!((buf.g1(), buf.g1()), (VERSION, 1));
        let locations: Vec<_> = (0..buf.gsmart1or2())
            .map(|_| (buf.gsmart1or2(), gjstr2(&mut buf)))
            .collect();
        assert_eq!(
            locations,
            [(225, "RuneScape".into()), (38, "Canada".into())]
        );
        assert_eq!(
            (buf.gsmart1or2(), buf.gsmart1or2(), buf.gsmart1or2()),
            (1, 3, 3)
        );
        let rows: Vec<_> = (0..3)
            .map(|_| {
                let id = buf.gsmart1or2();
                let location = buf.g1();
                let flags = buf.g4s();
                (id, location, flags, gjstr2(&mut buf), gjstr2(&mut buf))
            })
            .collect();
        assert_eq!((rows[0].1, rows[0].3.as_str()), (0, NO_ACTIVITY));
        assert_eq!((rows[1].1, rows[1].3.as_str()), (0, "Clan Wars"));
        assert_eq!((rows[2].0, rows[2].1, rows[2].2), (2, 1, MEMBERS));
        assert_eq!(rows[2].4, worlds[2].address);
    }

    #[cfg(before_647)]
    #[test]
    fn the_530_service_reply_is_status_0_then_the_framed_list() {
        let list = WorldList::new(&worlds(1));
        let full = list.service_reply(0);
        assert_eq!(full[0], 0);
        assert_eq!(
            usize::from(u16::from_be_bytes([full[1], full[2]])),
            full.len() - 3
        );
        assert_eq!(full[3..], *list.bytes(0));
        assert_eq!(list.service_reply(list.checksum), [0, 0, 5, 1, 0, 0, 0, 1]);
    }
}
