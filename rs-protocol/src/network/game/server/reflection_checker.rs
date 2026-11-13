use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::STRING_TERMINATOR;

/// Something a `REFLECTION_CHECKER` request has the client read about its own
/// classes, by name (`AClass23_Sub2.method3077`). The client can also set a
/// field or run a method, which the server never asks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectionCheck {
    FieldValue {
        class: String,
        field: String,
    },
    FieldModifiers {
        class: String,
        field: String,
    },
    MethodModifiers {
        class: String,
        method: String,
        params: Vec<String>,
    },
}

#[server_prot(ReflectionChecker, Buffered, VarShort)]
pub struct ReflectionChecker<'a> {
    pub id: i32,
    pub checks: &'a [ReflectionCheck],
}

impl ServerProtMessage for ReflectionChecker<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.checks.len() as u8);
        buf.p4(self.id);
        for check in self.checks {
            match check {
                ReflectionCheck::FieldValue { class, field } => {
                    buf.p1(0);
                    buf.pjstr(class, STRING_TERMINATOR);
                    buf.pjstr(field, STRING_TERMINATOR);
                }
                ReflectionCheck::FieldModifiers { class, field } => {
                    buf.p1(2);
                    buf.pjstr(class, STRING_TERMINATOR);
                    buf.pjstr(field, STRING_TERMINATOR);
                }
                ReflectionCheck::MethodModifiers {
                    class,
                    method,
                    params,
                } => {
                    buf.p1(4);
                    buf.pjstr(class, STRING_TERMINATOR);
                    buf.pjstr(method, STRING_TERMINATOR);
                    buf.p1(params.len() as u8);
                    for param in params {
                        buf.pjstr(param, STRING_TERMINATOR);
                    }
                }
            }
        }
    }

    fn sizeof(&self) -> usize {
        let check = |check: &ReflectionCheck| match check {
            ReflectionCheck::FieldValue { class, field }
            | ReflectionCheck::FieldModifiers { class, field } => {
                1 + Packet::sizejstr(class) + Packet::sizejstr(field)
            }
            ReflectionCheck::MethodModifiers {
                class,
                method,
                params,
            } => {
                2 + Packet::sizejstr(class)
                    + Packet::sizejstr(method)
                    + params.iter().map(|p| Packet::sizejstr(p)).sum::<usize>()
            }
        };
        1 + size_of_val(&self.id) + self.checks.iter().map(check).sum::<usize>()
    }
}
