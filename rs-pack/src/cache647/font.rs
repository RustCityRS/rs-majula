pub struct FontType {
    pub id: i32,
    pub glyph_widths: [u8; 256],
}

impl FontType {
    pub fn decode(id: i32, data: &[u8]) -> Self {
        let mut glyph_widths = [0u8; 256];
        if data.len() >= 258 {
            glyph_widths.copy_from_slice(&data[2..258]);
        }
        FontType { id, glyph_widths }
    }

    pub fn string_width(&self, s: &str) -> u16 {
        let bytes = s.as_bytes();
        let mut size: u16 = 0;
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'@' && i + 4 < bytes.len() && bytes[i + 4] == b'@' {
                i += 5;
            } else {
                size += self.glyph_widths[bytes[i] as usize] as u16;
                i += 1;
            }
        }
        size
    }

    pub fn split(&self, s: &str, max_width: u16) -> Vec<String> {
        if s.is_empty() {
            return vec![s.to_string()];
        }

        let mut lines = Vec::new();
        let mut remaining = s;

        while !remaining.is_empty() {
            let width = self.string_width(remaining);
            if width <= max_width && !remaining.contains('|') {
                lines.push(remaining.to_string());
                break;
            }

            let mut split_index = remaining.len();
            let bytes = remaining.as_bytes();

            for i in 0..bytes.len() {
                if bytes[i] == b' ' {
                    let w = self.string_width(&remaining[..i]);
                    if w > max_width {
                        break;
                    }
                    split_index = i;
                } else if bytes[i] == b'|' {
                    split_index = i;
                    break;
                }
            }

            lines.push(remaining[..split_index].to_string());
            remaining = &remaining[split_index + 1..];
        }

        lines
    }
}
