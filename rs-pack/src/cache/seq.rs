use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

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

#[cfg_attr(since_647, derive(Debug, Clone))]
pub struct SeqTypeRaw {
    pub id: u16,
    pub frames: Option<Box<[u32]>>,
    pub iframes: Option<Box<[Option<u32>]>>,
    pub delays: Option<Box<[u16]>>,
    pub duration: u32,
    pub loops: Option<u16>,
    pub walkmerge: Option<Box<[i32]>>,
    pub reachforward: bool,
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
    #[cfg(since_647)]
    pub synths: Option<Box<[Option<SeqSynth>]>>,
    #[cfg(since_647)]
    pub a_bool9: bool,
    #[cfg(since_647)]
    pub tween: bool,
    #[cfg(since_647)]
    pub a_bool8: bool,
    #[cfg(since_647)]
    pub a_bool10: bool,
    #[cfg(since_647)]
    pub op19: Vec<(u8, u8)>,
    #[cfg(since_647)]
    pub op20: Vec<(u8, u16, u16)>,
    debugname: Option<Box<str>>,
    pub opcode_order: Vec<u8>,
    #[cfg(since_647)]
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
}

impl CacheType for SeqTypeRaw {
    #[cfg(before_647)]
    type Context = Box<[u8]>;
    #[cfg(since_647)]
    type Context = ();

    fn new(id: u16) -> Self {
        SeqTypeRaw {
            id,
            frames: None,
            iframes: None,
            delays: None,
            duration: 0,
            loops: None,
            walkmerge: None,
            reachforward: false,
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
            #[cfg(since_647)]
            synths: None,
            #[cfg(since_647)]
            a_bool9: false,
            #[cfg(since_647)]
            tween: false,
            #[cfg(since_647)]
            a_bool8: false,
            #[cfg(since_647)]
            a_bool10: false,
            #[cfg(since_647)]
            op19: Vec::new(),
            #[cfg(since_647)]
            op20: Vec::new(),
            debugname: None,
            opcode_order: Vec::new(),
            #[cfg(since_647)]
            raw_opcodes: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            #[cfg(since_647)]
            let start = buf.pos;
            self.opcode_order.push(code);
            match code {
                1 => self.decode_frames(buf),
                2 => self.loops = Some(buf.g2()),
                3 => {
                    let count = buf.g1() as usize;
                    let mut walkmerge = Vec::with_capacity(count + 1);
                    for _ in 0..count {
                        walkmerge.push(buf.g1() as i32);
                    }
                    #[cfg(before_647)]
                    walkmerge.push(9999999);
                    self.walkmerge = Some(walkmerge.into_boxed_slice());
                }
                #[cfg(before_647)]
                4 => self.reachforward = true,
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
                #[cfg(since_647)]
                12 => {
                    let count = buf.g1() as usize;
                    self.iframes = Some(
                        Self::read_frames(buf, count)
                            .into_iter()
                            .map(Some)
                            .collect(),
                    );
                }
                #[cfg(since_647)]
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
                #[cfg(since_647)]
                14 => self.a_bool9 = true,
                #[cfg(since_647)]
                15 => self.tween = true,
                #[cfg(since_647)]
                16 => self.a_bool8 = true,
                #[cfg(since_647)]
                18 => self.a_bool10 = true,
                #[cfg(since_647)]
                19 => {
                    let idx = buf.g1();
                    let val = buf.g1();
                    self.op19.push((idx, val));
                }
                #[cfg(since_647)]
                20 => {
                    let idx = buf.g1();
                    let a = buf.g2();
                    let b = buf.g2();
                    self.op20.push((idx, a, b));
                }
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("seq", code),
            }
            #[cfg(since_647)]
            self.raw_opcodes
                .push((code, buf.data[start..buf.pos].to_vec()));
        }
    }

    fn post_decode(types: &mut Vec<Self>, _frame_delays: &Self::Context) {
        for seq in types.iter_mut() {
            #[cfg(before_647)]
            if let (Some(frames), Some(delays)) = (&seq.frames, &mut seq.delays) {
                let delays = delays.as_mut();
                let mut duration = 0;
                for i in 0..frames.len() {
                    if delays[i] == 0 {
                        let frame_id = frames[i] as usize;
                        delays[i] = _frame_delays.get(frame_id).copied().unwrap_or(0) as u16;
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
                #[cfg(before_647)]
                if seq.frames.is_none() {
                    seq.frames = Some(Box::new([u16::MAX as u32]));
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

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct SeqSynth {
    pub head: i32,
    pub tail: Box<[u16]>,
}

impl SeqTypeRaw {
    fn decode_frames(&mut self, buf: &mut Packet) {
        #[cfg(before_647)]
        let count = buf.g1() as usize;
        #[cfg(since_647)]
        let count = buf.g2() as usize;
        let mut delays = Vec::with_capacity(count);
        #[cfg(before_647)]
        let mut frames = Vec::with_capacity(count);
        #[cfg(before_647)]
        let mut iframes = Vec::with_capacity(count);
        for _ in 0..count {
            #[cfg(before_647)]
            {
                frames.push(buf.g2() as u32);
                let iframe = buf.g2();
                iframes.push((iframe != u16::MAX).then_some(iframe as u32));
            }
            delays.push(buf.g2());
        }
        #[cfg(before_647)]
        let frames = frames.into_boxed_slice();
        #[cfg(before_647)]
        {
            self.iframes = Some(iframes.into_boxed_slice());
        }
        #[cfg(since_647)]
        let frames = Self::read_frames(buf, count);
        #[cfg(since_647)]
        {
            self.duration = delays.iter().map(|&delay| delay as u32).sum();
        }
        self.frames = Some(frames);
        self.delays = Some(delays.into_boxed_slice());
    }

    #[cfg(since_647)]
    fn read_frames(buf: &mut Packet, count: usize) -> Box<[u32]> {
        let mut frames: Box<[u32]> = (0..count).map(|_| buf.g2() as u32).collect();
        for frame in &mut frames {
            *frame |= (buf.g2() as u32) << 16;
        }
        frames
    }
}
