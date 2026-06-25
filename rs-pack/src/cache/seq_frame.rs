#[cfg(rev = "225")]
use rs_io::jag::JagFile;
#[cfg(since_244)]
use std::path::Path;

#[cfg(rev = "225")]
pub struct SeqFrameProvider {
    pub delays: Box<[u8]>,
}

#[cfg(rev = "225")]
impl SeqFrameProvider {
    pub fn from_jag(jag_bytes: &[u8]) -> Self {
        let jag = JagFile::from(jag_bytes.to_vec());
        let delays = match jag.read("frame_del.dat") {
            Some(packet) => packet.data.into_boxed_slice(),
            None => Box::new([]),
        };
        SeqFrameProvider { delays }
    }

    pub fn get_delay(&self, frame_id: u16) -> u8 {
        self.delays.get(frame_id as usize).copied().unwrap_or(0)
    }

    pub fn count(&self) -> usize {
        self.delays.len()
    }
}

#[cfg(since_244)]
pub fn anim_frame_delays(content_dir: &Path) -> Box<[u8]> {
    let anim_dir = content_dir.join("models").join("anim");
    let mut delays: Vec<u8> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&anim_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("anim"))
                && let Ok(data) = std::fs::read(&path)
            {
                parse_anim_delays(&data, &mut delays);
            }
        }
    }
    delays.into_boxed_slice()
}

#[cfg(since_244)]
fn parse_anim_delays(src: &[u8], delays: &mut Vec<u8>) {
    if src.len() < 8 {
        return;
    }
    let g2 = |p: usize| ((src[p] as usize) << 8) | src[p + 1] as usize;
    // The trailing 8 bytes are four g2 section lengths: head (+2), tran1, tran2, del.
    let meta = src.len() - 8;
    let del_off = (g2(meta) + 2) + g2(meta + 2) + g2(meta + 4);
    // head: total (g2), then per frame [id (g2), groupCount (g1)]. The matching
    // `del` section holds one g1 delay per frame, in the same order.
    let total = g2(0);
    let mut head_pos = 2;
    for i in 0..total {
        if head_pos + 3 > meta || del_off + i >= meta {
            return;
        }
        let id = g2(head_pos);
        head_pos += 3;
        let delay = src[del_off + i];
        if id >= delays.len() {
            delays.resize(id + 1, 0);
        }
        delays[id] = delay;
    }
}
