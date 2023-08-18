use std::collections::HashMap;

use crate::{error::Result, types::Headers};

pub fn parse_headers<B: AsRef<[u8]>>(response: B) -> Result<Headers> {
    let (parsed, _) = mailparse::parse_headers(response.as_ref())?;

    let mut headers: Headers = HashMap::new();

    for header in parsed.into_iter() {
        match headers.insert(header.get_key(), header.get_value()) {
            _ => {}
        }
    }

    Ok(headers)
}
