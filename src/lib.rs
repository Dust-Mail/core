mod cache;
mod client;
mod parse;

pub mod error;

#[cfg(feature = "discover")]
pub mod discover;

pub mod types;

pub use client::{
    create, Credentials, EmailClient, IncomingEmailProtocol, KeepAlive, OutgoingEmailProtocol,
    ServerCredentials, ThreadableEmailClient,
};
