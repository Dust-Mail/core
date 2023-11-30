use std::path::PathBuf;

use async_trait::async_trait;
use maildir::Maildir;

use crate::{
    client::{
        builder::MessageBuilder,
        mailbox::{Mailbox, MailboxStats},
        message::{Message, Preview},
        protocol::IncomingProtocol,
    },
    error::{err, ErrorKind, Result},
    tree::Node,
};

pub enum DirType {
    Current,
    New,
}

pub struct MaildirClient {
    maildir: Maildir,
}

impl MaildirClient {
    pub fn list(&self, dir: DirType) -> Result<Vec<MessageBuilder>> {
        let entries = match dir {
            DirType::Current => self.maildir.list_cur(),
            DirType::New => self.maildir.list_new(),
        };

        let mut list = Vec::new();

        for entry in entries {
            let entry = entry?;

            let id = entry.id().to_string();

            let builder: MessageBuilder = entry.try_into()?;

            list.push(builder.id(id))
        }

        Ok(list)
    }

    pub fn retr<I: AsRef<str>>(&self, id: I) -> Result<MessageBuilder> {
        match self.maildir.find(id.as_ref()) {
            Some(mail_entry) => {
                let builder: MessageBuilder = mail_entry.try_into()?;

                Ok(builder.id(id.as_ref()))
            }
            None => {
                err!(
                    ErrorKind::MessageNotFound,
                    "Could not find a message with id {}",
                    id.as_ref()
                );
            }
        }
    }

    pub fn get_inbox(&self) -> Result<Node<Mailbox>> {
        let new = self.maildir.count_new();

        let stats = MailboxStats::new(new, new + self.maildir.count_cur());

        let inbox: Mailbox = stats.into();

        Ok(inbox.into())
    }
}

#[async_trait]
impl IncomingProtocol for MaildirClient {
    async fn send_keep_alive(&mut self) -> Result<()> {
        Ok(())
    }

    fn should_keep_alive(&self) -> bool {
        false
    }

    async fn get_mailbox_list(&mut self) -> Result<Node<Mailbox>> {
        self.get_inbox()
    }

    async fn get_mailbox(&mut self, _id: &str) -> Result<Node<Mailbox>> {
        self.get_inbox()
    }

    async fn rename_mailbox(&mut self, _old_name: &str, _new_name: &str) -> Result<()> {
        Ok(())
    }

    async fn create_mailbox(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn delete_mailbox(&mut self, _box_id: &str) -> Result<()> {
        Ok(())
    }

    async fn get_messages(
        &mut self,
        _box_id: &str,
        start: usize,
        end: usize,
    ) -> Result<Vec<Preview>> {
        let mut previews = Vec::new();

        for builder in self.list(DirType::Current)? {
            previews.push(builder.try_into()?)
        }

        for builder in self.list(DirType::New)? {
            previews.push(builder.try_into()?)
        }

        if previews.len() <= start {
            return Ok(Vec::new());
        }

        let end = end.min(previews.len());

        Ok(previews.drain(start..end).collect())
    }

    async fn get_message(&mut self, _box_id: &str, msg_id: &str) -> Result<Message> {
        let message = self.retr(msg_id)?;

        Ok(message.build()?)
    }

    async fn get_attachment(
        &mut self,
        box_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<Vec<u8>> {
        todo!()
    }

    async fn logout(&mut self) -> Result<()> {
        Ok(())
    }
}

pub fn create(dir: PathBuf) -> Result<Box<dyn IncomingProtocol + Send + Sync>> {
    let session = MaildirClient {
        maildir: Maildir::from(dir),
    };

    Ok(Box::new(session))
}
