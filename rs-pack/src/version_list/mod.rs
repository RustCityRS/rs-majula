use std::collections::HashMap;
use std::path::Path;

use rs_io::jag::JagFile;

use rs_io::js5::Js5Store;

#[derive(Clone, Copy, Debug)]
pub struct MapEntry {
    pub mapsquare: u16,
    pub land_file: u16,
    pub loc_file: u16,
    pub free2play: bool,
}

impl MapEntry {
    pub fn map_x(&self) -> u8 {
        (self.mapsquare >> 8) as u8
    }

    pub fn map_z(&self) -> u8 {
        self.mapsquare as u8
    }
}

#[derive(Debug, Default)]
pub struct VersionList {
    pub maps: Vec<MapEntry>,
    pub model_flags: Vec<u8>,
    pub midi_flags: Vec<u8>,
}

impl VersionList {
    pub fn from_jag(jag: &JagFile) -> Self {
        let maps = jag
            .read("map_index")
            .map(|p| parse_map_index(&p.data))
            .unwrap_or_default();
        let model_flags = jag.read("model_index").map(|p| p.data).unwrap_or_default();
        let midi_flags = jag.read("midi_index").map(|p| p.data).unwrap_or_default();

        Self {
            maps,
            model_flags,
            midi_flags,
        }
    }
}

pub fn parse_map_index(bytes: &[u8]) -> Vec<MapEntry> {
    bytes
        .chunks_exact(7)
        .map(|c| MapEntry {
            mapsquare: u16::from_be_bytes([c[0], c[1]]),
            land_file: u16::from_be_bytes([c[2], c[3]]),
            loc_file: u16::from_be_bytes([c[4], c[5]]),
            free2play: c[6] != 0,
        })
        .collect()
}

pub const TABLE_NAMES: [&str; 12] = [
    "model_version",
    "model_crc",
    "model_index",
    "anim_version",
    "anim_crc",
    "anim_index",
    "midi_version",
    "midi_crc",
    "midi_index",
    "map_version",
    "map_crc",
    "map_index",
];

#[derive(Debug, Default)]
pub struct VersionListMeta {
    pub order: Vec<String>,
    pub model_version: Vec<u16>,
    pub model_crc: Vec<i32>,
    pub model_flags: Vec<u8>,
    pub anim_version: Vec<u16>,
    pub anim_crc: Vec<i32>,
    pub anim_index: Vec<u8>,
    pub midi_version: Vec<u16>,
    pub midi_crc: Vec<i32>,
    pub map_version: Vec<u16>,
    pub map_crc: Vec<i32>,
    pub maps: Vec<MapEntry>,
}

impl VersionListMeta {
    pub fn extract(jag: &JagFile, cache: &Js5Store) -> Self {
        let file_count = (0..).take_while(|&i| jag.get(i).is_some()).count();
        let order = (0..file_count)
            .filter_map(|i| name_for_hash(jag.file_hash(i)).map(String::from))
            .collect();

        let absent_only = |index: usize, name: &str| -> Vec<i32> {
            read_i32_table(jag, name)
                .into_iter()
                .enumerate()
                .map(|(id, c)| {
                    if cache
                        .read(index, id, false)
                        .filter(|d| !d.is_empty())
                        .is_some()
                    {
                        0
                    } else {
                        c
                    }
                })
                .collect()
        };

        Self {
            order,
            model_version: read_u16_table(jag, "model_version"),
            model_crc: absent_only(1, "model_crc"),
            model_flags: jag.read("model_index").map(|p| p.data).unwrap_or_default(),
            anim_version: read_u16_table(jag, "anim_version"),
            anim_crc: absent_only(2, "anim_crc"),
            anim_index: jag.read("anim_index").map(|p| p.data).unwrap_or_default(),
            midi_version: read_u16_table(jag, "midi_version"),
            midi_crc: absent_only(3, "midi_crc"),
            map_version: read_u16_table(jag, "map_version"),
            map_crc: absent_only(4, "map_crc"),
            maps: jag
                .read("map_index")
                .map(|p| parse_map_index(&p.data))
                .unwrap_or_default(),
        }
    }

