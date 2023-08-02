mod cache;
mod client;
mod parse;
mod tcp;

mod validate;

#[cfg(feature = "detect")]
pub mod detect;

pub mod types;

pub use client::{
    create, Credentials, EmailClient, IncomingEmailProtocol, OutgoingEmailProtocol,
    ServerCredentials,
};
