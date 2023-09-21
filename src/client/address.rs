#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::parser;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmailAddress {
    name: Option<String>,
    email: String,
}

impl EmailAddress {
    pub fn email(&self) -> &str {
        &self.email
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }
}

impl Into<Address> for EmailAddress {
    fn into(self) -> Address {
        Address::Single(self)
    }
}

impl From<email::Mailbox> for EmailAddress {
    fn from(mailbox: email::Mailbox) -> Self {
        Self::new(mailbox.name, mailbox.address)
    }
}

impl EmailAddress {
    pub fn new(name: Option<String>, email: String) -> Self {
        Self { name, email }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Address {
    Group {
        name: Option<String>,
        list: Vec<Address>,
    },
    Single(EmailAddress),
}

impl Address {
    pub fn single(name: Option<String>, address: String) -> Self {
        Address::Single(EmailAddress::new(name, address))
    }

    pub fn group(name: Option<String>, list: Vec<Self>) -> Self {
        Self::Group { name, list }
    }

    pub fn as_list(&self) -> Vec<&EmailAddress> {
        let mut addresses = Vec::new();

        match self {
            Address::Single(addr) => addresses.push(addr),
            Address::Group { list, .. } => {
                for addr in list {
                    addresses.append(&mut addr.as_list());
                }
            }
        }

        addresses
    }

    pub fn first(&self) -> Option<&EmailAddress> {
        match self {
            Address::Group { list, .. } => {
                for addr in list {
                    if let Some(addr) = addr.first() {
                        return Some(addr);
                    }
                }

                None
            }
            Address::Single(addr) => Some(addr),
        }
    }
}

impl From<email::Address> for Address {
    fn from(address: email::Address) -> Self {
        match address {
            email::Address::Group(name, list) => Self::group(
                Some(name),
                list.into_iter()
                    .map(|item| EmailAddress::from(item).into())
                    .collect::<Vec<Self>>(),
            ),
            email::Address::Mailbox(mailbox) => Self::Single(mailbox.into()),
        }
    }
}

impl<A: Into<Address>> From<Vec<A>> for Address {
    fn from(mut list: Vec<A>) -> Self {
        if list.len() == 1 {
            let first = list.remove(0);

            return first.into();
        }

        let iter = list.into_iter();

        Self::group(None, iter.map(|addr| addr.into()).collect())
    }
}

impl<N: Into<String>, A: Into<String>> From<(N, A)> for Address {
    fn from((name, address): (N, A)) -> Self {
        Self::Single(EmailAddress::new(Some(name.into()), address.into()))
    }
}

impl<'a> Into<mail_builder::headers::address::Address<'a>> for Address {
    fn into(self) -> mail_builder::headers::address::Address<'a> {
        match self {
            Address::Group { name, list } => mail_builder::headers::address::Address::new_group(
                name,
                list.into_iter().map(|item| item.into()).collect(),
            ),
            Address::Single(address) => {
                mail_builder::headers::address::Address::new_address(address.name, address.email)
            }
        }
    }
}

impl Address {
    pub fn from_header<H: Into<String>>(header: H) -> Result<Vec<Self>> {
        parser::address::address_list(header)
    }
}
