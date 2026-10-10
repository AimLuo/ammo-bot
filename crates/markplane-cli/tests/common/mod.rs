//! Shared helpers for markplane-cli integration tests.

use std::net::TcpListener;

/// Bind an ephemeral localhost port and return its number.
pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
