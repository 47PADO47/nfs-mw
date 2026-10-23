//! The seam between a [`super::ClientSession`] and the network: what it needs from a connection.

use blackbox_net_core::endpoint::{ClientEndpoint, ClientEvent};

/// One connection to one server, as the session drives it. [`ClientEndpoint`] implements it (the net thread);
/// [`super::MockLink`] implements it for tests with a scripted server.
pub trait ClientLink {
    /// The next event, oldest first. Never blocks.
    fn poll_event(&mut self) -> Option<ClientEvent>;

    /// Queues a reliable message on `channel` (0 to 3).
    fn send_reliable(&mut self, channel: u8, bytes: Vec<u8>);

    /// Queues an unreliable frame of `frame_type`; a `tag` asks for a delivery report.
    fn send_unreliable(&mut self, frame_type: u8, bytes: Vec<u8>, tag: Option<u32>);

    /// Closes the connection with a DISCONNECT reason byte (`blackbox_net_core::endpoint::reason`).
    fn disconnect(&mut self, reason: u8);

    /// The Noise handshake hash the room password is bound to (`security.md` 4.6), when the link knows it. `None`
    /// binds the password exchange to 32 zero bytes, as the host's own client on the in-memory link does.
    fn handshake_hash(&self) -> Option<[u8; 32]> {
        None
    }
}

impl ClientLink for ClientEndpoint {
    fn poll_event(&mut self) -> Option<ClientEvent> {
        ClientEndpoint::poll_event(self)
    }

    fn send_reliable(&mut self, channel: u8, bytes: Vec<u8>) {
        ClientEndpoint::send_reliable(self, channel, bytes);
    }

    fn send_unreliable(&mut self, frame_type: u8, bytes: Vec<u8>, tag: Option<u32>) {
        ClientEndpoint::send_unreliable(self, frame_type, bytes, tag);
    }

    fn disconnect(&mut self, reason: u8) {
        ClientEndpoint::disconnect(self, reason);
    }
}
