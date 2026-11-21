use rustc_hash::FxHashMap;

/// The client's table of subs (IF_OPENSUB): the interface each parent
/// component holds and its type, and the events IF_SETEVENTS gave each
/// component slot.
#[derive(Default)]
pub struct SubTable {
    pub open: FxHashMap<i32, (u16, u8)>,
    pub events: FxHashMap<(i32, u16), u32>,
}

impl SubTable {
    /// Records IF_OPENSUB. As the client does, whatever was mounted on
    /// `parent` closes first.
    pub fn opened(&mut self, parent: i32, sub: u16, kind: u8) {
        self.closed(parent);
        self.open.insert(parent, (sub, kind));
    }

    /// Records IF_CLOSESUB. The client closes the sub mounted on `parent`,
    /// then, recursively, every type-3 sub mounted inside it
    /// (`LightDetailClientOption.method2499`); others stay in its table,
    /// unreachable until something re-parents them.
    pub fn closed(&mut self, parent: i32) {
        let Some((closed, _)) = self.open.remove(&parent) else {
            return;
        };
        self.events
            .retain(|&(com, _), _| (com as u32 >> 16) as u16 != closed);
        let children: Vec<i32> = self
            .open
            .iter()
            .filter(|(p, (_, kind))| *kind == 3 && (**p as u32 >> 16) as u16 == closed)
            .map(|(&p, _)| p)
            .collect();
        for child in children {
            self.closed(child);
        }
    }

    /// Records IF_MOVESUB: the sub on `from` moves to `to`, closing whatever
    /// `to` held, as the client's branch for it does.
    pub fn moved(&mut self, from: i32, to: i32) {
        let moving = self.open.remove(&from);
        self.closed(to);
        if let Some(sub) = moving {
            self.open.insert(to, sub);
        }
    }

    /// The parent component interface `sub` is mounted on, and its type.
    pub fn holder(&self, sub: u16) -> Option<(i32, u8)> {
        self.open
            .iter()
            .find(|(_, (s, _))| *s == sub)
            .map(|(&parent, &(_, kind))| (parent, kind))
    }

    /// The IF_OPENSUB type for a sub mounted on `parent`: that of the sub
    /// holding `parent`'s interface, or 1 on the toplevel.
    pub fn kind(&self, parent: i32) -> u8 {
        self.holder((parent as u32 >> 16) as u16)
            .map_or(1, |(_, kind)| kind)
    }

    /// Whether interface `iface` is on screen under `toplevel`: the toplevel,
    /// or a sub mounted on a component of something that is. A sub left on a
    /// parent that is no longer reachable (after IF_OPENTOP swaps the root)
    /// is in the client's table but drawn nowhere.
    pub fn is_open(&self, toplevel: u16, iface: u16) -> bool {
        let mut current = iface;
        for _ in 0..16 {
            if current == toplevel {
                return true;
            }
            let Some((parent, _)) = self.holder(current) else {
                return false;
            };
            current = (parent as u32 >> 16) as u16;
        }
        false
    }

    /// Records IF_SETEVENTS for slots `slot1` to `slot2` of `com`, -1 being
    /// the component itself.
    pub fn set_events(&mut self, com: i32, slot1: i32, slot2: i32, settings: u32) {
        for slot in slot1..=slot2 {
            self.events.insert((com, slot as u16), settings);
        }
    }

    /// The events `slot` of `com` allows: those IF_SETEVENTS gave it, else
    /// for the component itself (slot 65535) its own, `active`.
    pub fn events(&self, com: i32, slot: u16, active: u32) -> Option<u32> {
        self.events
            .get(&(com, slot))
            .copied()
            .or_else(|| (slot == u16::MAX).then_some(active))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(iface: u16, com: u16) -> i32 {
        ((iface as i32) << 16) | com as i32
    }

    /// A sub is open when it hangs, mount by mount, off the toplevel.
    #[test]
    fn an_interface_is_open_when_it_hangs_off_the_toplevel() {
        let mut subs = SubTable::default();
        subs.opened(hash(548, 206), 149, 1);
        subs.opened(hash(548, 18), 762, 0);
        subs.opened(hash(762, 3), 9999, 3);
        assert!(subs.is_open(548, 548), "the toplevel");
        assert!(subs.is_open(548, 149) && subs.is_open(548, 762));
        assert!(subs.is_open(548, 9999), "two mounts down");
        assert!(!subs.is_open(548, 763), "never opened");
        assert_eq!(subs.kind(hash(762, 5)), 0, "inside the bank");
        assert_eq!(subs.kind(hash(548, 5)), 1, "on the toplevel");
    }

    /// IF_OPENTOP leaves the client's sub table alone; what hung off the old
    /// root is no longer drawn.
    #[test]
    fn a_new_toplevel_strands_what_hung_off_the_old_one() {
        let mut subs = SubTable::default();
        subs.opened(hash(548, 206), 149, 1);
        assert!(!subs.is_open(746, 149));
        subs.moved(hash(548, 206), hash(746, 90));
        assert!(subs.is_open(746, 149), "moved across, it is drawn again");
    }

    /// Mounting on an occupied component closes the occupant, and closing a
    /// sub takes its type-3 children and its events with it.
    #[test]
    fn opening_over_a_sub_closes_it_and_its_type_3_children() {
        let mut subs = SubTable::default();
        subs.opened(hash(548, 18), 762, 0);
        subs.opened(hash(762, 3), 9999, 3);
        subs.opened(hash(762, 4), 8888, 1);
        subs.set_events(hash(762, 7), -1, 2, 6);
        assert_eq!(subs.events(hash(762, 7), u16::MAX, 0), Some(6));
        subs.opened(hash(548, 18), 620, 0);
        assert!(!subs.is_open(548, 762) && !subs.is_open(548, 9999));
        assert!(
            !subs.open.contains_key(&hash(762, 3)),
            "type 3 closed with it"
        );
        assert!(
            subs.open.contains_key(&hash(762, 4)),
            "type 1 stays in the table"
        );
        assert!(!subs.is_open(548, 8888), "but is drawn nowhere");
        assert!(subs.is_open(548, 620));
        assert_eq!(subs.events(hash(762, 7), 1, 0), None, "its events went too");
        assert_eq!(subs.events(hash(762, 7), u16::MAX, 3), Some(3));
    }
}
