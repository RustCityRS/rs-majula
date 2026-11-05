#[cfg(since_647)]
use crate::{PACK_DIR, repack::midi::decode_jagex_midi};
#[cfg(since_647)]
use rs_dat2::{CacheStore, archive};
use rs_io::bz2::bz2_decompress;
use rs_io::crc::getcrc;
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(since_647)]
use std::{fs, path::Path};
#[cfg(since_647)]
use tracing::info;

pub struct MidiType {
    pub name: Box<str>,
    pub data: Arc<[u8]>,
    pub crc: i32,
    pub length_ms: u32,
}

impl MidiType {
    pub fn tick_length(&self) -> u32 {
        (self.length_ms as f64 / 600.0).ceil() as u32 + 1
    }
}

pub struct MidiProvider {
    pub names: HashMap<Box<str>, usize>,
    pub midis: Box<[MidiType]>,
}

impl MidiProvider {
    pub fn from_compressed(entries: HashMap<String, Vec<u8>>) -> MidiProvider {
        let mut names = HashMap::new();
        let mut midis = Vec::new();

        for (name, compressed) in entries {
            let crc = getcrc(&compressed, 0, compressed.len());
            let raw = decompress_song(&compressed);
            let length_ms = parse_midi_length(&raw).unwrap_or(0);
            let id = midis.len();
            let name = name.into_boxed_str();
            names.insert(name.clone(), id);
            midis.push(MidiType {
                name,
                data: Arc::from(compressed),
                crc,
                length_ms,
            });
        }

        MidiProvider {
            names,
            midis: Box::from(midis),
        }
    }

    pub fn get(&self, id: usize) -> Option<&MidiType> {
        self.midis.get(id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&MidiType> {
        self.names.get(name).and_then(|&id| self.midis.get(id))
    }

    pub fn count(&self) -> usize {
        self.midis.len()
    }
}

fn decompress_song(data: &[u8]) -> Vec<u8> {
    if data.len() < 4 {
        return Vec::new();
    }
    let size = ((data[0] as usize) << 24)
        | ((data[1] as usize) << 16)
        | ((data[2] as usize) << 8)
        | (data[3] as usize);
    bz2_decompress(&data[4..], size, true, 0)
}

fn read_u16_be(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16) << 8) | data[offset + 1] as u16
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    ((data[offset] as u32) << 24)
        | ((data[offset + 1] as u32) << 16)
        | ((data[offset + 2] as u32) << 8)
        | data[offset + 3] as u32
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    (data[offset] as u32)
        | ((data[offset + 1] as u32) << 8)
        | ((data[offset + 2] as u32) << 16)
        | ((data[offset + 3] as u32) << 24)
}

fn read_chunk_id(data: &[u8], offset: usize) -> &[u8] {
    &data[offset..offset + 4]
}

fn read_var_len(data: &[u8], mut offset: usize, limit: usize) -> Option<(u32, usize)> {
    let mut value: u32 = 0;
    for _ in 0..4 {
        if offset >= limit {
            return None;
        }
        let byte = data[offset];
        offset += 1;
        value = (value << 7) | (byte & 0x7F) as u32;
        if byte & 0x80 == 0 {
            return Some((value, offset));
        }
    }
    None
}

fn unwrap_riff_midi(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 12 {
        return Some(data);
    }

    if read_chunk_id(data, 0) != b"RIFF" {
        return Some(data);
    }

    if read_chunk_id(data, 8) != b"RMID" {
        return None;
    }

    let mut offset = 12;
    while offset + 8 <= data.len() {
        let id = read_chunk_id(data, offset);
        let size = read_u32_le(data, offset + 4) as usize;
        offset += 8;

        if offset + size > data.len() {
            return None;
        }

        if id == b"data" {
            return Some(&data[offset..offset + size]);
        }

        offset += size + (size % 2);
    }

    None
}

struct TempoEvent {
    tick: u32,
    tempo: u32,
    order: u32,
}

