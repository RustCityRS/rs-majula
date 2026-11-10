use rs_io::Packet;

/// The first smart of a hit that carries a second mark (`Class244`).
const HIT_DOUBLE: i32 = 32767;

/// The first smart of a hit with no splat, only its health bar (`Class244`).
const HIT_NONE: i32 = 32766;

/// Hitmark config 5, the blue splat with the shield icon: damage soaked
/// (2011 wiki, Hitsplat, 2011-04-13; void-634 `HitSplat.Mark.Absorb`).
const HITMARK_SOAK: i32 = 5;

/// Hitmark config 9, the heal, which 2011 never darkens ("this splat has no
/// dark version": 2011 wiki, Hitsplat, 2011-04-13; void-634 `HitSplat`).
const HITMARK_HEAL: i32 = 9;

/// Cache 786's hitmarks 14-27 are 0-13 on darker backgrounds: what a player
/// sees of a hit they neither dealt nor took (2011 wiki, Hitsplat: "Darker
/// splats appear when someone takes damage not dealt or received by you";
/// "Damage absorbed by other players has a darker splat"; void-634
/// `HitSplat` adds 14).
const HITMARK_DARKER: i32 = 14;

/// A hit shown at once.
const NO_DELAY: i32 = 0;

/// A full health bar.
const FULL: u32 = 255;

/// The most a hit takes: a soaked hit's five smarts, its delay and its health bar.
const MAX_SIZE: usize = 12;

#[derive(Clone, Copy, Default)]
pub struct Hit {
    pub hitmark: u8,
    pub damage: u16,
    pub soak: u16,
    pub health: u8,
}

impl Hit {
    /// The health bar's fill, 255 for full: `current` of `base`.
    pub fn health(current: u8, base: u8) -> u8 {
        if base == 0 {
            return 0;
        }
        (current as u32 * FULL / base as u32).min(FULL) as u8
    }

    /// The hit as the client reads it (`Class244`): a soaked hit is one entry
    /// with two marks, which the client paints side by side (32767, the hit's
    /// mark and damage, the soak mark and the amount soaked; void-634
    /// `HitSplat.write`). `bright` is the view of its receiver and dealer;
    /// anyone else sees it darker, and sees no splat for a hit of nothing
    /// (32766), only the health bar. Both views are the same length.
    pub fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>, bright: bool) {
        if !bright && self.damage == 0 {
            buf.psmart1or2(HIT_NONE);
        } else {
            let darker = |mark: i32| {
                if bright || mark == HITMARK_HEAL {
                    mark
                } else {
                    mark + HITMARK_DARKER
                }
            };
            if self.soak > 0 {
                buf.psmart1or2(HIT_DOUBLE);
            }
            buf.psmart1or2(darker(self.hitmark as i32));
            buf.psmart1or2(self.damage as i32);
            if self.soak > 0 {
                buf.psmart1or2(darker(HITMARK_SOAK));
                buf.psmart1or2(self.soak as i32);
            }
        }
        buf.psmart1or2(NO_DELAY);
        buf.p1(self.health);
    }

    pub fn sizeof(&self, bright: bool) -> usize {
        let mut buf = Packet::from([0u8; MAX_SIZE]);
        self.encode(&mut buf, bright);
        buf.pos
    }
}
