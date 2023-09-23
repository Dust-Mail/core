use std::path::{Path, PathBuf};

use maildir::Maildir;
use sled::Db;

use crate::error::{err, ErrorKind, Result};

use super::builder::MessageBuilder;

pub struct MailDirectory {
    dir: Maildir,
    id_mappings: Db,
}

impl MailDirectory {
    pub fn open<P: AsRef<str>>(path: P) -> Result<Self> {
        let dir: PathBuf = Path::new(path.as_ref()).into();

        let db = sled::open(dir.join("metadata"))?;

        let mail_dir = Self {
            dir: Maildir::from(dir),
            id_mappings: db,
        };

        Ok(mail_dir)
    }

    pub fn retr<I: AsRef<str>>(&self, id: I) -> Result<MessageBuilder> {
        match self
            .id_mappings
            .get(id.as_ref())?
            .map(|maildir_id| self.dir.find(std::str::from_utf8(&maildir_id).ok()?))
            .flatten()
        {
            Some(mail_entry) => Ok(mail_entry.try_into()?),
            None => {
                err!(
                    ErrorKind::MessageNotFound,
                    "Could not find a message with id {}",
                    id.as_ref()
                );
            }
        }
    }

    pub fn store<B: AsRef<[u8]>, I: AsRef<str>>(&mut self, id: I, data: B) -> Result<()> {
        self.dir.create_dirs()?;

        let maildir_id = self.dir.store_new(data.as_ref())?;

        self.id_mappings
            .insert(id.as_ref(), maildir_id.as_bytes())?;

        Ok(())
    }
}