pub(crate) fn parse_midi_length(src: &[u8]) -> Option<u32> {
    let data = unwrap_riff_midi(src)?;

    if data.len() < 14 {
        return None;
    }

    let mut offset = 0;
    if read_chunk_id(data, offset) != b"MThd" {
        return None;
    }

    let header_length = read_u32_be(data, offset + 4) as usize;
    offset += 8;
    if header_length < 6 || offset + header_length > data.len() {
        return None;
    }

    let format = read_u16_be(data, offset);
    let track_count = read_u16_be(data, offset + 2);
    let division = read_u16_be(data, offset + 4);
    offset += header_length;

    if format > 2 || track_count == 0 {
        return None;
    }

    let mut max_tick: u32 = 0;
    let mut tempos: Vec<TempoEvent> = Vec::new();
    let mut tempo_order: u32 = 0;

    for _ in 0..track_count {
        if offset + 8 > data.len() {
            return None;
        }

        if read_chunk_id(data, offset) != b"MTrk" {
            return None;
        }

        let track_length = read_u32_be(data, offset + 4) as usize;
        offset += 8;
        let track_end = offset + track_length;
        if track_end > data.len() {
            return None;
        }

        let mut tick: u32 = 0;
        let mut running_status: u8 = 0;

        while offset < track_end {
            let (delta, new_offset) = read_var_len(data, offset, track_end)?;
            tick += delta;
            offset = new_offset;

            if offset >= track_end {
                break;
            }

            let mut status = data[offset];
            if status < 0x80 {
                if running_status == 0 {
                    return None;
                }
                status = running_status;
            } else {
                offset += 1;
                // At 647 a meta event sets the running status too: the 647
                // client's MIDI (rebuilt from Jagex's columns, `Linkable_Sub25`)
                // writes a meta event's next data bytes under it. A standard
                // file never leans on it, a meta event cancelling running status.
                if status < 0xF0 || (cfg!(since_647) && status == 0xFF) {
                    running_status = status;
                }
            }

            if status == 0xFF {
                if offset >= track_end {
                    return None;
                }

                let meta_type = data[offset];
                offset += 1;
                let (meta_length, new_offset) = read_var_len(data, offset, track_end)?;
                let meta_length = meta_length as usize;
                offset = new_offset;

                if offset + meta_length > track_end {
                    return None;
                }

                if meta_type == 0x51 && meta_length == 3 {
                    let tempo = ((data[offset] as u32) << 16)
                        | ((data[offset + 1] as u32) << 8)
                        | data[offset + 2] as u32;
                    tempos.push(TempoEvent {
                        tick,
                        tempo,
                        order: tempo_order,
                    });
                    tempo_order += 1;
                }

                offset += meta_length;

                if meta_type == 0x2F {
                    offset = track_end;
                    break;
                }
            } else if status == 0xF0 || status == 0xF7 {
                let (sysex_length, new_offset) = read_var_len(data, offset, track_end)?;
                offset = new_offset + sysex_length as usize;
                if offset > track_end {
                    return None;
                }
            } else if status >= 0xF0 {
                let data_bytes: usize = match status {
                    0xF1 | 0xF3 => 1,
                    0xF2 => 2,
                    _ => 0,
                };
                offset += data_bytes;
                if offset > track_end {
                    return None;
                }
            } else {
                let msg_type = status & 0xF0;
                let data_bytes: usize = if msg_type == 0xC0 || msg_type == 0xD0 {
                    1
                } else {
                    2
                };
                offset += data_bytes;
                if offset > track_end {
                    return None;
                }
            }
        }

        if tick > max_tick {
            max_tick = tick;
        }
    }

    if division & 0x8000 != 0 {
        let smpte = (division >> 8) & 0xFF;
        let frames_per_second = 0x100 - smpte;
        let ticks_per_frame = division & 0xFF;
        let ticks_per_second = (frames_per_second as u32) * (ticks_per_frame as u32);
        let ms = if ticks_per_second > 0 {
            ((max_tick as u64 * 1000 + ticks_per_second as u64 / 2) / ticks_per_second as u64)
                as u32
        } else {
            0
        };
        return Some(ms);
    }

    let ppq = if division == 0 { 1u32 } else { division as u32 };
    tempos.sort_by(|a, b| a.tick.cmp(&b.tick).then(a.order.cmp(&b.order)));

    let mut current_tempo: u64 = 500_000;
    let mut last_tick: u32 = 0;
    let mut total_us: u64 = 0;

    for event in &tempos {
        if event.tick < last_tick {
            continue;
        }
        let delta_ticks = (event.tick - last_tick) as u64;
        total_us += delta_ticks * current_tempo / ppq as u64;
        current_tempo = event.tempo as u64;
        last_tick = event.tick;
    }

    if max_tick > last_tick {
        total_us += (max_tick - last_tick) as u64 * current_tempo / ppq as u64;
    }

    Some((total_us / 1000) as u32)
}

#[cfg(since_647)]
pub const MIDI_JINGLE_BASE: usize = 10000;

#[cfg(since_647)]
pub(super) fn load_midi_ids() -> HashMap<Box<str>, u16> {
    let mut ids = HashMap::new();
    for stem in ["songs", "jingles"] {
        let path = Path::new(PACK_DIR).join(format!("{stem}.pack"));
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let Some((id, name)) = line.split_once('=') else {
                continue;
            };
            if let Ok(id) = id.trim().parse::<u16>() {
                ids.insert(name.trim().into(), id);
            }
        }
    }
    ids
}

#[cfg(since_647)]
pub(super) fn load_midi_tick_lengths(cache: &mut CacheStore) -> Box<[Option<u16>]> {
    let mut lengths: Vec<Option<u16>> = Vec::new();
    for (archive, base) in [(archive::SONGS, 0), (archive::JINGLES, MIDI_JINGLE_BASE)] {
        let Some(table) = cache.reference_table(archive).cloned() else {
            continue;
        };
        for group in &table.groups {
            let Ok(Some(g)) = cache.read_group(archive, group.id) else {
                continue;
            };
            let Some(ms) = g
                .entries
                .first()
                .and_then(|e| decode_jagex_midi(&e.data).ok())
                .and_then(|smf| parse_midi_length(&smf))
            else {
                continue;
            };
            let i = base + group.id as usize;
            if i >= lengths.len() {
                lengths.resize(i + 1, None);
            }
            lengths[i] = Some(((ms as f64 / 600.0).ceil() as u32 + 1) as u16);
        }
    }
    info!(
        "Loaded {} song and jingle lengths",
        lengths.iter().flatten().count()
    );
    lengths.into_boxed_slice()
}
