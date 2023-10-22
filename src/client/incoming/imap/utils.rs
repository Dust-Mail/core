use async_imap::types::Name;

use crate::{
    client::incoming::types::mailbox::Mailbox,
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
