pub mod client;
pub mod client_prot;
pub mod client_prot_category;
pub mod client_prot_frame;
pub mod client_prot_message;
/// Rev-647: the client's own server prot table (`ServerProt.java`).
#[cfg(since_647)]
pub mod client_server_prots_647;
pub mod info_prot;
pub mod info_prot_message;
pub mod server;
pub mod server_prot;
pub mod server_prot_message;
pub mod server_prot_priority;

/// The client reads and writes packets through 5000-byte buffers, so no
/// packet either way, opcode and length included, can be bigger.
#[cfg(before_647)]
pub const MAX_PACKET_SIZE: usize = 5000;
/// From rev 647 the client reads through a 15000-byte buffer (its
/// `input_packet`): player info and region rebuilds outgrow 5000.
#[cfg(since_647)]
pub const MAX_PACKET_SIZE: usize = 15000;
