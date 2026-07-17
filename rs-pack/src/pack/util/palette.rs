use std::collections::HashMap;
use std::collections::hash_map::Entry;

use crate::back;
use crate::sheet::Parsed;

const MAGENTA: [u8; 3] = [255, 0, 255];
const MAX_ENTRIES: usize = 255;

fn is_prime(n: usize) -> bool {
    n >= 2
        && (2..)
            .take_while(|d| d * d <= n)
            .all(|d| !n.is_multiple_of(d))
}

fn algorithmic_cols(n: usize) -> usize {
    let n = n.max(1);
    if n == 1 || is_prime(n) {
        return n;
    }
    let mut cols = n.isqrt();
    if cols * cols < n {
        cols += 1;
    }
    let mut rows = n.div_ceil(cols);
    let mut tries = 0;
    while cols * rows > n && tries < 10 {
        cols += 1;
        rows -= 1;
        tries += 1;
    }
    if cols * rows == n { cols } else { n }
}

fn scan_cols(group: &Parsed) -> usize {
    group
        .scan_cols
        .unwrap_or_else(|| algorithmic_cols(group.frames.len()))
}

fn rgb_at(palette: &[u8], name: &str, idx: u8) -> [u8; 3] {
    if idx == 0 {
        return MAGENTA;
    }
    let i = idx as usize * 3;
    assert!(
        i + 3 <= palette.len(),
        "{name}: pixel references color-map entry {idx} but the sheet only has {} entries",
        palette.len() / 3
    );
    [palette[i], palette[i + 1], palette[i + 2]]
}

struct Scan {
    order: Vec<u8>,
    index_of: HashMap<[u8; 3], u8>,
}

impl Scan {
    fn new() -> Self {
        Self {
            order: MAGENTA.to_vec(),
            index_of: HashMap::from([(MAGENTA, 0)]),
        }
    }

    fn see(&mut self, rgb: [u8; 3], name: &str) {
        if let Entry::Vacant(slot) = self.index_of.entry(rgb) {
            let next = self.order.len() / 3;
            assert!(
                next < MAX_ENTRIES,
                "{name}: more than {MAX_ENTRIES} palette entries (including transparent); \
                 the cache palette-count byte cannot hold more"
            );
            slot.insert(next as u8);
            self.order.extend_from_slice(&rgb);
        }
    }
}

fn scan_frames(
    name: &str,
    tile_w: usize,
    tile_h: usize,
    palette: &[u8],
    frames: &[Vec<u8>],
    cols: usize,
) -> Scan {
    let mut scan = Scan::new();
    let rows = frames.len().max(1).div_ceil(cols);
    for gy in 0..rows * tile_h {
        let (row, fy) = (gy / tile_h, gy % tile_h);
        for col in 0..cols {
            let Some(frame) = frames.get(row * cols + col) else {
                continue;
            };
            for fx in 0..tile_w {
                scan.see(rgb_at(palette, name, frame[fy * tile_w + fx]), name);
            }
        }
    }
    scan
}

fn apply(scan: &Scan, name: &str, group: &mut Parsed) {
    let old = std::mem::replace(&mut group.palette, scan.order.clone());
    for frame in &mut group.frames {
        for px in frame.iter_mut() {
            if *px != 0 {
                let rgb = rgb_at(&old, name, *px);
                *px = *scan.index_of.get(&rgb).unwrap_or_else(|| {
                    panic!("{name}: color {rgb:?} missing from derived palette")
                });
            }
        }
    }
}

pub fn derive_group(name: &str, group: &mut Parsed) {
    let scan = scan_frames(
        name,
        group.tile_w,
        group.tile_h,
        &group.palette,
        &group.frames,
        scan_cols(group),
    );
    apply(&scan, name, group);
}

pub fn detect_scan_cols(
    name: &str,
    tile_w: usize,
    tile_h: usize,
    palette: &[u8],
    frames: &[Vec<u8>],
) -> Option<usize> {
    let matches =
        |cols: usize| scan_frames(name, tile_w, tile_h, palette, frames, cols).order == palette;
    if matches(algorithmic_cols(frames.len())) {
        return None;
    }
    (1..=frames.len()).find(|&cols| matches(cols))
}

pub fn derive_media(groups: &mut [(String, Parsed)]) {
    let mut pieces: HashMap<&str, &Parsed> = HashMap::new();
    for (name, group) in groups.iter() {
        pieces.insert(name.as_str(), group);
    }

    let mut canvas = vec![MAGENTA; back::CANVAS_W * back::CANVAS_H];
    for &(name, x, y, w, h) in back::FRAME {
        let piece = pieces
            .get(name)
            .unwrap_or_else(|| panic!("missing back piece {name}"));
        assert_eq!(
            (piece.tile_w, piece.tile_h, piece.frames.len()),
            (w, h, 1),
            "{name}: expected a single {w}x{h} frame"
        );
        for fy in 0..h {
            for fx in 0..w {
                canvas[(y + fy) * back::CANVAS_W + x + fx] =
                    rgb_at(&piece.palette, name, piece.frames[0][fy * w + fx]);
            }
        }
    }
    let mut scan = Scan::new();
    for rgb in canvas {
        scan.see(rgb, "back composite");
    }

    let back_names: Vec<&str> = back::FRAME.iter().map(|&(n, ..)| n).collect();
    for (name, group) in groups.iter_mut() {
        if back_names.contains(&name.as_str()) {
            apply(&scan, name, group);
        } else {
            derive_group(name, group);
        }
    }
}
