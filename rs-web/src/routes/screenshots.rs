use crate::app::{App, AppError};
use crate::routes::pages::not_found;
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use minijinja::context;
use minijinja::value::Serde;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// How many screenshots an index page shows: three rows of two.
const PER_PAGE: usize = 6;

/// One of the January 2005 site's game screenshots, under
/// `/img/screenshots/`. Those whose picture didn't survive in the Web
/// Archive are left out; `full` is `None` where only the thumbnail did.
#[derive(Serialize)]
struct Shot {
    name: &'static str,
    thumb: &'static str,
    full: Option<&'static str>,
    caption: &'static str,
}

/// In the old site's order: the free game's, then the members'.
const SHOTS: &[Shot] = &[
    Shot {
        name: "free_chardesign",
        thumb: "free_chardesign.jpg",
        full: Some("free_chardesign.jpg"),
        caption: "Choose Your Look",
    },
    Shot {
        name: "free_alkharid",
        thumb: "free_alkharid.jpg",
        full: Some("free_alkharid.jpg"),
        caption: "The Heat Of The Desert",
    },
    Shot {
        name: "free_draynor",
        thumb: "free_draynor.jpg",
        full: Some("free_draynor.jpg"),
        caption: "Explore Haunted Mansions!",
    },
    Shot {
        name: "free_darkwizardscircle",
        thumb: "free_darkwizardcircle.jpg",
        full: Some("free_darkwizardcircle.jpg"),
        caption: "Discover Mysterious Ruins",
    },
    Shot {
        name: "free_faladorcastle",
        thumb: "free_faladorcastle.jpg",
        full: Some("free_faladorcastle.jpg"),
        caption: "Join The White Knights",
    },
    Shot {
        name: "free_firemake2",
        thumb: "free_firemake2.jpg",
        full: Some("free_firemake2.jpg"),
        caption: "Light Up Your Life",
    },
    Shot {
        name: "free_flyfish2",
        thumb: "free_flyfish2.jpg",
        full: Some("free_flyfish2.jpg"),
        caption: "Unwind With A Little Fishing",
    },
    Shot {
        name: "free_karamjadock",
        thumb: "free_karamjadock.jpg",
        full: Some("free_karamjadock.jpg"),
        caption: "Sail Away To A Tropical Paradise",
    },
    Shot {
        name: "free_wilderness2",
        thumb: "free_wilderness2.jpg",
        full: Some("free_wilderness2.jpg"),
        caption: "Visit Ancient Battlefields",
    },
    Shot {
        name: "free_lumbridge_map2",
        thumb: "free_lumbridge_map2.jpg",
        full: None,
        caption: "Navigate Your Surroundings",
    },
    Shot {
        name: "free_lumbridgesmelting",
        thumb: "free_lumbridgesmelting.jpg",
        full: None,
        caption: "Earn A Living As A Smith",
    },
    Shot {
        name: "free_lumbridgeswamp",
        thumb: "free_lumbridgeswamp.jpg",
        full: None,
        caption: "Meander Through Murky Swamps",
    },
    Shot {
        name: "free_newbieisland1",
        thumb: "free_newbieisland1.jpg",
        full: Some("free_newbieisland1.jpg"),
        caption: "Welcome To RuneScape!",
    },
    Shot {
        name: "free_newbieisland10",
        thumb: "free_newbieisland10.jpg",
        full: Some("free_newbieisland10.jpg"),
        caption: "Learn From Holy Men In Temples",
    },
    Shot {
        name: "free_newbieisland2",
        thumb: "free_newbieisland2.jpg",
        full: Some("free_newbieisland2.jpg"),
        caption: "Admire Medieval Architecture",
    },
    Shot {
        name: "free_newbieisland5",
        thumb: "free_newbieisland5.jpg",
        full: None,
        caption: "Cook Over Campfires",
    },
    Shot {
        name: "free_runeessence",
        thumb: "free_runeessence.jpg",
        full: Some("free_runeessence.jpg"),
        caption: "Mine Stones From Secret Locations",
    },
    Shot {
        name: "free_varrockmarket4",
        thumb: "free_varrockmarket4.jpg",
        full: Some("free_varrockmarket4.jpg"),
        caption: "Trade And Sell With Other Players",
    },
    Shot {
        name: "free_wilderness1",
        thumb: "free_wilderness1.jpg",
        full: Some("free_wilderness1.jpg"),
        caption: "Battle Other Players To The Death",
    },
    Shot {
        name: "free_lumbridge_bridge",
        thumb: "free_lumbridge_bridge.jpg",
        full: Some("free_lumbridge_bridge.jpg"),
        caption: "Watch The World Pass You By",
    },
    Shot {
        name: "free_wilderness6",
        thumb: "free_wilderness6.jpg",
        full: Some("free_wilderness6.jpg"),
        caption: "Ransack Evil Temples",
    },
    Shot {
        name: "free_zombiegraveyard",
        thumb: "free_zombiegraveyard.jpg",
        full: None,
        caption: "Combat Sinister Zombies",
    },
    Shot {
        name: "members_battlefield",
        thumb: "members_battlefield.jpg",
        full: Some("members_battlefield.jpg"),
        caption: "Join Armies Going To War",
    },
    Shot {
        name: "members_duelarena",
        thumb: "members_duelarena.jpg",
        full: None,
        caption: "Spectate Gladiators In Combat",
    },
    Shot {
        name: "members_dwarftunnel",
        thumb: "members_dwarftunnel.jpg",
        full: None,
        caption: "Wander Hidden Dwarf Tunnels",
    },
    Shot {
        name: "members_elementalworkshop",
        thumb: "members_elementalworkshop.jpg",
        full: None,
        caption: "Rediscover Forgotten Technologies",
    },
    Shot {
        name: "members_entrana_organ",
        thumb: "members_entrana_organ.jpg",
        full: None,
        caption: "Visit Majestic Cathedrals",
    },
    Shot {
        name: "members_fishing_guild",
        thumb: "members_fishing_guild.jpg",
        full: None,
        caption: "Fishing On The Dock Of The Bay",
    },
    Shot {
        name: "members_gnomeagility",
        thumb: "members_gnomeagility.jpg",
        full: None,
        caption: "Use Agility To Reach New Areas",
    },
    Shot {
        name: "members_grandtreeglider",
        thumb: "members_grandtreeglider.jpg",
        full: None,
        caption: "Discover New Methods Of Travel",
    },
    Shot {
        name: "members_iceplateau3",
        thumb: "members_iceplateau3.jpg",
        full: None,
        caption: "Battle In Frozen Wastelands",
    },
    Shot {
        name: "members_karamjajungle",
        thumb: "members_karamjajungle.jpg",
        full: None,
        caption: "Explore The Deadly Jungles",
    },
    Shot {
        name: "members_legendsquest",
        thumb: "members_legendsquest.jpg",
        full: None,
        caption: "Solve Mysterious Puzzles",
    },
    Shot {
        name: "members_mageguild2",
        thumb: "members_mageguild2.jpg",
        full: None,
        caption: "Discover Mystical Portals",
    },
    Shot {
        name: "members_partyroom",
        thumb: "members_partyroom.jpg",
        full: Some("members_partyroom.jpg"),
        caption: "Host Parties With Your Friends",
    },
    Shot {
        name: "members_piratebar",
        thumb: "members_piratebar.jpg",
        full: None,
        caption: "Infiltrate Pirate Hideouts",
    },
    Shot {
        name: "members_trainingground2",
        thumb: "members_trainingground2.jpg",
        full: None,
        caption: "Train On Captive Foes",
    },
    Shot {
        name: "members_waterfallrapids",
        thumb: "members_waterfallrapids.jpg",
        full: None,
        caption: "Travel On Dangerous Rapids",
    },
    Shot {
        name: "members_sorcerortower",
        thumb: "members_sorcerortower.jpg",
        full: None,
        caption: "Help a Mighty Sorcerer",
    },
    Shot {
        name: "members_werewolves",
        thumb: "members_werewolves.jpg",
        full: None,
        caption: "Battle Mighty Werewolves",
    },
    Shot {
        name: "members_wildyagility",
        thumb: "members_wildyagility.jpg",
        full: None,
        caption: "Discover Secret Training Areas",
    },
];

