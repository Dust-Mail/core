pub mod io {

    #[cfg(feature = "runtime-async-std")]
    pub(crate) use async_std::io::{Read, Write};

    #[cfg(feature = "runtime-tokio")]
    pub(crate) use tokio::io::{
        AsyncBufRead as BufRead, AsyncRead as Read, AsyncWrite as Write, BufStream,
    };
}

pub mod time {
    #[cfg(feature = "runtime-async-std")]
    pub use async_std::task::sleep;
    #[cfg(feature = "runtime-async-std")]
    pub use std::time::{Duration, Instant};

    #[cfg(feature = "runtime-tokio")]
    pub use tokio::time::{sleep, Duration, Instant};
}

pub mod thread {
    #[cfg(feature = "runtime-async-std")]
    pub(crate) use async_std::{sync::RwLock, task::spawn};

    #[cfg(feature = "runtime-tokio")]
    pub(crate) use tokio::{sync::RwLock, task::spawn};
}

pub mod net {
    #[cfg(feature = "runtime-async-std")]
    pub(crate) use async_std::net::TcpStream;

    #[cfg(feature = "runtime-tokio")]
    pub(crate) use tokio::net::TcpStream;
}

#[cfg(feature = "runtime-async-std")]
pub(crate) use async_std::task::JoinHandle;

#[cfg(feature = "runtime-tokio")]
pub(crate) use tokio::task::JoinHandle;
