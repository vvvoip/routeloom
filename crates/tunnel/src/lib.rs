//! RouteLoom RL/1: bounded IP packets inside authenticated TLS 1.3.
pub mod config;
pub mod packet;
pub mod tls;
pub mod wire;
pub use std::io::{Error, ErrorKind, Result};
pub fn invalid(message: &'static str) -> Error {
    Error::new(ErrorKind::InvalidData, message)
}

pub mod flow;
