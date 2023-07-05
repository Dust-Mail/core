use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::{self, Duration, Instant},
};

use crate::{debug, types::Result};

// Exactly 29 minutes, recommended by the imap rfc.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(60 * 29);

pub struct ImapSessionWithKeepAlive<S: AsyncWrite + AsyncRead + Unpin + Debug + Send> {
    session: async_imap::Session<S>,
    last_command_sent: Option<Instant>,
}

impl<S: AsyncWrite + AsyncRead + Unpin + Debug + Send> ImapSessionWithKeepAlive<S> {
    pub fn new(session: async_imap::Session<S>) -> Self {
        Self {
            session,
            last_command_sent: None,
        }
    }

    /// Resets the `last_command_sent` to `Instant::now` so that we don't sent a keep alive packet to the server.
    fn reset_keep_alive(&mut self) {
        debug!("We just sent a command!");
        self.last_command_sent = Some(Instant::now());
    }

    pub async fn keep_alive(&mut self) -> Result<()> {
        // We want to check every minute if it has been longer than 29 minutes since the last command has been sent
        let check_interval = Duration::from_secs(60);

        loop {
            time::sleep(check_interval).await;

            if let Some(instant) = &self.last_command_sent {
                let elapsed = instant.elapsed();

                if elapsed >= KEEP_ALIVE_INTERVAL {
                    self.session.noop().await?;
                }
            }
        }
    }
}

impl<S: AsyncWrite + AsyncRead + Unpin + Debug + Send> Deref for ImapSessionWithKeepAlive<S> {
    type Target = async_imap::Session<S>;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl<S: AsyncWrite + AsyncRead + Unpin + Debug + Send> DerefMut for ImapSessionWithKeepAlive<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.reset_keep_alive();
        &mut self.session
    }
}
