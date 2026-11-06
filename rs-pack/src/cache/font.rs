#[cfg(since_647)]
use crate::archive;
use crate::types::Font;
use rs_io::jag::JagFile;
#[cfg(since_647)]
use rs_io::js5::{Js5Store, java_string_hash};
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(since_647)]
use tracing::warn;

#[cfg(before_274)]
const CHAR_LOOKUP: [u8; 256] = {
    // £ (Latin-1 byte 163) at position 63 is handled separately below as \0 placeholder
    let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!\"\xA3$%^&*()-_=+[{]};:'@#~,<.>/?\\| ";
    let charset_len = charset.len();
    let mut lookup = [74; 256];
    let mut i: usize = 0;
    while i < 256 {
        let mut j: usize = 0;
        while j < charset_len {
            if charset[j] == i as u8 {
                lookup[i] = j as u8;
                break;
            }
            j += 1;
        }
        i += 1;
    }
    lookup
};

pub struct FontType {
    pub id: u8,
    pub draw_width: [u8; 256],
}

impl FontType {
    pub fn string_width(&self, s: &str) -> u16 {
        let bytes = s.as_bytes();
        let mut size = 0;
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'@' && i + 4 < bytes.len() && bytes[i + 4] == b'@' {
                i += 5;
            } else {
                size += self.draw_width[bytes[i] as usize] as u16;
                i += 1;
            }
        }
        size
    }

    pub fn split(&self, s: &str, max_width: u16) -> Vec<String> {
        if s.is_empty() {
            return vec![s.to_string()];
        }

        let mut lines = Vec::new();
        let mut saved_col: Option<String> = None;
        let mut remaining = s.to_string();

        while !remaining.is_empty() {
            // check if the string even needs to be broken up
            let width = self.string_width(&remaining);
            if width <= max_width && !remaining.contains('|') {
                lines.push(remaining);
                break;
            }

            // we need to split on the next word boundary
            let mut split_index = remaining.len();
            let bytes = remaining.as_bytes();

            for i in 0..bytes.len() {
                if bytes[i] == b' ' {
                    let w = self.string_width(&remaining[..i]);
                    if w > max_width {
                        break;
                    }
                    split_index = i;
                } else if bytes[i] == b'|' {
                    split_index = i;
                    break;
                }
            }

            let line = remaining[..split_index].to_string();

            // save color from the emitted line
            if line.contains('@') {
                let lb = line.as_bytes();
                let mut i = 0;
                while i + 4 < lb.len() {
                    if lb[i] == b'@' && lb[i + 4] == b'@' {
                        if &lb[i + 1..i + 4] == b"str" {
                            saved_col = None;
                            if lb.get(i + 5..i + 10) == Some(&b"@bla@"[..]) {
                                i += 10;
                                continue;
                            }
                        } else {
                            saved_col = Some(line[i..i + 5].to_string());
                        }
                        i += 5;
                        continue;
                    }
                    i += 1;
                }
            }

            lines.push(line);

            // advance past the split point
            let start = (split_index + 1).min(remaining.len());
            let mut next = remaining[start..].to_string();

            // apply saved color to the start of the next line
            if let Some(col) = saved_col.clone() {
                if !next.is_empty() && !next.starts_with('|') {
                    if let Some(str_index) = next.find("@str@") {
                        let after = str_index + 5;
                        if next.as_bytes().get(after..after + 5) != Some(&b"@bla@"[..]) {
                            next.insert_str(after, "@bla@");
                        }
                        saved_col = None;
                    } else {
                        next = format!("{col}{next}");
                    }
                }
            }

            remaining = next;
        }

        lines
    }
}

fn decode_draw_width(jag: &JagFile, font: Font) -> [u8; 256] {
    let mut data = jag
        .read(&format!("{}.dat", font.name()))
        .expect("missing font dat");
    let mut index = jag.read("index.dat").expect("missing index.dat");
    index.pos = data.g2() as usize + 4;
    let pal_count = index.g1();
    if pal_count > 0 {
        index.pos += (pal_count as usize - 1) * 3;
    }

    let count = if cfg!(before_274) { 94 } else { 256 };
    let mut advance = [0; 256];
    for slot in &mut advance[..count] {
        index.pos += 2;
        let width = index.g2() as usize;
        let height = index.g2() as usize;
        let order = index.g1();
        let mut mask = vec![0; width * height];
        if order == 0 {
            for pixel in &mut mask {
                *pixel = data.g1();
            }
        } else if order == 1 {
            for x in 0..width {
                for y in 0..height {
                    mask[x + y * width] = data.g1();
                }
            }
        }
        *slot = (width + 2) as u8;
        if !mask.is_empty() {
            let threshold = height / 7;
            for x in [0, width - 1] {
                let edge: u32 = (threshold..height)
                    .map(|y| mask[x + y * width] as u32)
                    .sum();
                if edge <= threshold as u32 {
                    *slot -= 1;
                }
            }
        }
    }

    #[cfg(before_274)]
    {
        advance[94] = advance[8];
        CHAR_LOOKUP.map(|index| advance[index as usize])
    }
    #[cfg(since_274)]
    {
        advance[b' ' as usize] =
            advance[if matches!(font, Font::Q8) { b'I' } else { b'i' } as usize];
        advance
    }
}