/// The free game's or the members'.
fn group(shot: &Shot) -> &'static str {
    if shot.name.starts_with("members_") {
        "members"
    } else {
        "free"
    }
}

/// `group`'s screenshots.
fn in_group(group_name: &str) -> Vec<&'static Shot> {
    SHOTS.iter().filter(|s| group(s) == group_name).collect()
}

/// How many index pages `group`'s screenshots fill.
fn pages(group_name: &str) -> usize {
    in_group(group_name).len().div_ceil(PER_PAGE)
}

#[derive(Deserialize)]
pub struct IndexQuery {
    page: Option<usize>,
}

/// `/screenshots?page=N`: the free game's pages first, then the members'.
pub async fn index(
    State(app): State<Arc<App>>,
    Query(q): Query<IndexQuery>,
) -> Result<Response, AppError> {
    let free_pages = pages("free");
    let page = q.page.unwrap_or(1);
    let (group_name, first) = if page <= free_pages {
        ("free", 1)
    } else {
        ("members", free_pages + 1)
    };
    if page == 0 || page >= first + pages(group_name) {
        return Ok(not_found());
    }
    let start = (page - first) * PER_PAGE;
    let shots: Vec<&Shot> = in_group(group_name)
        .into_iter()
        .skip(start)
        .take(PER_PAGE)
        .collect();
    let html = app.templates.render(
        "views/screenshots.html",
        context! {
            page,
            free_pages => (1..=free_pages).collect::<Vec<_>>(),
            members_pages => (free_pages + 1..free_pages + 1 + pages("members")).collect::<Vec<_>>(),
            rows => Serde(shots.chunks(2).map(<[_]>::to_vec).collect::<Vec<_>>()),
        },
    )?;
    Ok(html.into_response())
}

/// `/screenshots/<name>`: one screenshot, with the ones either side of it
/// in its group.
pub async fn shot(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
) -> Result<Response, AppError> {
    let Some(shot) = SHOTS.iter().find(|s| s.name == name) else {
        return Ok(not_found());
    };
    let group_shots = in_group(group(shot));
    let at = group_shots.iter().position(|s| s.name == name).unwrap_or(0);
    let prev = at.checked_sub(1).map(|i| group_shots[i].name);
    let next = group_shots.get(at + 1).map(|s| s.name);
    let html = app.templates.render(
        "views/screenshot.html",
        context! { shot => Serde(shot), prev, next },
    )?;
    Ok(html.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn every_picture_exists() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("public/img/screenshots");
        for shot in SHOTS {
            assert!(
                dir.join("thumbs").join(shot.thumb).is_file(),
                "{}",
                shot.thumb
            );
            if let Some(full) = shot.full {
                assert!(dir.join("shots").join(full).is_file(), "{full}");
            }
        }
    }

    #[test]
    fn names_are_unique() {
        for (i, shot) in SHOTS.iter().enumerate() {
            assert!(
                SHOTS[i + 1..].iter().all(|s| s.name != shot.name),
                "{}",
                shot.name
            );
        }
    }
}
