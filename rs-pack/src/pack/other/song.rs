#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, insert_archive, load_group_files, load_name_hashes};
#[cfg(since_647)]
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use anyhow::{Result, bail, ensure};
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::bz2::bz2_compress_with_size;
#[cfg(since_647)]
use rs_io::js5::reference::FileInfo;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::{info, warn};

#[cfg(before_647)]
pub fn pack_midis(content_dir: &Path, kind: &str) -> HashMap<String, Vec<u8>> {
    let songs_dir = content_dir.join(kind);
    let mut result = HashMap::new();

    if !songs_dir.exists() {
        return result;
    }

    let Ok(entries) = fs::read_dir(&songs_dir) else {
        return result;
    };

    let mut files: Vec<_> = entries.flatten().collect();
    files.sort_by_key(|e| e.file_name());

    for entry in files {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("mid"))
            && let Some(name) = path.file_stem().and_then(|n| n.to_str())
        {
            if result.contains_key(name) {
                continue;
            }
            if let Ok(data) = fs::read(&path) {
                result.insert(name.to_string(), bz2_compress_with_size(&data));
            }
        }
    }

    debug!("Packed {} {kind}", result.len());
    result
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let aid = archive::SONGS;
    let format = archive::archive_format(aid).unwrap();
    let unpack_dir = content_dir.join(format.content_dir).join("_unpack");

    if !unpack_dir.is_dir() {
        info!(
            "  archive {:>2} ({}) -- no _unpack dir, skipping",
            aid, format.content_dir
        );
        return Ok(());
    }

    let group_files = load_group_files(&unpack_dir, "mid");

    let name_hashes: HashMap<u32, i32> = if format.named {
        let gids: Vec<u32> = group_files.iter().map(|(gid, _)| *gid).collect();
        load_name_hashes(pack_dir, aid, &gids)
    } else {
        HashMap::new()
    };

    let prepared: Vec<PreparedGroup> = group_files
        .par_iter()
        .filter_map(|(gid, path)| {
            let midi_bytes = fs::read(path).ok()?;
            match encode_jagex_midi(&midi_bytes) {
                Ok(b) => Some((*gid, b)),
                Err(e) => {
                    warn!("song group {}: encode failed ({}), skipping", gid, e);
                    None
                }
            }
        })
        .map(|(gid, data)| {
            let mut group = PreparedGroup::new(aid, gid, &data, versions)?;
            group.name_hash = name_hashes.get(&gid).copied().unwrap_or(-1);
            group.files = vec![FileInfo {
                id: 0,
                name_hash: None,
            }];
            Ok(group)
        })
        .collect::<Result<Vec<_>>>()?;

    insert_archive(store, aid, format, versions, prepared, "from .mid encode")?;

    Ok(())
}

#[cfg(since_647)]
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

#[cfg(since_647)]
#[derive(Debug, Clone)]
struct MidiEvent {
    delta: u32,
    kind: MidiEventKind,
}

