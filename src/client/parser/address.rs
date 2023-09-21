use email::FromHeader;

use crate::{client::address::Address, error::Result};

pub fn address_list<H: Into<String>>(header: H) -> Result<Vec<Address>> {
    let address_list: Vec<email::Address> = Vec::from_header(header.into())?;

    Ok(address_list.into_iter().map(|addr| addr.into()).collect())
}
