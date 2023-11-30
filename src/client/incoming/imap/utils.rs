use std::{collections::HashMap, fmt::Display, str::FromStr};

use async_imap::{
    imap_proto::{
        BodyContentCommon, BodyContentSinglePart, BodyStructure, ContentDisposition, ContentType,
        SectionPath,
    },
    types::Name,
};
use mime::Mime;

use crate::{
    client::{attachment::Attachment, incoming::types::mailbox::Mailbox},
    error,
    tree::{Find, Node},
};

pub struct MailboxFinder(String);

impl Find<Mailbox> for MailboxFinder {
    fn find(&self, item: &Mailbox) -> bool {
        item.id().to_lowercase() == self.0
    }
}

impl MailboxFinder {
    pub fn with_id<I: Into<String>>(id: I) -> Self {
        Self(Into::<String>::into(id).to_lowercase())
    }
}

pub fn build_mailbox_tree<L: IntoIterator<Item = Name>>(names: L) -> Node<Mailbox> {
    let names: Vec<Name> = names.into_iter().collect();

    let mut root: Node<Mailbox> = Node::empty_root();

    for name in &names {
        match name.delimiter() {
            Some(delimiter) => {
                let parts: Vec<_> = name.name().split(delimiter).collect();

                add_children(&names, &mut root, delimiter, parts, 0)
            }
            None => {
                root.insert(Node::empty_branch(name.into()));
            }
        }
    }

    Node::create_leaves(root)
}

fn add_children(
    names: &Vec<Name>,
    node: &mut Node<Mailbox>,
    delimiter: &str,
    parts: Vec<&str>,
    index: usize,
) {
    if let Some(_) = parts.get(index) {
        let id = parts[0..index + 1].join(delimiter);

        let child = node.find_mut(&MailboxFinder::with_id(&id));

        match child {
            Some(child) => add_children(names, child, delimiter, parts, index + 1),
            None => {
                if let Some(found) = names.iter().find(|child| child.name() == id) {
                    node.insert(Node::empty_branch(found.into()));

                    add_children(names, node, delimiter, parts, index);
                }
            }
        }
    }
}

const PART_NUMBER_DELIM: &str = ".";

#[derive(Clone, Debug)]
pub struct PartNumber {
    inner: Vec<usize>,
}

impl FromStr for PartNumber {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<usize> = Vec::new();

        for part in s.split(PART_NUMBER_DELIM) {
            parts.push(part.parse()?)
        }

        Ok(PartNumber { inner: parts })
    }
}

impl Into<SectionPath> for PartNumber {
    fn into(self) -> SectionPath {
        SectionPath::Part(self.inner.into_iter().map(|u| u as u32).collect(), None)
    }
}

impl PartNumber {
    fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn clone_and_add(&self, i: usize) -> Self {
        let mut cloned = self.clone();

        cloned.add(i);

        cloned
    }

    pub fn add(&mut self, i: usize) {
        self.inner.push(i);
    }
}

impl Display for PartNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.inner
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(PART_NUMBER_DELIM)
        )
    }
}

pub struct BodyStructureParser<'a> {
    structure: &'a BodyStructure<'a>,
}

impl<'a> From<&'a BodyStructure<'a>> for BodyStructureParser<'a> {
    fn from(value: &'a BodyStructure) -> Self {
        Self { structure: value }
    }
}

impl<'a> BodyStructureParser<'a> {
    fn extract_file_name(disposition: &ContentDisposition) -> Option<String> {
        if let Some(params) = &disposition.params {
            let params: HashMap<_, _> = params
                .iter()
                .map(|(key, value)| (key.to_lowercase(), value))
                .collect();

            if let Some(file_name) = params.get("filename") {
                return Some(file_name.to_string());
            }
        }

        None
    }

    fn extract_attachment(
        part_number: PartNumber,
        common: &BodyContentCommon<'a>,
        other: &BodyContentSinglePart,
    ) -> Option<Attachment> {
        if let Some(disposition) = &common.disposition {
            if disposition.ty.to_lowercase() == String::from("attachment") {
                let file_name = Self::extract_file_name(disposition);

                let size = other.octets as usize;

                let attachment = Attachment::new(part_number.to_string(), file_name, size);

                return Some(attachment);
            }
        }

        None
    }

    fn extract_attachments_rec(
        body_structure: &'a BodyStructure<'a>,
        part_number: PartNumber,
    ) -> Vec<Attachment> {
        let mut attachments = Vec::new();

        match body_structure {
            BodyStructure::Multipart { bodies, .. } => {
                for (i, body) in bodies.into_iter().enumerate() {
                    for attachment in
                        Self::extract_attachments_rec(body, part_number.clone_and_add(i + 1))
                    {
                        attachments.push(attachment);
                    }
                }
            }
            BodyStructure::Message { common, other, .. } => {
                if let Some(attachment) = Self::extract_attachment(part_number, common, other) {
                    attachments.push(attachment)
                }
            }
            BodyStructure::Basic { common, other, .. } => {
                if let Some(attachment) = Self::extract_attachment(part_number, common, other) {
                    attachments.push(attachment)
                }
            }
            BodyStructure::Text { common, other, .. } => {
                if let Some(attachment) = Self::extract_attachment(part_number, common, other) {
                    attachments.push(attachment)
                }
            }
        };

        attachments
    }

    /// Extracts information about the file attachments from the body structure of a IMAP message.
    pub fn extract_attachments(&self) -> Vec<Attachment> {
        Self::extract_attachments_rec(self.structure, PartNumber::new())
    }

    fn check_mime_type(mime: &Mime, content_type: &ContentType) -> bool {
        let to_match: Mime = match format!("{}/{}", content_type.ty, content_type.subtype).parse() {
            Ok(mime) => mime,
            Err(_) => return false,
        };

        mime == &to_match
    }

    fn find_part_number_rec(
        body_structure: &BodyStructure,
        mime: &Mime,
        part_number: PartNumber,
    ) -> Option<PartNumber> {
        match body_structure {
            BodyStructure::Multipart { bodies, .. } => {
                for (i, body) in bodies.into_iter().enumerate() {
                    if let Some(part_number) =
                        Self::find_part_number_rec(body, mime, part_number.clone_and_add(i + 1))
                    {
                        return Some(part_number);
                    }
                }
            }
            BodyStructure::Message { common, .. } => {
                if Self::check_mime_type(mime, &common.ty) {
                    return Some(part_number);
                }
            }
            BodyStructure::Basic { common, .. } => {
                if Self::check_mime_type(mime, &common.ty) {
                    return Some(part_number);
                }
            }
            BodyStructure::Text { common, .. } => {
                if Self::check_mime_type(mime, &common.ty) {
                    return Some(part_number);
                }
            }
        };

        None
    }

    pub fn find_part_number_for(&self, mime_type: Mime) -> Option<PartNumber> {
        Self::find_part_number_rec(self.structure, &mime_type, PartNumber::new())
    }
}
