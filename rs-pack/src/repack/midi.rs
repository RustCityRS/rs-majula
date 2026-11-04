#![allow(unused_parens, clippy::double_parens)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Jagex columnar MIDI codec — decode and encode.
//!
//! Decode: ported from `Linkable_Sub25` in the deobfuscated rev 647 client.
//! The Jagex format stores MIDI events in column-major order with delta
//! encoding. The decoder reconstructs standard MIDI (MThd/MTrk).
//!
//! Encode: exact inverse — parses standard MIDI and writes the columnar
//! representation. Roundtrip: decode(encode(decode(jagex))) == decode(jagex).

use std::path::Path;

use anyhow::{Context, Result, bail};
use rs_dat2::CacheStore;
use rs_dat2::compression::CompressionType;
use tracing::warn;

use crate::asset_roundtrip::AssetManifest;

// ---------------------------------------------------------------------------
// Jagex columnar MIDI decoder (from standard MIDI output)
// ---------------------------------------------------------------------------

struct MidiWriter {
    buf: Vec<u8>,
}

impl MidiWriter {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn p1(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn p2(&mut self, v: u16) {
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
    }

    fn p4(&mut self, v: u32) {
        self.buf.push((v >> 24) as u8);
        self.buf.push((v >> 16) as u8);
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
    }

    fn pvarint(&mut self, mut v: i32) {
        if v < 0 {
            v = 0;
        }
        if v < 128 {
            self.buf.push(v as u8);
        } else {
            let mut bytes = Vec::new();
            bytes.push((v & 0x7F) as u8);
            v >>= 7;
            while v > 0 {
                bytes.push((v & 0x7F | 0x80) as u8);
                v >>= 7;
            }
            bytes.reverse();
            self.buf.extend_from_slice(&bytes);
        }
    }

    fn psize4(&mut self, pos: usize, size: u32) {
        self.buf[pos] = (size >> 24) as u8;
        self.buf[pos + 1] = (size >> 16) as u8;
        self.buf[pos + 2] = (size >> 8) as u8;
        self.buf[pos + 3] = size as u8;
    }

    fn pos(&self) -> usize {
        self.buf.len()
    }
}

fn gvarint(data: &[u8], pos: &mut usize) -> i32 {
    let mut result: i32 = 0;
    loop {
        let b = data[*pos] as i32;
        *pos += 1;
        result = (result << 7) | (b & 0x7F);
        if b & 0x80 == 0 {
            break;
        }
    }
    result
}

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
        gvarint(data, &mut pos);
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

    let mut out = MidiWriter::new();
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
        let size_pos = out.pos();
        out.p4(0);
        let track_start = out.pos();

        let mut prev_status: i32 = -1;

