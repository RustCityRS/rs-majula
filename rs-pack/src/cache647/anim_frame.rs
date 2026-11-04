#![allow(clippy::needless_range_loop)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Animation frame and skeleton base binary codec.
//!
//! Archive 0 (ANIMS): groups of animation frames. Each file within a group
//! is one frame. The first byte is a type marker, bytes 1-2 are the skeleton
//! base ID (from archive 1), followed by per-bone transform masks and values.
//!
//! Archive 1 (BASES): skeleton base definitions. Each group has one file
//! defining the bone hierarchy: transform types, parent flags, labels, and
//! label groups.
//!
//! Decoded from rev-647 client `Class310` (frame) and `Linkable_Sub11` (base).

use rs_io::Packet;

/// Split an animation frame's raw bytes into 3 stripes matching Jagex's
/// interleaved group encoding:
///   Stripe 0: header (type + base_id + bone_count) = 4 bytes
///   Stripe 1: per-bone mask bytes = bone_count bytes
///   Stripe 2: smart-encoded transform values = remainder
pub fn split_frame_into_stripes(data: &[u8], _num_stripes: u8) -> Vec<Vec<u8>> {
    if data.len() < 4 {
        return vec![data.to_vec(), Vec::new(), Vec::new()];
    }
    let bone_count = data[3] as usize;
    let header_end = 4;
    let masks_end = header_end + bone_count;

    if masks_end > data.len() {
        return vec![data.to_vec(), Vec::new(), Vec::new()];
    }

    vec![
        data[..header_end].to_vec(),
        data[header_end..masks_end].to_vec(),
        data[masks_end..].to_vec(),
    ]
}

// -- Skeleton Base (Archive 1) --

#[derive(Debug, Clone)]
pub struct SkeletonBase {
    pub bone_count: u8,
    pub transform_types: Vec<u8>,
    pub parent_flags: Vec<bool>,
    pub labels: Vec<u16>,
    pub label_groups: Vec<Vec<u8>>,
}

impl SkeletonBase {
    pub fn decode(data: &[u8]) -> Self {
        let mut pkt = Packet::from(data.to_vec());
        let bone_count = pkt.g1();
        let count = bone_count as usize;

        let mut transform_types = Vec::with_capacity(count);
        for _ in 0..count {
            let mut t = pkt.g1();
            if t == 6 {
                t = 2;
            }
            transform_types.push(t);
        }

        let mut parent_flags = Vec::with_capacity(count);
        for _ in 0..count {
            parent_flags.push(pkt.g1() == 1);
        }

        let mut labels = Vec::with_capacity(count);
        for _ in 0..count {
            labels.push(pkt.g2());
        }

        let mut group_counts = Vec::with_capacity(count);
        for _ in 0..count {
            group_counts.push(pkt.g1());
        }

        let mut label_groups = Vec::with_capacity(count);
        for i in 0..count {
            let gc = group_counts[i] as usize;
            let mut group = Vec::with_capacity(gc);
            for _ in 0..gc {
                group.push(pkt.g1());
            }
            label_groups.push(group);
        }

        Self {
            bone_count,
            transform_types,
            parent_flags,
            labels,
            label_groups,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let count = self.bone_count as usize;
        let mut out = Vec::with_capacity(count * 6);

        out.push(self.bone_count);
        for i in 0..count {
            out.push(self.transform_types[i]);
        }
        for i in 0..count {
            out.push(if self.parent_flags[i] { 1 } else { 0 });
        }
        for i in 0..count {
            out.push((self.labels[i] >> 8) as u8);
            out.push(self.labels[i] as u8);
        }
        for i in 0..count {
            out.push(self.label_groups[i].len() as u8);
        }
        for i in 0..count {
            out.extend_from_slice(&self.label_groups[i]);
        }

        out
    }
}

// -- Signed smart encode helper --

fn encode_smart_s(out: &mut Vec<u8>, value: i32) {
    if (-64..64).contains(&value) {
        out.push((value + 64) as u8);
    } else {
        out.push(((value + 49152) >> 8) as u8);
        out.push((value + 49152) as u8);
    }
}

// -- Animation Frame (Archive 0) --

#[derive(Debug, Clone)]
pub struct AnimFrame {
    pub type_marker: u8,
    pub base_id: u16,
    pub bone_count: u8,
    pub bones: Vec<FrameBone>,
}

#[derive(Debug, Clone)]
pub struct FrameBone {
    pub bone_index: u8,
    pub mask: u8,
    pub dx: i16,
    pub dy: i16,
    pub dz: i16,
}

impl AnimFrame {
    pub fn decode(data: &[u8]) -> Self {
        let mut hdr = Packet::from(data.to_vec());
        let mut val = Packet::from(data.to_vec());

        let type_marker = hdr.g1();
        let base_id = hdr.g2();
        let bone_count = hdr.g1();

        val.pos = hdr.pos + bone_count as usize;

        let mut bones = Vec::new();

        for i in 0..bone_count {
            let mask = hdr.g1();
            if mask == 0 {
                continue;
            }

            let dx = if (mask & 0x1) != 0 {
                val.gsmart1or2s() as i16
            } else {
                0
            };
            let dy = if (mask & 0x2) != 0 {
                val.gsmart1or2s() as i16
            } else {
                0
            };
            let dz = if (mask & 0x4) != 0 {
                val.gsmart1or2s() as i16
            } else {
                0
            };

            bones.push(FrameBone {
                bone_index: i,
                mask,
                dx,
                dy,
                dz,
            });
        }

        Self {
            type_marker,
            base_id,
            bone_count,
            bones,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut hdr_buf = Vec::new();
        let mut val_buf = Vec::new();

        hdr_buf.push(self.type_marker);
        hdr_buf.push((self.base_id >> 8) as u8);
        hdr_buf.push(self.base_id as u8);
        hdr_buf.push(self.bone_count);

        let mut bone_idx = 0usize;
        for i in 0..self.bone_count {
            if bone_idx < self.bones.len() && self.bones[bone_idx].bone_index == i {
                let bone = &self.bones[bone_idx];
                hdr_buf.push(bone.mask);
                if (bone.mask & 0x1) != 0 {
                    encode_smart_s(&mut val_buf, bone.dx as i32);
                }
                if (bone.mask & 0x2) != 0 {
                    encode_smart_s(&mut val_buf, bone.dy as i32);
                }
                if (bone.mask & 0x4) != 0 {
                    encode_smart_s(&mut val_buf, bone.dz as i32);
                }
                bone_idx += 1;
            } else {
                hdr_buf.push(0);
            }
        }

        let mut out = Vec::with_capacity(hdr_buf.len() + val_buf.len());
        out.extend_from_slice(&hdr_buf);
        out.extend_from_slice(&val_buf);
        out
    }
}
