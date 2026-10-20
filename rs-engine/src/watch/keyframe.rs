//! Building a replay keyframe: what a client that has just logged in needs
//! to show what a player's client shows now, so a replay can start there.
//!
//! The map, what's on it, inventories, and the players and NPCs in view are
//! built here from the world's own state, read-only: the messages go to the
//! player's capture list, not to them. What only the client holds on to
//! (interface text, varps, the camera...) comes from [`super::state`].

use crate::active_player::ActivePlayer;
use crate::engine::{Engine, engine};
use crate::info::{NpcInfo, PlayerInfo};
use rs_entity::EntityLifeTime;
use rs_inv::Inventory;
use rs_pack::cache::inv::InvScope;
use rs_protocol::network::game::server::loc_add_change::LocAddChange;
use rs_protocol::network::game::server::loc_del::LocDel;
use rs_protocol::network::game::server::obj_add::ObjAdd;
use rs_protocol::network::game::server::update_zone_full_follows::UpdateZoneFullFollows;
use rs_protocol::network::game::server::update_zone_partial_follows::UpdateZonePartialFollows;
use rs_vm::engine::ScriptEngine;
use rs_zone::zone_map::ZoneMap;
use rustc_hash::FxHashMap;

impl Engine {
    /// Records a keyframe of `active`'s session, after this tick's output.
    pub(crate) fn replay_keyframe(&mut self, active: &mut ActivePlayer) {
        active.capture = Some(Vec::new());
        active.keyframe_map();
        active.keyframe_zones(&self.zones, self.clock);
        let head = active.capture.replace(Vec::new()).unwrap_or_default();
        active.keyframe_invs(&self.invs);
        for body in
            PlayerInfo::keyframe(&mut self.player_renderer, &self.player_list.players, active)
        {
            active.player_info(&body);
        }
        for body in NpcInfo::keyframe(&mut self.npc_renderer, &self.npc_list.npcs, active) {
            active.npc_info(&body);
        }
        let tail = active.capture.take().unwrap_or_default();
        active.watch.keyframe(self.clock, &head, &tail);
    }
}

impl ActivePlayer {
    /// The map the client has loaded: where it last rebuilt.
    fn keyframe_map(&mut self) {
        let origin = self.player.build_area.origin;
        #[cfg(rev = "225")]
        {
            let cache = engine().cache;
            let mut crcs = std::collections::HashMap::new();
            let mapsquares = self.player.build_area.mapsquares.clone();
            for &mapsquare in &mapsquares {
                let x = (mapsquare >> 8) as u8;
                let z = (mapsquare & 0xFF) as u8;
                for kind in ['m', 'l'] {
                    if let Some(crc) = cache.mapcrcs.get(&(kind, x, z)).copied() {
                        crcs.insert((kind, x, z), crc);
                    }
                }
            }
            self.write(
                rs_protocol::network::game::server::rebuild_normal::RebuildNormal {
                    zone_x: origin.zone_x(),
                    zone_z: origin.zone_z(),
                    mapsquares: mapsquares.into_iter().collect(),
                    crcs,
                },
            );
        }
        #[cfg(since_244)]
        self.write(
            rs_protocol::network::game::server::rebuild_normal::RebuildNormal {
                zone_x: origin.zone_x(),
                zone_z: origin.zone_z(),
            },
        );
    }

    /// Everything on the zones the client has loaded, as when it first
    /// loads one.
    fn keyframe_zones(&mut self, zones: &ZoneMap, clock: u32) {
        let origin_x = self.player.build_area.origin.zone_origin_x();
        let origin_z = self.player.build_area.origin.zone_origin_z();
        let user37 = self.player.uid.username37();
        let loaded = self.player.build_area.loaded_zones.clone();
        for zone_coord in loaded {
            let x = zone_coord.x().wrapping_sub(origin_x) as u8;
            let z = zone_coord.z().wrapping_sub(origin_z) as u8;
            self.write(UpdateZoneFullFollows { x, z });
            let Some(zone) = zones.zone(zone_coord.x(), zone_coord.y(), zone_coord.z()) else {
                continue;
            };
            let mut follows = false;
            for obj in zone.visible_objs(user37, clock) {
                if !follows {
                    self.write(UpdateZonePartialFollows { x, z });
                    follows = true;
                }
                self.write(ObjAdd {
                    coord: obj.packed_zone_coord(),
                    id: obj.id(),
                    count: obj.count() as u16,
                });
            }
            for loc in &zone.locs {
                let coord = loc.packed_zone_coord();
                let shape_angle = loc.packed_shape_angle();
                let hidden = loc.lifetime() == EntityLifeTime::Respawn && !loc.visible();
                let added = loc.lifetime() == EntityLifeTime::Despawn || loc.is_changed();
                if !hidden && !added {
                    continue;
                }
                if !follows {
                    self.write(UpdateZonePartialFollows { x, z });
                    follows = true;
                }
                if hidden {
                    self.write(LocDel { coord, shape_angle });
                } else {
                    self.write(LocAddChange {
                        coord,
                        shape_angle,
                        id: loc.id(),
                    });
                }
            }
        }
    }

    /// The inventories shown on the client's interfaces, in full.
    fn keyframe_invs(&mut self, shared: &FxHashMap<u16, Inventory>) {
        let invs = engine().invs();
        let transmits: Vec<(u16, Vec<u16>)> = self
            .player
            .inv_transmits
            .iter()
            .map(|(inv, coms)| (*inv, coms.to_vec()))
            .collect();
        for (inv_id, coms) in transmits {
            let is_shared = invs
                .get_by_id(inv_id)
                .is_some_and(|t| t.scope == InvScope::Shared);
            let inv = if is_shared {
                shared.get(&inv_id)
            } else {
                self.player.invs.get(&inv_id)
            };
            let Some(inv) = inv else { continue };
            let full: Vec<Option<(u16, i32)>> = inv
                .slots
                .iter()
                .map(|i| i.map(|it| (it.obj, it.num as i32)))
                .collect();
            for com in coms {
                self.update_inv_full(com, &full);
            }
        }
    }
}
