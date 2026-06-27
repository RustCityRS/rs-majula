use std::collections::HashMap;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const MAX_BASE_LEN: usize = 6;
const SUFFIXES: &[&str] = &["", ".dat", ".idx", ".txt"];
const MAX_CANDIDATES: usize = 32;

const INV61: u32 = {
    let mut x: u32 = 1;
    let mut i = 0;
    while i < 5 {
        x = x.wrapping_mul(2u32.wrapping_sub(61u32.wrapping_mul(x)));
        i += 1;
    }
    x
};

#[inline]
fn digit(c: u8) -> u32 {
    (c.to_ascii_uppercase() as u32).wrapping_sub(32)
}

#[inline]
fn unhash_char(h: u32, c: u8) -> u32 {
    h.wrapping_sub(digit(c)).wrapping_mul(INV61)
}

pub fn crack(targets: &[i32]) -> HashMap<i32, Vec<String>> {
    crack_with(targets, MAX_BASE_LEN)
}

fn crack_with(targets: &[i32], max_len: usize) -> HashMap<i32, Vec<String>> {
    let mut wanted: HashMap<u32, Vec<(i32, &'static str)>> = HashMap::new();
    for &target in targets {
        for &ext in SUFFIXES {
            let mut stem_hash = target as u32;
            for &c in ext.as_bytes().iter().rev() {
                stem_hash = unhash_char(stem_hash, c);
            }
            wanted.entry(stem_hash).or_default().push((target, ext));
        }
    }

    let lhalf = max_len.div_ceil(2);
    let rhalf = max_len / 2;

    let mut left: HashMap<u32, Vec<String>> = HashMap::new();
    enumerate(lhalf, &mut |s, h| {
        left.entry(h).or_default().push(s.to_string());
    });

    let mut inv_pow = vec![1u32; rhalf + 1];
    for k in 1..=rhalf {
        inv_pow[k] = inv_pow[k - 1].wrapping_mul(INV61);
    }

    let mut out: HashMap<i32, Vec<String>> = HashMap::new();
    enumerate(rhalf, &mut |right, right_hash| {
        let inv = inv_pow[right.len()];
        for (&stem_hash, hits) in &wanted {
            // hash(left) * 61^rlen + hash(right) == stem_hash
            let need = stem_hash.wrapping_sub(right_hash).wrapping_mul(inv);
            let Some(lefts) = left.get(&need) else {
                continue;
            };
            for l in lefts {
                if l.len() + right.len() == 0 {
                    continue; // the empty stem is not a real name
                }
                let stem = format!("{l}{right}");
                for &(target, ext) in hits {
                    out.entry(target).or_default().push(format!("{stem}{ext}"));
                }
            }
        }
    });

    for names in out.values_mut() {
        names.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        names.dedup();
        names.truncate(MAX_CANDIDATES);
    }
    out
}

fn enumerate(max: usize, f: &mut impl FnMut(&str, u32)) {
    fn go(buf: &mut Vec<u8>, hash: u32, max: usize, f: &mut impl FnMut(&str, u32)) {
        f(std::str::from_utf8(buf).unwrap(), hash);
        if buf.len() == max {
            return;
        }
        for &c in CHARSET {
            buf.push(c);
            go(buf, hash.wrapping_mul(61).wrapping_add(digit(c)), max, f);
            buf.pop();
        }
    }
    go(&mut Vec::with_capacity(max), 0, max, f);
}