pub struct FontTypeProvider {
    pub fonts: Vec<FontType>,
    #[cfg(since_647)]
    pub id_index: HashMap<u16, usize>,
}

impl FontTypeProvider {
    pub fn from_jag(jag_bytes: &[u8]) -> FontTypeProvider {
        let jag = JagFile::from(jag_bytes.to_vec());
        let fonts = Font::ALL
            .iter()
            .map(|&font_id| FontType {
                id: font_id as u8,
                draw_width: decode_draw_width(&jag, font_id),
            })
            .collect();
        FontTypeProvider::from_slots(fonts)
    }

    /// Ordinal-indexed slots only; script font ids are assumed to be ordinals.
    pub fn from_slots(fonts: Vec<FontType>) -> FontTypeProvider {
        FontTypeProvider {
            fonts,
            #[cfg(since_647)]
            id_index: HashMap::new(),
        }
    }

    pub fn get(&self, id: Font) -> Option<&FontType> {
        self.fonts.get(id as usize)
    }

    pub fn get_by_id(&self, id: u16) -> Option<&FontType> {
        // An id map, once populated, is authoritative: falling back to the
        // ordinal would silently hand back the wrong font for any cache id that
        // happens to be small.
        #[cfg(since_647)]
        if !self.id_index.is_empty() {
            return self.id_index.get(&id).and_then(|&i| self.fonts.get(i));
        }
        self.fonts.get(id as usize)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&FontType> {
        self.get(Font::from_config_str(name))
    }

    pub fn count(&self) -> usize {
        self.fonts.len()
    }
}

#[cfg(since_647)]
impl FontTypeProvider {
    pub fn from_cache(cache: &mut Js5Store) -> FontTypeProvider {
        let slot_names: [(&str, Font); 4] = [
            ("p11_full", Font::P11),
            ("p12_full", Font::P12),
            ("b12_full", Font::B12),
            ("q8_full", Font::Q8),
        ];
        let mut fonts: Vec<FontType> = Font::ALL
            .iter()
            .map(|&f| FontType {
                id: f as u8,
                draw_width: [0; 256],
            })
            .collect();
        let mut id_index: HashMap<u16, usize> = HashMap::new();

        let Some(ref_table) = cache.reference_table(archive::FONTMETRICS).cloned() else {
            warn!("No reference table for archive 13 (FONTMETRICS)");
            return FontTypeProvider::from_slots(fonts);
        };

        // Which group backs each ordinal slot, so a script naming one of the four
        // well-known faces resolves to the same FontType the engine uses.
        let mut slot_of_group: HashMap<u32, usize> = HashMap::new();
        for (name, slot) in slot_names {
            let hash = java_string_hash(name);
            if let Some(group) = ref_table.groups.iter().find(|g| g.name_hash == hash) {
                slot_of_group.insert(group.id, slot as usize);
            }
        }

        for group in &ref_table.groups {
            let Ok(Some(g)) = cache.read_group(archive::FONTMETRICS, group.id) else {
                continue;
            };
            let Some(entry) = g.entries.first() else {
                continue;
            };
            // 2-byte header then one advance per code point.
            if entry.data().len() < 258 {
                continue;
            }
            let mut draw_width = [0u8; 256];
            draw_width.copy_from_slice(&entry.data()[2..258]);

            let index = match slot_of_group.get(&group.id) {
                Some(&slot) => {
                    fonts[slot].draw_width = draw_width;
                    slot
                }
                None => {
                    fonts.push(FontType {
                        // FontType::id is a u8 slot tag; cache ids do not fit and are
                        // carried by `id_index` instead.
                        id: 0,
                        draw_width,
                    });
                    fonts.len() - 1
                }
            };
            id_index.insert(group.id as u16, index);
        }

        FontTypeProvider { fonts, id_index }
    }
}