        loop {
            let delta = gvarint(data, &mut c_delta);
            out.pvarint(delta);

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
                let track_size = (out.pos() - track_start) as u32;
                out.psize4(size_pos, track_size);
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
                channel_xor ^= (opcode >> 4);
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

    Ok(out.buf)
}

const ARCHIVE_SONGS: u8 = 6;

// ---------------------------------------------------------------------------
// Standard MIDI parser
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum MidiEventKind {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    PitchBend {
        channel: u8,
        lo: u8,
        hi: u8,
    },
    ChannelPressure {
        channel: u8,
        pressure: u8,
    },
    PolyAftertouch {
        channel: u8,
        note: u8,
        pressure: u8,
    },
    ProgramChange {
        channel: u8,
        program: u8,
    },
    Tempo {
        b1: u8,
        b2: u8,
        b3: u8,
    },
    EndOfTrack,
}

#[derive(Debug, Clone)]
struct MidiEvent {
    delta: u32,
    kind: MidiEventKind,
}

fn read_u16(data: &[u8], pos: &mut usize) -> Result<u16> {
    if *pos + 2 > data.len() {
        bail!("unexpected EOF reading u16 at {}", *pos);
    }
    let v = u16::from_be_bytes([data[*pos], data[*pos + 1]]);
    *pos += 2;
    Ok(v)
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32> {
    if *pos + 4 > data.len() {
        bail!("unexpected EOF reading u32 at {}", *pos);
    }
    let v = u32::from_be_bytes([data[*pos], data[*pos + 1], data[*pos + 2], data[*pos + 3]]);
    *pos += 4;
    Ok(v)
}

fn read_varint(data: &[u8], pos: &mut usize) -> Result<u32> {
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

fn parse_standard_midi(data: &[u8]) -> Result<(u16, Vec<Vec<MidiEvent>>)> {
    let mut pos = 0usize;

    let magic = read_u32(data, &mut pos)?;
    if magic != 0x4D546864 {
        bail!("not a MIDI file (bad MThd magic)");
    }
    let header_len = read_u32(data, &mut pos)?;
    if header_len < 6 {
        bail!("MThd header too short: {}", header_len);
    }
    let _format = read_u16(data, &mut pos)?;
    let num_tracks = read_u16(data, &mut pos)?;
    let division = read_u16(data, &mut pos)?;
    pos = 8 + header_len as usize;

    let mut tracks = Vec::with_capacity(num_tracks as usize);

    for _ in 0..num_tracks {
        let magic = read_u32(data, &mut pos)?;
        if magic != 0x4D54726B {
            bail!("bad MTrk magic at {}", pos - 4);
        }
        let track_len = read_u32(data, &mut pos)? as usize;
        let track_end = pos + track_len;

        let mut events = Vec::new();
        let mut running_status: u8 = 0;

        while pos < track_end {
            let delta = read_varint(data, &mut pos)?;

            if pos >= track_end {
                bail!("unexpected end of track data");
            }

            let status = if data[pos] & 0x80 != 0 {
                let s = data[pos];
                pos += 1;
                running_status = s;
                s
            } else {
                running_status
            };

            let kind = match status & 0xF0 {
                0x80 => {
                    let note = data[pos] & 0x7F;
                    pos += 1;
                    let vel = data[pos] & 0x7F;
                    pos += 1;
                    MidiEventKind::NoteOff {
                        channel: status & 0x0F,
                        note,
                        velocity: vel,
                    }
                }
                0x90 => {
                    let note = data[pos] & 0x7F;
                    pos += 1;
                    let vel = data[pos] & 0x7F;
                    pos += 1;
                    MidiEventKind::NoteOn {
                        channel: status & 0x0F,
                        note,
                        velocity: vel,
                    }
                }
                0xA0 => {
                    let note = data[pos] & 0x7F;
                    pos += 1;
                    let pressure = data[pos] & 0x7F;
                    pos += 1;
                    MidiEventKind::PolyAftertouch {
                        channel: status & 0x0F,
                        note,
                        pressure,
                    }
                }
                0xB0 => {
                    let ctrl = data[pos] & 0x7F;
                    pos += 1;
                    let val = data[pos] & 0x7F;
                    pos += 1;
                    MidiEventKind::ControlChange {
                        channel: status & 0x0F,
                        controller: ctrl,
                        value: val,
                    }
                }
                0xC0 => {
                    let prog = data[pos] & 0x7F;
                    pos += 1;
                    MidiEventKind::ProgramChange {
                        channel: status & 0x0F,
                        program: prog,
                    }
                }
                0xD0 => {
                    let pressure = data[pos] & 0x7F;
                    pos += 1;
                    MidiEventKind::ChannelPressure {
                        channel: status & 0x0F,
                        pressure,
                    }
                }
                0xE0 => {
                    let lo = data[pos] & 0x7F;
                    pos += 1;
                    let hi = data[pos] & 0x7F;
                    pos += 1;
                    MidiEventKind::PitchBend {
                        channel: status & 0x0F,
                        lo,
                        hi,
                    }
                }
                0xF0 if status == 0xFF => {
                    let meta_type = data[pos];
                    pos += 1;
                    let meta_len = read_varint(data, &mut pos)? as usize;
                    match meta_type {
                        0x2F => {
                            pos += meta_len;
                            MidiEventKind::EndOfTrack
                        }
                        0x51 if meta_len == 3 => {
                            let b1 = data[pos];
                            let b2 = data[pos + 1];
                            let b3 = data[pos + 2];
                            pos += 3;
                            MidiEventKind::Tempo { b1, b2, b3 }
                        }
                        _ => {
                            pos += meta_len;
                            continue;
                        }
                    }
                }
                0xF0 if status == 0xF0 || status == 0xF7 => {
                    let sysex_len = read_varint(data, &mut pos)? as usize;
                    pos += sysex_len;
                    continue;
                }
                _ => {
                    bail!("unsupported MIDI status 0x{:02X} at {}", status, pos);
                }
            };

            events.push(MidiEvent { delta, kind });

            if matches!(&events.last().unwrap().kind, MidiEventKind::EndOfTrack) {
                break;
            }
        }

        if !matches!(
            events.last().map(|e| &e.kind),
            Some(MidiEventKind::EndOfTrack)
        ) {
            events.push(MidiEvent {
                delta: 0,
                kind: MidiEventKind::EndOfTrack,
            });
        }

        pos = track_end;
        tracks.push(events);
    }

    Ok((division, tracks))
}

// ---------------------------------------------------------------------------
// Jagex columnar MIDI encoder
// ---------------------------------------------------------------------------

fn write_varint(buf: &mut Vec<u8>, mut v: u32) {
    if v < 128 {
        buf.push(v as u8);
        return;
    }
    let mut bytes = Vec::new();
    bytes.push((v & 0x7F) as u8);
    v >>= 7;
    while v > 0 {
        bytes.push((v & 0x7F | 0x80) as u8);
        v >>= 7;
    }
    bytes.reverse();
    buf.extend_from_slice(&bytes);
}

fn delta_byte(value: i32, accum: &mut i32) -> u8 {
    // The Jagex encoder stores deltas masked to 7 bits (0..127).
    // The decoder interprets stored bytes as `i8` — values 0..127 are
    // non-negative, so the accumulator only grows.  Output is `accum & 0x7F`,
    // giving the correct 7-bit MIDI value through modular arithmetic.
    let stored = (value.wrapping_sub(*accum) & 0x7F) as u8;
    *accum += stored as i32;
    stored
}

pub fn encode_jagex_midi(midi_data: &[u8]) -> Result<Vec<u8>> {
    let (division, tracks) = parse_standard_midi(midi_data)?;
    let num_tracks = tracks.len();
    if num_tracks == 0 || num_tracks > 255 {
        bail!("invalid track count: {}", num_tracks);
    }

    // Output section buffers
    let mut opcodes: Vec<u8> = Vec::new();
    let mut deltas: Vec<u8> = Vec::new();
    let mut cc_nums: Vec<u8> = Vec::new();

    let mut switch_vals: Vec<u8> = Vec::new();
    let mut poly_pressure_vals: Vec<u8> = Vec::new();
    let mut chan_pressure_vals: Vec<u8> = Vec::new();
    let mut pitch_bend_hi_vals: Vec<u8> = Vec::new();
    let mut mod_wheel_vals: Vec<u8> = Vec::new();
    let mut volume_vals: Vec<u8> = Vec::new();
    let mut pan_vals: Vec<u8> = Vec::new();
    let mut note_vel_vals: Vec<u8> = Vec::new(); // note pitches (shared: on+off+poly)
    let mut note_pitch_vals: Vec<u8> = Vec::new(); // note-on velocities
    let mut other_cc_vals: Vec<u8> = Vec::new();
    let mut note_off_vel_vals: Vec<u8> = Vec::new();
    let mut mod_wheel_fine_vals: Vec<u8> = Vec::new();
    let mut volume_fine_vals: Vec<u8> = Vec::new();
    let mut pan_fine_vals: Vec<u8> = Vec::new();
    let mut bank_vals: Vec<u8> = Vec::new(); // CC 0/32 + program change (shared)
    let mut pitch_bend_lo_vals: Vec<u8> = Vec::new();
    let mut nrpn_msb_vals: Vec<u8> = Vec::new();
    let mut nrpn_lsb_vals: Vec<u8> = Vec::new();
    let mut rpn_msb_vals: Vec<u8> = Vec::new();
    let mut rpn_lsb_vals: Vec<u8> = Vec::new();
    let mut tempo_vals: Vec<u8> = Vec::new();

    // Accumulators persist across ALL tracks
    let mut channel_xor: i32 = 0;
    let mut pitch_delta: i32 = 0;
    let mut note_on_vel_delta: i32 = 0;
    let mut note_off_vel_delta: i32 = 0;
    let mut pitch_bend_delta: i32 = 0;
    let mut chan_press_delta: i32 = 0;
    let mut poly_press_delta: i32 = 0;
    let mut cc_val_accum = [0i32; 128];
    let mut cc_num_running: u8 = 0;

    for track in &tracks {
        for event in track {
            write_varint(&mut deltas, event.delta);

            match &event.kind {
                MidiEventKind::EndOfTrack => {
                    opcodes.push(7);
                }
                MidiEventKind::Tempo { b1, b2, b3 } => {
                    opcodes.push(23);
                    tempo_vals.push(*b1);
                    tempo_vals.push(*b2);
                    tempo_vals.push(*b3);
                }
                MidiEventKind::NoteOn {
                    channel,
                    note,
                    velocity,
                } => {
                    let ch = *channel as i32;
                    let ch_delta = channel_xor ^ ch;
                    channel_xor = ch;
                    opcodes.push((ch_delta << 4) as u8);

                    note_vel_vals.push(delta_byte(*note as i32, &mut pitch_delta));
                    note_pitch_vals.push(delta_byte(*velocity as i32, &mut note_on_vel_delta));
                }
                MidiEventKind::NoteOff {
                    channel,
                    note,
                    velocity,
                } => {
                    let ch = *channel as i32;
                    let ch_delta = channel_xor ^ ch;
                    channel_xor = ch;
                    opcodes.push(((ch_delta << 4) | 1) as u8);

                    note_vel_vals.push(delta_byte(*note as i32, &mut pitch_delta));
                    note_off_vel_vals.push(delta_byte(*velocity as i32, &mut note_off_vel_delta));
                }
                MidiEventKind::ControlChange {
                    channel,
                    controller,
                    value,
                } => {
                    let ch = *channel as i32;
                    let ch_delta = channel_xor ^ ch;
                    channel_xor = ch;
                    opcodes.push(((ch_delta << 4) | 2) as u8);

                    let cc = *controller;
                    let cc_stored = cc.wrapping_sub(cc_num_running) & 0x7F;
                    cc_num_running = cc_num_running.wrapping_add(cc_stored) & 0x7F;
                    cc_nums.push(cc_stored);

                    let val_byte = delta_byte(*value as i32, &mut cc_val_accum[cc as usize]);
                    match cc {
                        0 | 32 => bank_vals.push(val_byte),
                        1 => mod_wheel_vals.push(val_byte),
                        33 => mod_wheel_fine_vals.push(val_byte),
                        7 => volume_vals.push(val_byte),
                        39 => volume_fine_vals.push(val_byte),
                        10 => pan_vals.push(val_byte),
                        42 => pan_fine_vals.push(val_byte),
                        99 => nrpn_msb_vals.push(val_byte),
                        98 => nrpn_lsb_vals.push(val_byte),
                        101 => rpn_msb_vals.push(val_byte),
                        100 => rpn_lsb_vals.push(val_byte),
                        64 | 65 | 120 | 121 | 123 => switch_vals.push(val_byte),
                        _ => other_cc_vals.push(val_byte),
                    }
                }
                MidiEventKind::PitchBend { channel, lo, hi } => {
                    let ch = *channel as i32;
                    let ch_delta = channel_xor ^ ch;
                    channel_xor = ch;
                    opcodes.push(((ch_delta << 4) | 3) as u8);

                    let lo_stored = ((*lo as i32).wrapping_sub(pitch_bend_delta) & 0x7F) as u8;
                    pitch_bend_delta += lo_stored as i32;

                    let hi_stored = ((*hi as i32).wrapping_sub(pitch_bend_delta >> 7) & 0x7F) as u8;
                    pitch_bend_delta += (hi_stored as i32) << 7;

                    pitch_bend_lo_vals.push(lo_stored);
                    pitch_bend_hi_vals.push(hi_stored);
                }
                MidiEventKind::ChannelPressure { channel, pressure } => {
                    let ch = *channel as i32;
                    let ch_delta = channel_xor ^ ch;
                    channel_xor = ch;
                    opcodes.push(((ch_delta << 4) | 4) as u8);

                    chan_pressure_vals.push(delta_byte(*pressure as i32, &mut chan_press_delta));
                }
                MidiEventKind::PolyAftertouch {
                    channel,
                    note,
                    pressure,
                } => {
                    let ch = *channel as i32;
                    let ch_delta = channel_xor ^ ch;
                    channel_xor = ch;
                    opcodes.push(((ch_delta << 4) | 5) as u8);

                    note_vel_vals.push(delta_byte(*note as i32, &mut pitch_delta));
                    poly_pressure_vals.push(delta_byte(*pressure as i32, &mut poly_press_delta));
                }
                MidiEventKind::ProgramChange { channel, program } => {
                    let ch = *channel as i32;
                    let ch_delta = channel_xor ^ ch;
                    channel_xor = ch;
                    opcodes.push(((ch_delta << 4) | 6) as u8);

                    bank_vals.push(*program);
                }
            }
        }
    }

    // Concatenate all sections in the exact order the decoder expects
    let mut out = Vec::new();
    out.extend_from_slice(&opcodes);
    out.extend_from_slice(&deltas);
    out.extend_from_slice(&cc_nums);

    out.extend_from_slice(&switch_vals);
    out.extend_from_slice(&poly_pressure_vals);
    out.extend_from_slice(&chan_pressure_vals);
    out.extend_from_slice(&pitch_bend_hi_vals);
    out.extend_from_slice(&mod_wheel_vals);
    out.extend_from_slice(&volume_vals);
    out.extend_from_slice(&pan_vals);
    out.extend_from_slice(&note_vel_vals);
    out.extend_from_slice(&note_pitch_vals);
    out.extend_from_slice(&other_cc_vals);
    out.extend_from_slice(&note_off_vel_vals);
    out.extend_from_slice(&mod_wheel_fine_vals);
    out.extend_from_slice(&volume_fine_vals);
    out.extend_from_slice(&pan_fine_vals);
    out.extend_from_slice(&bank_vals);
    out.extend_from_slice(&pitch_bend_lo_vals);
    out.extend_from_slice(&nrpn_msb_vals);
    out.extend_from_slice(&nrpn_lsb_vals);
    out.extend_from_slice(&rpn_msb_vals);
    out.extend_from_slice(&rpn_lsb_vals);
    out.extend_from_slice(&tempo_vals);

    // Footer: num_tracks (u8) + division (u16 BE)
    out.push(num_tracks as u8);
    out.push((division >> 8) as u8);
    out.push(division as u8);

    Ok(out)
}

// ---------------------------------------------------------------------------
// Repack integration
// ---------------------------------------------------------------------------

pub struct RepackStats {
    pub groups_written: u32,
    pub groups_skipped: u32,
}

pub fn repack_songs(store: &mut CacheStore, content_dir: &Path) -> Result<RepackStats> {
    let archive_dir = content_dir.join("songs").join("_unpack");
    let manifest_path = archive_dir.join("manifest.json");
    let manifest: AssetManifest = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("reading {}", manifest_path.display()))?,
    )?;

    let mut table = store
        .reference_table(ARCHIVE_SONGS)
        .ok_or_else(|| anyhow::anyhow!("no reference table for archive {}", ARCHIVE_SONGS))?
        .clone();

    let mut stats = RepackStats {
        groups_written: 0,
        groups_skipped: 0,
    };

    for entry in &manifest.groups {
        let file_path = if let Some(ref name) = entry.file_name {
            archive_dir.join(name)
        } else {
            archive_dir.join(format!("{}.{}", entry.group_id, manifest.file_extension))
        };

        let midi_bytes = match std::fs::read(&file_path) {
            Ok(b) => b,
            Err(e) => {
                warn!(
                    "song group {}: read failed ({}), skipping",
                    entry.group_id, e
                );
                stats.groups_skipped += 1;
                continue;
            }
        };

        let jagex_bytes = match encode_jagex_midi(&midi_bytes) {
            Ok(b) => b,
            Err(e) => {
                warn!(
                    "song group {}: encode failed ({}), skipping",
                    entry.group_id, e
                );
                stats.groups_skipped += 1;
                continue;
            }
        };

        if entry.compression == 0xFF {
            store
                .write_raw_group(ARCHIVE_SONGS, entry.group_id, &jagex_bytes)
                .with_context(|| format!("write_raw_group songs group {}", entry.group_id))?;
            stats.groups_written += 1;
            continue;
        }

        let comp = CompressionType::try_from(entry.compression)
            .map_err(|_| anyhow::anyhow!("unknown compression {}", entry.compression))?;

        let new_crc = store
            .write_group(
                ARCHIVE_SONGS,
                entry.group_id,
                &jagex_bytes,
                comp,
                entry.version,
            )
            .with_context(|| format!("write_group songs group {}", entry.group_id))?;

        if let Some(g) = table.groups.iter_mut().find(|g| g.id == entry.group_id) {
            g.crc = new_crc;
        }
        stats.groups_written += 1;
    }

    store
        .write_reference_table(ARCHIVE_SONGS, &table)
        .context("write_reference_table for songs")?;

    Ok(stats)
}
