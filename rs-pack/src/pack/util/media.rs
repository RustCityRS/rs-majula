use std::path::Path;

use crate::sheet::{self, Parsed};
use crate::tga;

pub fn read_group(path: &Path) -> Parsed {
    let sheet = tga::read(path);
    sheet::parse(&sheet)
}

pub fn read_index_order(archive_dir: &Path) -> Vec<String> {
    let path = archive_dir.join("meta").join("index.order");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()))
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect()
}

pub fn emit_group(index: &mut Vec<u8>, group: &Parsed) -> Vec<u8> {
    let tile_w = group.tile_w;
    let tile_h = group.tile_h;
    let palette_count = group.palette.len() / 3;

    let mut data = Vec::new();
    let index_pos = index.len() as u16;
    data.push((index_pos >> 8) as u8);
    data.push(index_pos as u8);

    index.push((tile_w >> 8) as u8);
    index.push(tile_w as u8);
    index.push((tile_h >> 8) as u8);
    index.push(tile_h as u8);
    index.push(palette_count as u8);
    index.extend_from_slice(&group.palette[3..]);

    for pixels in &group.frames {
        let mut min_x = usize::MAX;
        let mut min_y = usize::MAX;
        let mut max_x = 0usize;
        let mut max_y = 0usize;
        let mut has_content = false;

        for y in 0..tile_h {
            for x in 0..tile_w {
                if pixels[y * tile_w + x] != 0 {
                    has_content = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        let (crop_x, crop_y, content_w, content_h) = if has_content {
            (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
        } else {
            (0, 0, tile_w, tile_h)
        };

        index.push(crop_x as u8);
        index.push(crop_y as u8);
        index.push((content_w >> 8) as u8);
        index.push(content_w as u8);
        index.push((content_h >> 8) as u8);
        index.push(content_h as u8);

        let mut indices = vec![0u8; content_w * content_h];
        for y in 0..content_h {
            for x in 0..content_w {
                indices[y * content_w + x] = pixels[(crop_y + y) * tile_w + crop_x + x];
            }
        }

        let mut row_runs: i64 = 0;
        let mut prev: i64 = -1;
        for idx in &indices {
            let v = *idx as i64;
            if v == prev {
                row_runs += 1;
            }
            prev = v;
        }
        let mut col_runs: i64 = 0;
        prev = -1;
        for x in 0..content_w {
            for y in 0..content_h {
                let v = indices[y * content_w + x] as i64;
                if v == prev {
                    col_runs += 1;
                }
                prev = v;
            }
        }
        let pixel_order: u8 = if col_runs > row_runs { 1 } else { 0 };
        index.push(pixel_order);

        if pixel_order == 0 {
            data.extend_from_slice(&indices);
        } else {
            for x in 0..content_w {
                for y in 0..content_h {
                    data.push(indices[y * content_w + x]);
                }
            }
        }
    }

    data
}