    pub fn write(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("order"), self.order.join("\n") + "\n")?;

        let mut versions = String::from(
            "# <table> count <n> default <v>, then exceptions '<table> <id> <version>'\n",
        );
        for (name, vers) in self.version_tables() {
            let default = mode(vers);
            versions += &format!("{name} count {} default {}\n", vers.len(), default);
            for (id, &v) in vers.iter().enumerate() {
                if v != default {
                    versions += &format!("{name} {id} {v}\n");
                }
            }
        }
        std::fs::write(dir.join("versions"), versions)?;

        let mut model = String::from(
            "# id  flags(hex bitfield: 0x80 player-chathead, 0x40/0x20 inv, 0x10/0x8 worn, 0x4 static, 0x2 dynamic, 0x1 tutorial)\n",
        );
        for (id, &f) in self.model_flags.iter().enumerate() {
            model += &format!("{id} 0x{f:02x}\n");
        }
        std::fs::write(dir.join("model"), model)?;

        let mut anim_index = String::from("# frame  flags\n");
        for (frame, c) in self.anim_index.chunks_exact(2).enumerate() {
            anim_index += &format!("{frame} {}\n", u16::from_be_bytes([c[0], c[1]]));
        }
        std::fs::write(dir.join("anim_index"), anim_index)?;

        // map_index: loc_file is always land_file + 1, so it is omitted and derived.
        let mut map_index =
            String::from("# square  land_file  free2play|members   (loc_file = land_file + 1)\n");
        for m in &self.maps {
            map_index += &format!(
                "m{}_{} {} {}\n",
                m.map_x(),
                m.map_z(),
                m.land_file,
                if m.free2play { "free2play" } else { "members" }
            );
        }
        std::fs::write(dir.join("map_index"), map_index)?;

