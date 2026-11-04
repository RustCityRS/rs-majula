#![allow(clippy::type_complexity)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! SeqType — CacheType implementation for animation sequence definitions.
//!
//! Decoded from archive 20, rev 647. Mirrors the client's `SeqType.decode`.

use rs_io::Packet;

use super::provider::CacheType;

/// One frame group from opcode 1.
#[derive(Debug, Clone)]
pub struct SeqFrames {
    pub durations: Box<[u16]>,
    pub frame_lo: Box<[u16]>,
    pub frame_hi: Box<[u16]>,
}

/// Synth entry from opcode 13.
#[derive(Debug, Clone)]
pub struct SeqSynth {
    /// First word (g3).
    pub head: i32,
    /// Remaining `size - 1` words.
    pub tail: Box<[u16]>,
}

/// Animation sequence definition.
#[derive(Debug, Clone)]
pub struct SeqType {
    pub id: u16,
    pub priority: u8,
    pub loop_offset: i16,
    pub frame_count: u16,
    pub total_duration: u32,
    pub interrupt_type: i8,
    pub has_interrupt_type: bool,
    pub loop_count: u8,
    pub move_type: i8,
    pub has_move_type: bool,
    pub loop_type: u8,
    pub a_bool9: bool,
    pub tween: bool,
    pub a_bool8: bool,
    pub a_bool10: bool,
    pub op6: i32,
    pub op7: i32,
    pub frames: Option<SeqFrames>,
    pub iframes: Option<(Box<[u16]>, Box<[u16]>)>,
    pub synths: Option<Box<[Option<SeqSynth>]>>,
    pub sound_mask: Option<Box<[u8]>>,
    has_sound_mask: bool,
    /// Op 19 entries: (idx, val) pairs.
    pub op19: Vec<(u8, u8)>,
    /// Op 20 entries: (idx, a, b) triples.
    pub op20: Vec<(u8, u16, u16)>,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
    /// Raw opcode payloads recorded during decode, for faithful re-emission of
    /// duplicate opcodes whose earlier values were overwritten in the struct.
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
}

impl CacheType for SeqType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            priority: 5,
            loop_offset: -1,
            frame_count: 0,
            total_duration: 0,
            interrupt_type: -1,
            has_interrupt_type: false,
            loop_count: 99,
            move_type: -1,
            has_move_type: false,
            loop_type: 2,
            a_bool9: false,
            tween: false,
            a_bool8: false,
            a_bool10: false,
            op6: -1,
            op7: -1,
            frames: None,
            iframes: None,
            synths: None,
            sound_mask: None,
            has_sound_mask: false,
            op19: Vec::new(),
            op20: Vec::new(),
            opcode_order: Vec::new(),
            raw_opcodes: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            self.opcode_order.push(opcode);
            let raw_start = buf.pos;
            match opcode {
                1 => {
                    let len = buf.g2() as usize;
                    let mut durations = Vec::with_capacity(len);
                    let mut total: u32 = 0;
                    for _ in 0..len {
                        let d = buf.g2();
                        total += d as u32;
                        durations.push(d);
                    }
                    let mut frame_lo = Vec::with_capacity(len);
                    for _ in 0..len {
                        frame_lo.push(buf.g2());
                    }
                    let mut frame_hi = Vec::with_capacity(len);
                    for _ in 0..len {
                        frame_hi.push(buf.g2());
                    }
                    self.frame_count = len as u16;
                    self.total_duration = total;
                    self.frames = Some(SeqFrames {
                        durations: durations.into_boxed_slice(),
                        frame_lo: frame_lo.into_boxed_slice(),
                        frame_hi: frame_hi.into_boxed_slice(),
                    });
                }
                2 => self.loop_offset = buf.g2() as i16,
                3 => {
                    let count = buf.g1() as usize;
                    let mut idxs = Vec::with_capacity(count);
                    for _ in 0..count {
                        idxs.push(buf.g1());
                    }
                    self.sound_mask = Some(idxs.into_boxed_slice());
                    self.has_sound_mask = true;
                }
                5 => self.priority = buf.g1(),
                6 => self.op6 = buf.g2() as i32,
                7 => self.op7 = buf.g2() as i32,
                8 => self.loop_count = buf.g1(),
                9 => {
                    self.interrupt_type = buf.g1s();
                    self.has_interrupt_type = true;
                }
                10 => {
                    self.move_type = buf.g1() as i8;
                    self.has_move_type = true;
                }
                11 => self.loop_type = buf.g1(),
                12 => {
                    let len = buf.g1() as usize;
                    let mut frame_lo = Vec::with_capacity(len);
                    for _ in 0..len {
                        frame_lo.push(buf.g2());
                    }
                    let mut frame_hi = Vec::with_capacity(len);
                    for _ in 0..len {
                        frame_hi.push(buf.g2());
                    }
                    self.iframes = Some((frame_lo.into_boxed_slice(), frame_hi.into_boxed_slice()));
                }
                13 => {
                    let len = buf.g2() as usize;
                    let mut synths = Vec::with_capacity(len);
                    for _ in 0..len {
                        let size = buf.g1() as usize;
                        if size > 0 {
                            let head = buf.g3();
                            let mut tail = Vec::with_capacity(size - 1);
                            for _ in 0..(size - 1) {
                                tail.push(buf.g2());
                            }
                            synths.push(Some(SeqSynth {
                                head,
                                tail: tail.into_boxed_slice(),
                            }));
                        } else {
                            synths.push(None);
                        }
                    }
                    self.synths = Some(synths.into_boxed_slice());
                }
                14 => self.a_bool9 = true,
                15 => self.tween = true,
                16 => self.a_bool8 = true,
                18 => self.a_bool10 = true,
                19 => {
                    let idx = buf.g1();
                    let val = buf.g1();
                    self.op19.push((idx, val));
                }
                20 => {
                    let idx = buf.g1();
                    let a = buf.g2();
                    let b = buf.g2();
                    self.op20.push((idx, a, b));
                }
                250 => {
                    let _debugname = buf.gjstr(0);
                }
                _ => panic!("SeqType: unknown opcode {opcode} at id {}", self.id),
            }
            let raw_end = buf.pos;
            self.raw_opcodes
                .push((opcode, buf.data[raw_start..raw_end].to_vec()));
        }
    }

    fn post_decode(types: &mut Vec<Self>, _ctx: &Self::Context) {
        for seq in types.iter_mut() {
            if seq.move_type == -1 {
                seq.move_type = if seq.has_sound_mask { 2 } else { 0 };
            }
            if seq.interrupt_type == -1 {
                seq.interrupt_type = if seq.has_sound_mask { 2 } else { 0 };
            }
        }
    }
}
