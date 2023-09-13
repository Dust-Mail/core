mod cache;
mod client;
mod runtime;

pub mod error;

#[cfg(feature = "discover")]
pub mod discover;

#[cfg(not(any(feature = "runtime-tokio", feature = "runtime-async-std")))]
compile_error!("one of 'runtime-async-std' or 'runtime-tokio' features must be enabled");

#[cfg(all(feature = "runtime-tokio", feature = "runtime-async-std"))]
compile_error!("only one of 'runtime-async-std' or 'runtime-tokio' features must be enabled");

pub mod types;

pub use client::{
    create, Credentials, EmailClient, IncomingEmailProtocol, KeepAlive, OutgoingEmailProtocol,
    ServerCredentials, ThreadableEmailClient,
};
