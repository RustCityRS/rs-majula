use crate::tga::Indexed;

pub const GRIDLINE: [u8; 3] = [128, 128, 128];

fn gridded(frame_count: usize) -> bool {
    frame_count > 1
}

pub fn grid(tile_w: usize, tile_h: usize, frame_count: usize) -> (usize, usize) {
    let n = frame_count.max(1);
    let (tw, th) = (tile_w.max(1), tile_h.max(1));
    let mut best_cols = 1;
    let mut best_score = usize::MAX;
    for cols in 1..=n {
        let rows = n.div_ceil(cols);
        let score = (cols * tw).abs_diff(rows * th);
        if score <= best_score {
            best_score = score;
            best_cols = cols;
        }
    }
    (best_cols, n.div_ceil(best_cols))
}

fn cell_origin(
    tile_w: usize,
    tile_h: usize,
    cols: usize,
    i: usize,
    frame_count: usize,
) -> (usize, usize) {
    let (col, row) = (i % cols, i / cols);
    if gridded(frame_count) {
        (1 + col * (tile_w + 1), 1 + row * (tile_h + 1))
    } else {
        (col * tile_w, row * tile_h)
    }
}

pub fn image_id(tile_w: usize, tile_h: usize, frame_count: usize) -> [u8; 6] {
    [
        (tile_w >> 8) as u8,
        tile_w as u8,
        (tile_h >> 8) as u8,
        tile_h as u8,
        (frame_count >> 8) as u8,
        frame_count as u8,
    ]
}

pub fn render(
    tile_w: usize,
    tile_h: usize,
    palette: &[u8],
    frames: &[Vec<u8>],
) -> (usize, usize, Vec<u8>, Vec<u8>) {
    let frame_count = frames.len();
    let (cols, rows) = grid(tile_w, tile_h, frame_count);

    if !gridded(frame_count) {
        let (w, h) = (cols * tile_w, rows * tile_h);
        let mut pixels = vec![0u8; w * h];
        for (i, frame) in frames.iter().enumerate() {
            let (ox, oy) = cell_origin(tile_w, tile_h, cols, i, frame_count);
            for fy in 0..tile_h {
                for fx in 0..tile_w {
                    pixels[(oy + fy) * w + ox + fx] = frame[fy * tile_w + fx];
                }
            }
        }
        return (w, h, pixels, palette.to_vec());
    }

    let grid_idx = (palette.len() / 3) as u8;
    let mut out_palette = palette.to_vec();
    out_palette.extend_from_slice(&GRIDLINE);
    let w = cols * (tile_w + 1) + 1;
    let h = rows * (tile_h + 1) + 1;
    let mut pixels = vec![grid_idx; w * h]; // start as all gridline...
    for slot in 0..cols * rows {
        let (ox, oy) = cell_origin(tile_w, tile_h, cols, slot, frame_count);
        for fy in 0..tile_h {
            for fx in 0..tile_w {
                pixels[(oy + fy) * w + ox + fx] =
                    frames.get(slot).map_or(0, |f| f[fy * tile_w + fx]);
            }
        }
    }
    (w, h, pixels, out_palette)
}

pub struct Parsed {
    pub tile_w: usize,
    pub tile_h: usize,
    pub palette: Vec<u8>,
    pub frames: Vec<Vec<u8>>,
}

pub fn parse(sheet: &Indexed) -> Parsed {
    assert!(
        sheet.image_id.len() >= 6,
        "sprite sheet is missing its grid dimensions in the image ID field"
    );
    let id = &sheet.image_id;
    let tile_w = ((id[0] as usize) << 8) | id[1] as usize;
    let tile_h = ((id[2] as usize) << 8) | id[3] as usize;
    let frame_count = ((id[4] as usize) << 8) | id[5] as usize;

    let (cols, _rows) = grid(tile_w, tile_h, frame_count);
    let w = sheet.width;
    let mut frames = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let (ox, oy) = cell_origin(tile_w, tile_h, cols, i, frame_count);
        let mut buf = vec![0u8; tile_w * tile_h];
        for fy in 0..tile_h {
            for fx in 0..tile_w {
                buf[fy * tile_w + fx] = sheet.pixels[(oy + fy) * w + ox + fx];
            }
        }
        frames.push(buf);
    }

    let real_entries = sheet.palette.len() / 3 - usize::from(gridded(frame_count));
    let palette = sheet.palette[..real_entries * 3].to_vec();

    Parsed {
        tile_w,
        tile_h,
        palette,
        frames,
    }
}
