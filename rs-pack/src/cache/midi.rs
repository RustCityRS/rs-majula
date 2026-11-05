#[cfg(since_647)]
use crate::PACK_DIR;
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use anyhow::{Result, bail};
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

#[cfg(since_647)]
pub(crate) fn read_varint(data: &[u8], pos: &mut usize) -> Result<u32> {
    let mut result: u32 = 0;
    loop {
        if *pos >= data.len() {
            bail!("unexpected EOF in varint at {}", *pos);
        }
        let b = data[*pos] as u32;
        *pos += 1;
        result = (result << 7) | (b & 0x7F);
        if b & 0x80 == 0 {
            return Ok(result);
        }
    }
}

#[cfg(since_647)]
pub fn decode_jagex_midi(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 3 {
        bail!("MIDI data too short: {} bytes", data.len());
    }

    let mut pos = data.len() - 3;
    let num_tracks = data[pos] as usize;
    pos += 1;
    let division = u16::from_be_bytes([data[pos], data[pos + 1]]);

    let mut header_size = num_tracks * 10 + 14;
    pos = 0;

    let mut tempo_count = 0usize;
    let mut note_on_count = 0usize;
    let mut note_off_count = 0usize;
    let mut control_count = 0usize;
    let mut pitch_bend_count = 0usize;
    let mut channel_pressure_count = 0usize;
    let mut poly_pressure_count = 0usize;
    let mut program_change_count = 0usize;

    for _ in 0..num_tracks {
        let mut prev: i32 = -1;
        loop {
            let opcode = data[pos] as i32;
            pos += 1;
            if opcode != prev {
                header_size += 1;
            }
            prev = opcode & 0xF;
            if opcode == 7 {
                break;
            }
            match prev {
                23 if opcode == 23 => tempo_count += 1,
                0 => note_on_count += 1,
                1 => note_off_count += 1,
                2 => control_count += 1,
                3 => pitch_bend_count += 1,
                4 => channel_pressure_count += 1,
                5 => poly_pressure_count += 1,
                6 => program_change_count += 1,
                _ => {
                    if opcode == 23 {
                        tempo_count += 1;
                    }
                }
            }
        }
    }

    let delta_size_estimate = header_size
        + tempo_count * 5
        + (note_on_count + note_off_count + control_count + pitch_bend_count + poly_pressure_count)
            * 2
        + channel_pressure_count
        + program_change_count;
    let _ = delta_size_estimate;

    let delta_start = pos;
    let total_events = num_tracks
        + tempo_count
        + control_count
        + note_on_count
        + note_off_count
        + pitch_bend_count
        + channel_pressure_count
        + poly_pressure_count
        + program_change_count;

    for _ in 0..total_events {
        read_varint(data, &mut pos)? as i32;
    }
    let after_deltas = pos;

    let mut controller_zero_or_32 = 0usize;
    let mut controller_1 = 0usize;
    let mut controller_33 = 0usize;
    let mut controller_7 = 0usize;
    let mut controller_39 = 0usize;
    let mut controller_10 = 0usize;
    let mut controller_42 = 0usize;
    let mut controller_99 = 0usize;
    let mut controller_98 = 0usize;
    let mut controller_101 = 0usize;
    let mut controller_100 = 0usize;
    let mut controller_switch = 0usize;
    let mut controller_other = 0usize;

    let mut cc_num: u8 = 0;
    for _ in 0..control_count {
        cc_num = cc_num.wrapping_add(data[pos]) & 0x7F;
        pos += 1;
        match cc_num {
            0 | 32 => controller_zero_or_32 += 1,
            1 => controller_1 += 1,
            33 => controller_33 += 1,
            7 => controller_7 += 1,
            39 => controller_39 += 1,
            10 => controller_10 += 1,
            42 => controller_42 += 1,
            99 => controller_99 += 1,
            98 => controller_98 += 1,
            101 => controller_101 += 1,
            100 => controller_100 += 1,
            64 | 65 | 120 | 121 | 123 => controller_switch += 1,
            _ => controller_other += 1,
        }
    }

    let mut section_pos = pos;
    let switch_off = section_pos;
    section_pos += controller_switch;
    let poly_pressure_off = section_pos;
    section_pos += poly_pressure_count;
    let chan_pressure_off = section_pos;
    section_pos += channel_pressure_count;
    let pitch_bend_hi_off = section_pos;
    section_pos += pitch_bend_count;
    let mod_wheel_off = section_pos;
    section_pos += controller_1;
    let volume_off = section_pos;
    section_pos += controller_7;
    let pan_off = section_pos;
    section_pos += controller_10;
    let note_vel_off = section_pos;
    section_pos += note_on_count + note_off_count + poly_pressure_count;
    let note_pitch_off = section_pos;
    section_pos += note_on_count;
    let other_cc_off = section_pos;
    section_pos += controller_other;
    let note_off_vel_off = section_pos;
    section_pos += note_off_count;
    let mod_wheel_fine_off = section_pos;
    section_pos += controller_33;
    let volume_fine_off = section_pos;
    section_pos += controller_39;
    let pan_fine_off = section_pos;
    section_pos += controller_42;
    let bank_off = section_pos;
    section_pos += controller_zero_or_32 + program_change_count;
    let pitch_bend_lo_off = section_pos;
    section_pos += pitch_bend_count;
    let nrpn_msb_off = section_pos;
    section_pos += controller_99;
    let nrpn_lsb_off = section_pos;
    section_pos += controller_98;
    let rpn_msb_off = section_pos;
    section_pos += controller_101;
    let rpn_lsb_off = section_pos;
    section_pos += controller_100;
    let tempo_off = section_pos;

    let mut out = PackedData::default();
    out.p4(0x4D546864);
    out.p4(6);
    out.p2(if num_tracks > 1 { 1 } else { 0 });
    out.p2(num_tracks as u16);
    out.p2(division);

    let mut c_opcode = 0usize;
    let mut c_delta = delta_start;
    let mut c_cc_num = after_deltas;
    let mut c_switch = switch_off;
    let mut c_poly_pressure = poly_pressure_off;
    let mut c_chan_pressure = chan_pressure_off;
    let mut c_pitch_bend_hi = pitch_bend_hi_off;
    let mut c_mod_wheel = mod_wheel_off;
    let mut c_volume = volume_off;
    let mut c_pan = pan_off;
    let mut c_note_vel = note_vel_off;
    let mut c_note_pitch = note_pitch_off;
    let mut c_other_cc = other_cc_off;
    let mut c_note_off_vel = note_off_vel_off;
    let mut c_mod_wheel_fine = mod_wheel_fine_off;
    let mut c_volume_fine = volume_fine_off;
    let mut c_pan_fine = pan_fine_off;
    let mut c_bank = bank_off;
    let mut c_pitch_bend_lo = pitch_bend_lo_off;
    let mut c_nrpn_msb = nrpn_msb_off;
    let mut c_nrpn_lsb = nrpn_lsb_off;
    let mut c_rpn_msb = rpn_msb_off;
    let mut c_rpn_lsb = rpn_lsb_off;
    let mut c_tempo = tempo_off;

    let mut channel_xor: i32 = 0;
    let mut pitch_delta: i32 = 0;
    let mut note_on_vel_delta: i32 = 0;
    let mut note_off_vel_delta: i32 = 0;
    let mut pitch_bend_delta: i32 = 0;
    let mut chan_press_delta: i32 = 0;
    let mut poly_press_delta: i32 = 0;
    let mut cc_val_accum = [0i32; 128];
    let mut cc_num_running: u8 = 0;

    for _ in 0..num_tracks {
        out.p4(0x4D54726B);
        let size_pos = out.dat.len();
        out.p4(0);
        let track_start = out.dat.len();

        let mut prev_status: i32 = -1;

        loop {
            let delta = read_varint(data, &mut c_delta)? as i32;
            out.pvarint(delta.max(0) as u32);

            let opcode = data[c_opcode] as i32;
            c_opcode += 1;
            let is_new_status = opcode != prev_status;
            prev_status = opcode & 0xF;

            if opcode == 7 {
                if is_new_status {
                    out.p1(0xFF);
                }
                out.p1(0x2F);
                out.p1(0x00);
                let track_size = (out.dat.len() - track_start) as u32;
                out.dat[size_pos..size_pos + 4].copy_from_slice(&track_size.to_be_bytes());
                break;
            }

            if opcode == 23 {
                if is_new_status {
                    out.p1(0xFF);
                }
                out.p1(0x51);
                out.p1(0x03);
                out.p1(data[c_tempo]);
                c_tempo += 1;
                out.p1(data[c_tempo]);
                c_tempo += 1;
                out.p1(data[c_tempo]);
                c_tempo += 1;
            } else {
                channel_xor ^= opcode >> 4;
                let event_type = opcode & 0xF;

                match event_type {
                    0 => {
                        if is_new_status {
                            out.p1((channel_xor + 144) as u8);
                        }
                        pitch_delta = pitch_delta.wrapping_add(data[c_note_vel] as i8 as i32);
                        c_note_vel += 1;
                        note_on_vel_delta =
                            note_on_vel_delta.wrapping_add(data[c_note_pitch] as i8 as i32);
                        c_note_pitch += 1;
                        out.p1((pitch_delta & 0x7F) as u8);
                        out.p1((note_on_vel_delta & 0x7F) as u8);
                    }
                    1 => {
                        if is_new_status {
                            out.p1((channel_xor + 128) as u8);
                        }
                        pitch_delta = pitch_delta.wrapping_add(data[c_note_vel] as i8 as i32);
                        c_note_vel += 1;
                        note_off_vel_delta =
                            note_off_vel_delta.wrapping_add(data[c_note_off_vel] as i8 as i32);
                        c_note_off_vel += 1;
                        out.p1((pitch_delta & 0x7F) as u8);
                        out.p1((note_off_vel_delta & 0x7F) as u8);
                    }
                    2 => {
                        if is_new_status {
                            out.p1((channel_xor + 176) as u8);
                        }
                        cc_num_running = cc_num_running.wrapping_add(data[c_cc_num]) & 0x7F;
                        c_cc_num += 1;
                        out.p1(cc_num_running);

                        let cc_val_byte: i8 = match cc_num_running {
                            0 | 32 => {
                                let v = data[c_bank] as i8;
                                c_bank += 1;
                                v
                            }
                            1 => {
                                let v = data[c_mod_wheel] as i8;
                                c_mod_wheel += 1;
                                v
                            }
                            33 => {
                                let v = data[c_mod_wheel_fine] as i8;
                                c_mod_wheel_fine += 1;
                                v
                            }
                            7 => {
                                let v = data[c_volume] as i8;
                                c_volume += 1;
                                v
                            }
                            39 => {
                                let v = data[c_volume_fine] as i8;
                                c_volume_fine += 1;
                                v
                            }
                            10 => {
                                let v = data[c_pan] as i8;
                                c_pan += 1;
                                v
                            }
                            42 => {
                                let v = data[c_pan_fine] as i8;
                                c_pan_fine += 1;
                                v
                            }
                            99 => {
                                let v = data[c_nrpn_msb] as i8;
                                c_nrpn_msb += 1;
                                v
                            }
                            98 => {
                                let v = data[c_nrpn_lsb] as i8;
                                c_nrpn_lsb += 1;
                                v
                            }
                            101 => {
                                let v = data[c_rpn_msb] as i8;
                                c_rpn_msb += 1;
                                v
                            }
                            100 => {
                                let v = data[c_rpn_lsb] as i8;
                                c_rpn_lsb += 1;
                                v
                            }
                            64 | 65 | 120 | 121 | 123 => {
                                let v = data[c_switch] as i8;
                                c_switch += 1;
                                v
                            }
                            _ => {
                                let v = data[c_other_cc] as i8;
                                c_other_cc += 1;
                                v
                            }
                        };
                        let accumulated =
                            cc_val_byte as i32 + cc_val_accum[cc_num_running as usize];
                        cc_val_accum[cc_num_running as usize] = accumulated;
                        out.p1((accumulated & 0x7F) as u8);
                    }
                    3 => {
                        if is_new_status {
                            out.p1((channel_xor + 224) as u8);
                        }
                        pitch_bend_delta =
                            pitch_bend_delta.wrapping_add(data[c_pitch_bend_lo] as i8 as i32);
                        c_pitch_bend_lo += 1;
                        pitch_bend_delta = pitch_bend_delta
                            .wrapping_add((data[c_pitch_bend_hi] as i8 as i32) << 7);
                        c_pitch_bend_hi += 1;
                        out.p1((pitch_bend_delta & 0x7F) as u8);
                        out.p1((pitch_bend_delta >> 7 & 0x7F) as u8);
                    }
                    4 => {
                        if is_new_status {
                            out.p1((channel_xor + 208) as u8);
                        }
                        chan_press_delta =
                            chan_press_delta.wrapping_add(data[c_chan_pressure] as i8 as i32);
                        c_chan_pressure += 1;
                        out.p1((chan_press_delta & 0x7F) as u8);
                    }
                    5 => {
                        if is_new_status {
                            out.p1((channel_xor + 160) as u8);
                        }
                        pitch_delta = pitch_delta.wrapping_add(data[c_note_vel] as i8 as i32);
                        c_note_vel += 1;
                        poly_press_delta =
                            poly_press_delta.wrapping_add(data[c_poly_pressure] as i8 as i32);
                        c_poly_pressure += 1;
                        out.p1((pitch_delta & 0x7F) as u8);
                        out.p1((poly_press_delta & 0x7F) as u8);
                    }
                    6 => {
                        if is_new_status {
                            out.p1((channel_xor + 192) as u8);
                        }
                        out.p1(data[c_bank]);
                        c_bank += 1;
                    }
                    _ => {
                        bail!("unknown MIDI event type {}", event_type);
                    }
                }
            }
        }
    }

    Ok(out.dat)
}
