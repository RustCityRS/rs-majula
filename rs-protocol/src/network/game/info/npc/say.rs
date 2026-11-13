use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub struct NpcInfoSay<'a> {
    pub say: &'a str,
}

impl InfoMessage for NpcInfoSay<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.pjstr(self.say, STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        Packet::sizejstr(self.say)
    }
}
