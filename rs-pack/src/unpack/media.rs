use std::path::Path;

use crate::unpack;
use rs_io::jag::JagFile;
use tracing::debug;

const MEDIA_NAMES: &[&str] = &[
    "backbase1",
    "backbase2",
    "backhmid1",
    "backhmid2",
    "backleft1",
    "backleft2",
    "backright1",
    "backright2",
    "backtop1",
    "backtop2",
    "backvmid1",
    "backvmid2",
    "backvmid3",
    "chatback",
    "combatboxes",
    "combaticons",
    "combaticons2",
    "combaticons3",
    "compass",
    "cross",
    "gnomeball_buttons",
    "headicons",
    "hitmarks",
    "index",
    "invback",
    "leftarrow",
    "magicoff",
    "magicoff2",
    "magicon",
    "magicon2",
    "mapback",
    "mapdots",
    "mapflag",
    "mapfunction",
    "mapscene",
    #[cfg(since_244)]
    "mapedge",
    #[cfg(since_244)]
    "mapmarker",
    #[cfg(since_244)]
    "mod_icons",
    "miscgraphics",
    "miscgraphics2",
    "miscgraphics3",
    "prayerglow",
    "prayeroff",
    "prayeron",
    "redstone1",
    "redstone2",
    "redstone3",
    "rightarrow",
    "scrollbar",
    "sideicons",
    "staticons",
    "staticons2",
    "steelborder",
    "steelborder2",
    "sworddecor",
    "tradebacking",
    "wornicons",
];

pub fn unpack_media(jag: &JagFile, output_dir: &Path) -> anyhow::Result<()> {
    let sprite_dir = output_dir.join("sprites");
    std::fs::create_dir_all(&sprite_dir)?;
    let meta_dir = sprite_dir.join("meta");
    std::fs::create_dir_all(&meta_dir)?;

    let index_data = jag
        .read("index.dat")
        .ok_or_else(|| anyhow::anyhow!("Missing index.dat in media JAG"))?;

    let mut sprites: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            continue;
        }
        if let Some(name) = find_media_name(hash)
            && let Some(dat) = jag.read(&format!("{name}.dat"))
        {
            sprites.push((name.to_string(), dat.data));
        }
    }

    let mut index_positions: Vec<(String, u16, Vec<u8>)> = sprites
        .into_iter()
        .map(|(name, data)| {
            let pos = if data.len() >= 2 {
                ((data[0] as u16) << 8) | data[1] as u16
            } else {
                0
            };
            (name, pos, data)
        })
        .collect();

    index_positions.sort_by_key(|(_, pos, _)| *pos);

    let index_order: Vec<String> = index_positions.iter().map(|(n, _, _)| n.clone()).collect();

    for (name, _, dat_data) in &index_positions {
        let sub_dir = sprite_dir.join(name);
        std::fs::create_dir_all(&sub_dir)?;
        unpack::decode_sprite_group(&index_data.data, dat_data, &sub_dir)?;
    }

    let index_content = index_order.join("\n") + "\n";
    std::fs::write(meta_dir.join("index.order"), &index_content)?;

    let mut sprite_order_entries = Vec::new();
    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            sprite_order_entries.push("index".to_string());
        } else if let Some(name) = find_media_name(hash) {
            sprite_order_entries.push(name.to_string());
        }
    }
    let sprite_order_content = sprite_order_entries.join("\n") + "\n";
    std::fs::write(meta_dir.join("sprite.order"), &sprite_order_content)?;

    debug!(
        "Unpacked {} sprite groups from media JAG",
        index_order.len()
    );
    Ok(())
}

fn find_media_name(hash: i32) -> Option<&'static str> {
    for name in MEDIA_NAMES {
        if JagFile::hash(&format!("{name}.dat")) == hash {
            return Some(name);
        }
    }
    None
}

pub(crate) fn known_hashes() -> Vec<i32> {
    let mut hashes = vec![JagFile::hash("index.dat")];
    hashes.extend(
        MEDIA_NAMES
            .iter()
            .map(|n| JagFile::hash(&format!("{n}.dat"))),
    );
    hashes
}
