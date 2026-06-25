use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;

pub type SeqTypeProvider = TypeProvider<SeqType>;

pub struct SeqType {
    pub id: u16,
    pub duration: u32,
    pub priority: u8,
    debugname: Option<Box<str>>,
}

impl SeqType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl From<SeqTypeRaw> for SeqType {
    fn from(raw: SeqTypeRaw) -> Self {
        SeqType {
            id: raw.id,
            duration: raw.duration,
            priority: raw.priority,
            debugname: raw.debugname,
        }
    }
}

pub struct SeqTypeRaw {
    pub id: u16,
    pub frames: Option<Box<[u16]>>,
    pub iframes: Option<Box<[Option<u16>]>>,
    pub delays: Option<Box<[u16]>>,
    pub loops: Option<u16>,
    pub walkmerge: Option<Box<[i32]>>,
    pub stretches: bool,
    pub priority: u8,
    pub replaceheldleft: Option<u16>,
    pub replaceheldright: Option<u16>,
    pub maxloops: u8,
    #[cfg(since_244)]
    pub preanim_move: Option<u8>,
    #[cfg(since_244)]
    pub postanim_move: Option<u8>,
    #[cfg(since_244)]
    pub duplicatebehaviour: Option<u8>,
    pub duration: u32,
    debugname: Option<Box<str>>,
}

impl CacheType for SeqTypeRaw {
    type Context = Box<[u8]>;

    fn new(id: u16) -> Self {
        SeqTypeRaw {
            id,
            frames: None,
            iframes: None,
            delays: None,
            loops: None,
            walkmerge: None,
            stretches: false,
            priority: 5,
            replaceheldleft: None,
            replaceheldright: None,
            maxloops: 99,
            #[cfg(since_244)]
            preanim_move: None,
            #[cfg(since_244)]
            postanim_move: None,
            #[cfg(since_244)]
            duplicatebehaviour: None,
            duration: 0,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    let count = buf.g1() as usize;
                    let mut frames = Vec::with_capacity(count);
                    let mut iframes = Vec::with_capacity(count);
                    let mut delays = Vec::with_capacity(count);
                    for _ in 0..count {
                        frames.push(buf.g2());
                        let iframe = buf.g2();
                        iframes.push(if iframe == 65535 { None } else { Some(iframe) });
                        delays.push(buf.g2());
                    }
                    self.frames = Some(frames.into_boxed_slice());
                    self.iframes = Some(iframes.into_boxed_slice());
                    self.delays = Some(delays.into_boxed_slice());
                }
                2 => self.loops = Some(buf.g2()),
                3 => {
                    let count = buf.g1() as usize;
                    let mut walkmerge = Vec::with_capacity(count + 1);
                    for _ in 0..count {
                        walkmerge.push(buf.g1() as i32);
                    }
                    walkmerge.push(9999999);
                    self.walkmerge = Some(walkmerge.into_boxed_slice());
                }
                4 => self.stretches = true,
                5 => self.priority = buf.g1(),
                6 => self.replaceheldleft = Some(buf.g2()),
                7 => self.replaceheldright = Some(buf.g2()),
                8 => self.maxloops = buf.g1(),
                #[cfg(since_244)]
                9 => self.preanim_move = Some(buf.g1()),
                #[cfg(since_244)]
                10 => self.postanim_move = Some(buf.g1()),
                #[cfg(since_244)]
                11 => self.duplicatebehaviour = Some(buf.g1()),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognized seq config code: {code}"),
            }
        }
    }

    fn post_decode(types: &mut Vec<Self>, frame_delays: &Self::Context) {
        for seq in types.iter_mut() {
            if let (Some(frames), Some(delays)) = (&seq.frames, &mut seq.delays) {
                let delays = delays.as_mut();
                let mut duration = 0;
                for i in 0..frames.len() {
                    if delays[i] == 0 {
                        let frame_id = frames[i] as usize;
                        delays[i] = frame_delays.get(frame_id).copied().unwrap_or(0) as u16;
                    }
                    if delays[i] == 0 {
                        delays[i] = 1;
                    }
                    duration += delays[i] as u32;
                }
                seq.duration = duration;
            }

            #[cfg(since_244)]
            {
                if seq.frames.is_none() {
                    seq.frames = Some(Box::new([u16::MAX]));
                    seq.iframes = Some(Box::new([None]));
                    seq.delays = Some(Box::new([u16::MAX]));
                }
                let walk_default = if seq.walkmerge.is_none() { 0 } else { 2 };
                if seq.preanim_move.is_none() {
                    seq.preanim_move = Some(walk_default);
                }
                if seq.postanim_move.is_none() {
                    seq.postanim_move = Some(walk_default);
                }
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
