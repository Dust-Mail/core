use email::FromHeader;

use crate::{error::Result, types::Address};

pub fn address_list<H: Into<String>>(header: H) -> Result<Vec<Address>> {
    let address_list: Vec<email::Address> = Vec::from_header(header.into())?;

    let mut addresses = Vec::new();

    for address in address_list {
        match address {
            email::Address::Group(_name, list) => {
                for mailbox in list {
                    addresses.push(mailbox.into());
                }
            }
            email::Address::Mailbox(mailbox) => {
                addresses.push(mailbox.into());
            }
        }
    }

    Ok(addresses)
}
