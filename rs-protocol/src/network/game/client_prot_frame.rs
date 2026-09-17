#[derive(Clone, Copy)]
pub enum ClientProtFrame {
    Fixed(u8),
    VarByte { min: u8 },
    VarShort { min: u8 },
}

impl ClientProtFrame {
    #[inline]
    pub const fn min(self) -> usize {
        match self {
            ClientProtFrame::Fixed(len) => len as usize,
            ClientProtFrame::VarByte { min } | ClientProtFrame::VarShort { min } => min as usize,
        }
    }
}
