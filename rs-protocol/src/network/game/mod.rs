pub mod client;
pub mod client_prot;
pub mod client_prot_category;
pub mod client_prot_frame;
pub mod client_prot_message;
pub mod info_prot;
pub mod info_prot_message;
pub mod server;
pub mod server_prot;
pub mod server_prot_message;
pub mod server_prot_priority;

/// The client reads and writes packets through 5000-byte buffers, so no
/// packet either way, opcode and length included, can be bigger.
pub const MAX_PACKET_SIZE: usize = 5000;
