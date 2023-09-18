use std::sync::Arc;

use crate::runtime::{
    thread::{spawn, RwLock},
    time::{sleep, Duration},
    JoinHandle,
};

use log::{info, trace, warn};

use super::EmailClient;

pub struct KeepAlive {
    client: Arc<RwLock<EmailClient>>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for KeepAlive {
    fn drop(&mut self) {
        self.stop();
    }
}

impl From<Arc<RwLock<EmailClient>>> for KeepAlive {
    fn from(client: Arc<RwLock<EmailClient>>) -> Self {
        Self {
            client,
            handle: None,
        }
    }
}

impl KeepAlive {
    pub fn new(client: &Arc<RwLock<EmailClient>>) -> Self {
        Self {
            client: Arc::clone(client),
            handle: None,
        }
    }

    const CHECK_TIME: Duration = Duration::from_secs(5);

    pub fn start(&mut self) {
        // Stop any threads that are already running.
        self.stop();

        let client = Arc::clone(&self.client);

        let handle = spawn(async move {
            loop {
                sleep(Self::CHECK_TIME).await;

                let read_lock = client.read().await;

                trace!("Checking if keep alive request is needed");

                if read_lock.should_keep_alive() {
                    let mut write_lock = client.write().await;

                    info!("Sending keep alive request to mail server");

                    if let Err(err) = write_lock.send_keep_alive().await {
                        warn!("Failed to send keep alive request to mail server: {}", err)
                    }
                }
            }
        });

        self.handle = Some(handle);
    }

    pub fn stop(&mut self) {
        if let Some(_handle) = &self.handle {
            info!("Stopping keep alive requests");

            #[cfg(feature = "runtime-tokio")]
            _handle.abort();

            self.handle = None;
        }
    }
}
