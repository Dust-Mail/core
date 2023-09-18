use std::{result, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

use super::parser;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Address {
    name: Option<String>,
    address: String,
}

impl<N: Into<String>, A: Into<String>> From<(N, A)> for Address {
    fn from((name, address): (N, A)) -> Self {
        Self::new(Some(name.into()), address.into())
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        Ok(email::Mailbox::from_str(s)?.into())
    }
}

impl Into<email::Address> for Address {
    fn into(self) -> email::Address {
        match self.name {
            Some(name) => email::Address::new_mailbox_with_name(name, self.address),
            None => email::Address::new_mailbox(self.address),
        }
    }
}

impl Into<email::Mailbox> for Address {
    fn into(self) -> email::Mailbox {
        match self.name {
            Some(name) => email::Mailbox::new_with_name(name, self.address),
            None => email::Mailbox::new(self.address),
        }
    }
}

impl From<email::Mailbox> for Address {
    fn from(mailbox: email::Mailbox) -> Self {
        Address::new(mailbox.name, mailbox.address)
    }
}

impl Address {
    pub fn new(name: Option<String>, address: String) -> Self {
        Self { name, address }
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn full(&self) -> String {
        match self.name.as_ref() {
            Some(name) => format!("{} <{}>", name, self.address),
            None => self.address.to_string(),
        }
    }

    pub fn from_header<H: Into<String>>(header: H) -> Result<Vec<Self>> {
        parser::address::address_list(header)
    }
}