        let mut absent = String::from(
            "# index(1=model 2=anim 3=midi 4=map)  id  crc - stale crcs of referenced-but-absent files (signed i32, as getcrc returns)\n",
        );
        for (index, crcs) in self.crc_tables() {
            for (id, &c) in crcs.iter().enumerate() {
                if c != 0 {
                    absent += &format!("{index} {id} {c}\n");
                }
            }
        }
        std::fs::write(dir.join("absent_crc"), absent)?;
        Ok(())
    }

    pub fn read(dir: &Path) -> Self {
        let order = data_lines(dir, "order").collect::<Vec<_>>();

        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut defaults: HashMap<String, u16> = HashMap::new();
        let mut exceptions: Vec<(String, usize, u16)> = Vec::new();
        for line in data_lines(dir, "versions") {
            let t: Vec<&str> = line.split_whitespace().collect();
            match t.as_slice() {
                [table, "count", n, "default", d] => {
                    counts.insert(table.to_string(), n.parse().unwrap_or(0));
                    defaults.insert(table.to_string(), d.parse().unwrap_or(0));
                }
                [table, id, v] => {
                    if let (Ok(id), Ok(v)) = (id.parse::<usize>(), v.parse::<u16>()) {
                        exceptions.push((table.to_string(), id, v));
                    }
                }
                _ => {}
            }
        }
        let table_versions = |name: &str| -> Vec<u16> {
            let mut v = vec![
                defaults.get(name).copied().unwrap_or(0);
                counts.get(name).copied().unwrap_or(0)
            ];
            for (t, id, ver) in &exceptions {
                if t == name && *id < v.len() {
                    v[*id] = *ver;
                }
            }
            v
        };
        let model_version = table_versions("model");
        let anim_version = table_versions("anim");
        let midi_version = table_versions("midi");
        let map_version = table_versions("map");

        let model_flags: Vec<u8> = data_lines(dir, "model")
            .filter_map(|l| l.split_whitespace().nth(1).map(|s| parse_u32(s) as u8))
            .collect();

        let mut anim_index = Vec::new();
        for line in data_lines(dir, "anim_index") {
            if let Some(v) = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u16>().ok())
            {
                anim_index.extend_from_slice(&v.to_be_bytes());
            }
        }

        // map_index: loc_file = land_file + 1.
        let mut maps = Vec::new();
        for line in data_lines(dir, "map_index") {
            let t: Vec<&str> = line.split_whitespace().collect();
            let (Some((mx, mz)), true) = (parse_square(t.first().copied()), t.len() >= 3) else {
                continue;
            };
            let land: u16 = t[1].parse().unwrap_or(0);
            maps.push(MapEntry {
                mapsquare: ((mx as u16) << 8) | mz as u16,
                land_file: land,
                loc_file: land + 1,
                free2play: matches!(t[2], "free2play" | "free" | "1" | "true"),
            });
        }

        let mut model_crc = vec![0i32; model_version.len()];
        let mut anim_crc = vec![0i32; anim_version.len()];
        let mut midi_crc = vec![0i32; midi_version.len()];
        let mut map_crc = vec![0i32; map_version.len()];
        for line in data_lines(dir, "absent_crc") {
            let t: Vec<&str> = line.split_whitespace().collect();
            if t.len() < 3 {
                continue;
            }
            let id: usize = t[1].parse().unwrap_or(usize::MAX);
            let crc = parse_i32(t[2]);
            let table = match t[0] {
                "1" => &mut model_crc,
                "2" => &mut anim_crc,
                "3" => &mut midi_crc,
                "4" => &mut map_crc,
                _ => continue,
            };
            if id < table.len() {
                table[id] = crc;
            }
        }

        Self {
            order,
            model_version,
            model_crc,
            model_flags,
            anim_version,
            anim_crc,
            anim_index,
            midi_version,
            midi_crc,
            map_version,
            map_crc,
            maps,
        }
    }

    fn version_tables(&self) -> [(&'static str, &[u16]); 4] {
        [
            ("model", &self.model_version),
            ("anim", &self.anim_version),
            ("midi", &self.midi_version),
            ("map", &self.map_version),
        ]
    }

    fn crc_tables(&self) -> [(u8, &[i32]); 4] {
        [
            (1, &self.model_crc),
            (2, &self.anim_crc),
            (3, &self.midi_crc),
            (4, &self.map_crc),
        ]
    }
}

fn mode(xs: &[u16]) -> u16 {
    let mut counts: HashMap<u16, usize> = HashMap::new();
    for &x in xs {
        *counts.entry(x).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|&(_, c)| c)
        .map(|(v, _)| v)
        .unwrap_or(0)
}

fn name_for_hash(h: i32) -> Option<&'static str> {
    TABLE_NAMES.iter().copied().find(|n| JagFile::hash(n) == h)
}

fn read_u16_table(jag: &JagFile, name: &str) -> Vec<u16> {
    jag.read(name)
        .map(|p| {
            p.data
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect()
        })
        .unwrap_or_default()
}

fn read_i32_table(jag: &JagFile, name: &str) -> Vec<i32> {
    jag.read(name)
        .map(|p| {
            p.data
                .chunks_exact(4)
                .map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        })
        .unwrap_or_default()
}

fn data_lines(dir: &Path, name: &str) -> impl Iterator<Item = String> {
    std::fs::read_to_string(dir.join(name))
        .unwrap_or_default()
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect::<Vec<_>>()
        .into_iter()
}

fn parse_u32(s: &str) -> u32 {
    s.strip_prefix("0x")
        .and_then(|h| u32::from_str_radix(h, 16).ok())
        .or_else(|| s.parse().ok())
        .unwrap_or(0)
}

fn parse_i32(s: &str) -> i32 {
    match s.strip_prefix("0x") {
        Some(h) => u32::from_str_radix(h, 16).map(|v| v as i32).unwrap_or(0),
        None => s.parse().unwrap_or(0),
    }
}

fn parse_square(s: Option<&str>) -> Option<(u8, u8)> {
    let (x, z) = s?.strip_prefix('m')?.split_once('_')?;
    Some((x.parse().ok()?, z.parse().ok()?))
}
