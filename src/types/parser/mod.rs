pub mod address;
pub mod message;

const ALLOWED_HTML_TAGS: [&str; 71] = [
    "address",
    "article",
    "aside",
    "footer",
    "header",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hgroup",
    "main",
    "nav",
    "section",
    "blockquote",
    "dd",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "hr",
    "li",
    "main",
    "ol",
    "p",
    "pre",
    "ul",
    "a",
    "abbr",
    "b",
    "bdi",
    "bdo",
    "br",
    "cite",
    "code",
    "data",
    "dfn",
    "em",
    "i",
    "kbd",
    "mark",
    "q",
    "rb",
    "rp",
    "rt",
    "rtc",
    "ruby",
    "s",
    "samp",
    "small",
    "span",
    "strong",
    "sub",
    "sup",
    "time",
    "u",
    "var",
    "wbr",
    "caption",
    "col",
    "colgroup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "center",
];

const GENERIC_HTML_ATTRIBUTES: [&str; 12] = [
    "style",
    "width",
    "height",
    "border",
    "cellspacing",
    "cellpadding",
    "colspan",
    "id",
    "target",
    "data-x-style-url",
    "class",
    "align",
];

pub fn sanitize_html(dirty: &str) -> String {
    let clean = ammonia::Builder::new()
        .add_tags(ALLOWED_HTML_TAGS)
        .add_generic_attributes(GENERIC_HTML_ATTRIBUTES)
        .clean(dirty)
        .to_string();

    clean
}

pub fn sanitize_text(dirty: &str) -> String {
    ammonia::clean_text(dirty)
}

#[cfg(feature = "json")]
pub mod json {
    use serde::Serialize;

    use crate::error::{Error, ErrorKind, Result};

    pub fn to_json<T: ?Sized + Serialize>(value: &T) -> Result<String> {
        serde_json::to_string(value).map_err(|e| {
            Error::new(
                ErrorKind::SerializeJSON,
                format!("Failed to serialize data to json: {}", e),
            )
        })
    }
}
