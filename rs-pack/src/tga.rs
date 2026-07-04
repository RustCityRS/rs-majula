use std::path::Path;

pub struct Indexed {
    pub image_id: Vec<u8>,
    pub palette: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

pub fn write(
    path: &Path,
    image_id: &[u8],
    palette: &[u8],
    width: u32,
    height: u32,
    pixels: &[u8],
) -> std::io::Result<()> {
    let entries = palette.len() / 3;
    let mut out = Vec::with_capacity(18 + image_id.len() + entries * 3 + pixels.len());
    out.extend_from_slice(&[
        image_id.len() as u8, // image id length
        1,                    // has color map
        1,                    // uncompressed color-mapped
        0,
        0, // color map first entry index
        entries as u8,
        (entries >> 8) as u8, // color map length
        24,                   // color map entry size
        0,
        0,
        0,
        0, // origin
        width as u8,
        (width >> 8) as u8,
        height as u8,
        (height >> 8) as u8,
        8, // bits per pixel
        0, // descriptor: bottom-left origin
    ]);
    out.extend_from_slice(image_id);
    for rgb in palette.chunks_exact(3) {
        out.extend_from_slice(&[rgb[2], rgb[1], rgb[0]]); // BGR
    }
    for y in (0..height as usize).rev() {
        let row = &pixels[y * width as usize..(y + 1) * width as usize];
        out.extend_from_slice(row);
    }
    std::fs::write(path, out)
}

pub fn read(path: &Path) -> Indexed {
    let data =
        std::fs::read(path).unwrap_or_else(|e| panic!("Cannot open {}: {e}", path.display()));
    let fail = |msg: &str| -> ! { panic!("{}: {msg}", path.display()) };
    if data.len() < 18 {
        fail("not a TGA file");
    }

    let id_len = data[0] as usize;
    let cmap_type = data[1];
    let image_type = data[2];
    let cmap_first = u16::from_le_bytes([data[3], data[4]]) as usize;
    let cmap_len = u16::from_le_bytes([data[5], data[6]]) as usize;
    let cmap_bits = data[7];
    let width = u16::from_le_bytes([data[12], data[13]]) as usize;
    let height = u16::from_le_bytes([data[14], data[15]]) as usize;
    let pixel_bits = data[16];
    let descriptor = data[17];

    if cmap_type != 1 || (image_type != 1 && image_type != 9) {
        fail("sprite sheets must be color-mapped TGAs (type 1 or 9)");
    }
    if pixel_bits != 8 {
        fail("sprite sheets must be 8-bit indexed");
    }
    if cmap_first != 0 {
        fail("color map must start at entry 0");
    }
    if descriptor & 0x10 != 0 {
        fail("right-to-left TGAs are not supported");
    }

    let mut pos = 18;
    let image_id = data[pos..pos + id_len].to_vec();
    pos += id_len;

    let cmap_bytes = (cmap_bits as usize).div_ceil(8);
    let mut palette = Vec::with_capacity(cmap_len * 3);
    for _ in 0..cmap_len {
        let b = data[pos];
        let g = data[pos + 1];
        let r = data[pos + 2];
        palette.extend_from_slice(&[r, g, b]);
        pos += cmap_bytes;
    }

    let count = width * height;
    let mut pixels = Vec::with_capacity(count);
    if image_type == 1 {
        pixels.extend_from_slice(&data[pos..pos + count]);
    } else {
        let mut done = 0;
        while done < count {
            let control = data[pos] as usize;
            pos += 1;
            let run = (control & 0x7F) + 1;
            if control & 0x80 != 0 {
                let v = data[pos];
                pos += 1;
                pixels.extend(std::iter::repeat_n(v, run));
            } else {
                pixels.extend_from_slice(&data[pos..pos + run]);
                pos += run;
            }
            done += run;
        }
    }

    // Bottom-left origin (the default) stores rows bottom-up.
    if descriptor & 0x20 == 0 {
        let mut flipped = Vec::with_capacity(pixels.len());
        for y in (0..height).rev() {
            flipped.extend_from_slice(&pixels[y * width..(y + 1) * width]);
        }
        pixels = flipped;
    }

    Indexed {
        image_id,
        palette,
        width,
        height,
        pixels,
    }
}
