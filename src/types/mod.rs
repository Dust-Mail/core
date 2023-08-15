mod client;
mod connection;
mod flags;
mod mailbox;
mod message;

use std::collections::HashMap;

pub use client::*;
pub use connection::ConnectionSecurity;
pub use flags::Flag;
pub use mailbox::{MailBox, MailBoxList, MessageCounts};
pub use message::{Address, Content, Message, Preview};

pub type Headers = HashMap<String, String>;