#[cfg(since_647)]
fn parse_standard_midi(data: &[u8]) -> Result<(u16, Vec<Vec<MidiEvent>>)> {
    ensure!(data.len() >= 14, "MIDI header too short");
    let mut buf = Packet::from(data);

    let magic = buf.g4s() as u32;
    if magic != 0x4D546864 {
        bail!("not a MIDI file (bad MThd magic)");
    }
    let header_len = buf.g4s() as u32;
    if header_len < 6 {
        bail!("MThd header too short: {}", header_len);
    }
    let _format = buf.g2();
    let num_tracks = buf.g2();
    let division = buf.g2();
    buf.pos = 8 + header_len as usize;

    let mut tracks = Vec::with_capacity(num_tracks as usize);

    for _ in 0..num_tracks {
        ensure!(buf.pos + 8 <= data.len(), "truncated MIDI track header");
        let magic = buf.g4s() as u32;
        if magic != 0x4D54726B {
            bail!("bad MTrk magic at {}", buf.pos - 4);
        }
        let track_len = buf.g4s() as u32 as usize;
        let track_end = buf.pos + track_len;
        ensure!(track_end <= data.len(), "truncated MIDI track");

        let mut events = Vec::new();
        let mut running_status: u8 = 0;

        while buf.pos < track_end {
            ensure!(
                data[buf.pos..track_end].iter().any(|byte| byte & 0x80 == 0),
                "truncated MIDI delta"
            );
            let delta = buf.gmidivarlen();

            if buf.pos >= track_end {
                bail!("unexpected end of track data");
            }

            let status = if data[buf.pos] & 0x80 != 0 {
                let s = data[buf.pos];
                buf.pos += 1;
                running_status = s;
                s
            } else {
                running_status
            };

            let kind = match status & 0xF0 {
                0x80 => {
                    let note = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    let vel = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    MidiEventKind::NoteOff {
                        channel: status & 0x0F,
                        note,
                        velocity: vel,
                    }
                }
                0x90 => {
                    let note = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    let vel = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    MidiEventKind::NoteOn {
                        channel: status & 0x0F,
                        note,
                        velocity: vel,
                    }
                }
                0xA0 => {
                    let note = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    let pressure = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    MidiEventKind::PolyAftertouch {
                        channel: status & 0x0F,
                        note,
                        pressure,
                    }
                }
                0xB0 => {
                    let ctrl = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    let val = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    MidiEventKind::ControlChange {
                        channel: status & 0x0F,
                        controller: ctrl,
                        value: val,
                    }
                }
                0xC0 => {
                    let prog = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    MidiEventKind::ProgramChange {
                        channel: status & 0x0F,
                        program: prog,
                    }
                }
                0xD0 => {
                    let pressure = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    MidiEventKind::ChannelPressure {
                        channel: status & 0x0F,
                        pressure,
                    }
                }
                0xE0 => {
                    let lo = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    let hi = data[buf.pos] & 0x7F;
                    buf.pos += 1;
                    MidiEventKind::PitchBend {
                        channel: status & 0x0F,
                        lo,
                        hi,
                    }
                }
                0xF0 if status == 0xFF => {
                    let meta_type = data[buf.pos];
                    buf.pos += 1;
                    ensure!(
                        data[buf.pos..track_end].iter().any(|byte| byte & 0x80 == 0),
                        "truncated MIDI event length"
                    );
                    let meta_len = buf.gmidivarlen() as usize;
                    match meta_type {
                        0x2F => {
                            buf.pos += meta_len;
                            MidiEventKind::EndOfTrack
                        }
                        0x51 if meta_len == 3 => {
                            let b1 = data[buf.pos];
                            let b2 = data[buf.pos + 1];
                            let b3 = data[buf.pos + 2];
                            buf.pos += 3;
                            MidiEventKind::Tempo { b1, b2, b3 }
                        }
                        _ => {
                            buf.pos += meta_len;
                            continue;
                        }
                    }
                }
                0xF0 if status == 0xF0 || status == 0xF7 => {
                    ensure!(
                        data[buf.pos..track_end].iter().any(|byte| byte & 0x80 == 0),
                        "truncated MIDI event length"
                    );
                    let sysex_len = buf.gmidivarlen() as usize;
                    buf.pos += sysex_len;
                    continue;
                }
                _ => {
                    bail!("unsupported MIDI status 0x{:02X} at {}", status, buf.pos);
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

        buf.pos = track_end;
        tracks.push(events);
    }

    Ok((division, tracks))
}

#[cfg(since_647)]
fn delta_byte(value: i32, accum: &mut i32) -> u8 {
    let stored = (value.wrapping_sub(*accum) & 0x7F) as u8;
    *accum += stored as i32;
    stored
}

#[cfg(since_647)]
pub fn encode_jagex_midi(midi_data: &[u8]) -> Result<Vec<u8>> {
    let (division, tracks) = parse_standard_midi(midi_data)?;
    let num_tracks = tracks.len();
    if num_tracks == 0 || num_tracks > 255 {
        bail!("invalid track count: {}", num_tracks);
    }

    let mut opcodes: Vec<u8> = Vec::new();
    let mut deltas = PackedData::default();
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
            deltas.pvarint(event.delta);

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

    let mut out = Vec::new();
    out.extend_from_slice(&opcodes);
    out.extend_from_slice(&deltas.dat);
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

    out.push(num_tracks as u8);
    out.push((division >> 8) as u8);
    out.push(division as u8);

    Ok(out)
}
