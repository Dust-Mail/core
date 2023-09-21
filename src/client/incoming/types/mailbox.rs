use std::{collections::HashMap, fmt::Display};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "imap")]
use async_imap::types::{
    Mailbox as ImapCounts, Name as ImapMailBox, NameAttribute as ImapBoxAttribute,
};

const DEFAULT_DELIMITER: &str = ".";

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mailbox {
    stats: Option<MailboxStats>,
    delimiter: Option<String>,
    children: Vec<Mailbox>,
    selectable: bool,
    id: String,
    name: String,
}

impl PartialEq for Mailbox {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Mailbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let indents = match self.id_delimited() {
            Some(delimited) => delimited.len().saturating_sub(1),
            None => 0,
        };

        for i in 0..indents {
            if i == indents - 1 {
                write!(f, "|--")?;
            } else {
                write!(f, "   ")?;
            };
        }

        write!(f, "{}", self.name())?;

        if let Some(counts) = self.stats() {
            write!(f, ": {}", counts)?;
        }

        for child in &self.children {
            write!(f, "\r\n")?;

            child.fmt(f)?;
        }

        Ok(())
    }
}

impl Mailbox {
    pub fn new<S: Into<String>>(
        counts: Option<MailboxStats>,
        delimiter: Option<String>,
        children: Vec<Mailbox>,
        selectable: bool,
        id: S,
        name: S,
    ) -> Self {
        Self {
            stats: counts,
            delimiter,
            children,
            selectable,
            id: id.into(),
            name: name.into(),
        }
    }

    /// A struct containing some info about the message counts in this mailbox.
    pub fn stats(&self) -> Option<&MailboxStats> {
        self.stats.as_ref()
    }

    pub fn add_stats(&mut self, stats: MailboxStats) {
        self.stats = Some(stats);
    }

    /// Whether the mailbox contains messages and can be selected.
    pub fn selectable(&self) -> &bool {
        &self.selectable
    }

    /// The name delimiter in this mailbox that indicates the hierachy in the id.
    pub fn delimiter(&self) -> Option<&str> {
        match &self.delimiter {
            Some(delimiter) => Some(delimiter),
            None => None,
        }
    }

    pub fn children(&self) -> &Vec<Mailbox> {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut Vec<Mailbox> {
        &mut self.children
    }

    /// A unique id for this mailbox.
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn id_delimited(&self) -> Option<Vec<&str>> {
        if let Some(delimiter) = self.delimiter.as_ref() {
            let split = self.id.split(delimiter);

            return Some(split.collect());
        };

        None
    }

    /// The mailbox name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(feature = "imap")]
impl From<ImapMailBox> for Mailbox {
    fn from(imap_mailbox: ImapMailBox) -> Self {
        // Whether the inbox is selectable
        let selectable = !imap_mailbox
            .attributes()
            .contains(&ImapBoxAttribute::NoSelect);

        // Create an owned string if the delimiter is specified
        let delimiter = imap_mailbox
            .delimiter()
            .map(|delimiter| delimiter.to_string());

        let id = imap_mailbox.name().to_string();

        // Split the id on the delimiter (using the default delimiter if it is not specified) and grab the last item
        // Example: 'INBOX.test.spam' becomes 'spam' if the delimiter is '.'
        let name = id
            .split(
                delimiter
                    .as_ref()
                    .unwrap_or(&String::from(DEFAULT_DELIMITER)),
            )
            .last()
            .unwrap_or("Unknown")
            .to_string();

        Self {
            delimiter,
            id,
            selectable,
            name,
            stats: None,
            children: vec![],
        }
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            stats: Some(MailboxStats::default()),
            delimiter: None,
            id: String::new(),
            name: String::new(),
            selectable: true,
        }
    }
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// A struct that holds the count for the total amount messages and the total amount of unseen messages in a mailbox
pub struct MailboxStats {
    unseen: usize,
    total: usize,
}

impl Display for MailboxStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Total messages: {} | Unseen messages: {}",
            self.total, self.unseen,
        )
    }
}

impl MailboxStats {
    pub fn new(unseen: usize, total: usize) -> Self {
        MailboxStats { unseen, total }
    }

    /// The total amount of message that have not been read in this mailbox
    pub fn unseen(&self) -> usize {
        self.unseen
    }

    /// The total amount of messages in this mailbox
    pub fn total(&self) -> usize {
        self.total
    }
}

#[cfg(feature = "imap")]
impl From<ImapCounts> for MailboxStats {
    fn from(imap_counts: ImapCounts) -> Self {
        Self::new(
            imap_counts.unseen.unwrap_or(0) as usize,
            imap_counts.exists as usize,
        )
    }
}

/// A struct representing a list of all of the mailboxes in a user's account.
#[derive(Debug)]
pub struct MailboxList {
    list: Vec<Mailbox>,
}

impl Default for MailboxList {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<L: IntoIterator<Item = impl Into<Mailbox>>> From<L> for MailboxList {
    fn from(iter: L) -> Self {
        Self::new(iter.into_iter().map(|mailbox| mailbox.into()).collect())
    }
}

impl Display for MailboxList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mailbox tree:")?;

        for node in &self.list {
            write!(f, "\r\n")?;
            node.fmt(f)?;
        }

        Ok(())
    }
}

impl MailboxList {
    pub fn new(list: Vec<Mailbox>) -> Self {
        // We must ensure that we have a tree like structure to make sure that our get_box function will work.
        let folder_tree = Self::build_folder_tree(list);

        Self { list: folder_tree }
    }

    /// This is a function that takes an array of mailboxes (a planar graph) and builds it into a folder tree of mailboxes.
    /// In the case that there is a mailbox present that has children, the children must also be present in the given array of mailboxes.
    fn build_folder_tree(planar_graph: Vec<Mailbox>) -> Vec<Mailbox> {
        let mut folders: HashMap<String, MailBoxNode> = HashMap::new();

        for folder in planar_graph.iter() {
            match folder.delimiter() {
                Some(delimiter) => {
                    let parts: Vec<_> = folder.id().split(delimiter).collect();

                    let mut current: Option<&mut MailBoxNode> = None;

                    for part in parts {
                        let id = match current.as_ref() {
                            Some(current) => format!("{}{}{}", current.id(), delimiter, part),
                            None => String::from(part),
                        };

                        if let Some(current_box) =
                            planar_graph.iter().find(|mailbox| mailbox.id() == &id)
                        {
                            let children = match current {
                                Some(current) => current.children_mut(),
                                None => &mut folders,
                            };

                            current = Some(
                                children
                                    .entry(String::from(part))
                                    .or_insert(MailBoxNode::from(current_box.clone())),
                            );
                        }
                    }
                }
                None => {
                    folders.insert(folder.id().to_string(), MailBoxNode::from(folder.clone()));
                }
            }
        }

        folders.into_iter().map(|(_, value)| value.into()).collect()
    }

    pub fn to_vec(self) -> Vec<Mailbox> {
        self.list
    }

    pub fn get_vec(&self) -> &Vec<Mailbox> {
        &self.list
    }

    pub fn get_vec_mut(&mut self) -> &mut Vec<Mailbox> {
        &mut self.list
    }

    pub fn get_box<S: AsRef<str>>(&self, box_id: S) -> Option<&Mailbox> {
        Self::find_box_in_list(&self.list, box_id)
    }

    pub fn get_box_mut<S: AsRef<str>>(&mut self, box_id: S) -> Option<&mut Mailbox> {
        Self::find_box_in_list_mut(&mut self.list, box_id)
    }

    /// Finds a mailbox with a given id in a tree-like array list using breadth-first search
    fn find_box_in_list_mut<'a, S: AsRef<str>>(
        list: &'a mut Vec<Mailbox>,
        box_id: S,
    ) -> Option<&'a mut Mailbox> {
        if list.len() < 1 {
            return None;
        };

        let mut list_iter_mut = list.iter_mut();

        let found = list_iter_mut.find(|mailbox| mailbox.id() == box_id.as_ref());

        if found.is_none() {
            let found = list_iter_mut.find_map(|mailbox| {
                Self::find_box_in_list_mut(mailbox.children_mut(), box_id.as_ref())
            });

            found
        } else {
            found
        }
    }

    /// Finds a mailbox with a given id in a tree-like array list using breadth-first search
    fn find_box_in_list<'a, S: AsRef<str>>(
        list: &'a Vec<Mailbox>,
        box_id: S,
    ) -> Option<&'a Mailbox> {
        if list.len() < 1 {
            return None;
        };

        let found = list.iter().find(|mailbox| mailbox.id() == box_id.as_ref());

        if found.is_some() {
            found
        } else {
            list.iter()
                .filter_map(|mailbox| Self::find_box_in_list(mailbox.children(), box_id.as_ref()))
                .find(|mailbox| mailbox.id() == box_id.as_ref())
        }
    }
}

#[derive(Debug)]
/// A struct useful for building a folder tree structure out of a flat mailbox array.
pub struct MailBoxNode {
    counts: Option<MailboxStats>,
    delimiter: Option<String>,
    children: HashMap<String, MailBoxNode>,
    selectable: bool,
    id: String,
    name: String,
}

impl MailBoxNode {
    pub fn children_mut(&mut self) -> &mut HashMap<String, MailBoxNode> {
        &mut self.children
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl From<Mailbox> for MailBoxNode {
    /// Go from a planar mailbox (expects no children) to a mailbox tree
    fn from(mailbox: Mailbox) -> Self {
        let mut children = HashMap::new();

        for child in mailbox.children {
            match child.id_delimited() {
                Some(delimited) => {
                    if let Some(id) = delimited.last() {
                        children.insert(id.to_string(), MailBoxNode::from(child));
                    }
                }
                None => {
                    let id = child.id.clone();
                    let node = MailBoxNode::from(child);

                    children.insert(id, node);
                }
            }
        }

        Self {
            children,
            counts: mailbox.stats,
            delimiter: mailbox.delimiter,
            id: mailbox.id,
            name: mailbox.name,
            selectable: mailbox.selectable,
        }
    }
}

impl Into<Mailbox> for MailBoxNode {
    fn into(self) -> Mailbox {
        let children: Vec<Mailbox> = self
            .children
            .into_iter()
            .map(|(_, value)| value.into())
            .collect();

        Mailbox::new(
            self.counts,
            self.delimiter,
            children,
            self.selectable,
            self.id,
            self.name,
        )
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn find_box() {
        let delimiter = Some(String::from("."));

        let box1 = Mailbox::new(None, delimiter.clone(), vec![], true, "box1", "box1");

        let box3 = Mailbox::new(None, delimiter.clone(), vec![], true, "box2.box1", "box3");

        let box4 = Mailbox::new(None, delimiter.clone(), vec![], true, "box2.box2", "box4");

        let box2 = Mailbox::new(
            None,
            delimiter.clone(),
            vec![box3.clone(), box4.clone()],
            true,
            "box2",
            "box2",
        );

        let mock_boxes = vec![box1.clone(), box2.clone()];

        assert_eq!(
            MailboxList::find_box_in_list(&mock_boxes, "box1").unwrap(),
            &box1
        );
        assert_eq!(
            MailboxList::find_box_in_list(&mock_boxes, "box2").unwrap(),
            &box2
        );
        assert_eq!(
            MailboxList::find_box_in_list(&mock_boxes, "box2.box1").unwrap(),
            &box3
        );
        assert_eq!(
            MailboxList::find_box_in_list(&mock_boxes, "box2.box2").unwrap(),
            &box4
        );

        assert_eq!(MailboxList::find_box_in_list(&mock_boxes, "box3"), None);
    }

    #[test]
    fn mailbox_display() {
        let delimiter = Some(String::from("."));

        let box1 = Mailbox::new(None, delimiter.clone(), vec![], true, "box1", "box1");

        println!("{}", box1);

        let box5 = Mailbox::new(
            Some(MailboxStats::new(30, 50)),
            delimiter.clone(),
            vec![],
            true,
            "box2.box1.box1",
            "box5",
        );

        let box6 = Mailbox::new(
            Some(MailboxStats::new(30, 50)),
            delimiter.clone(),
            vec![],
            true,
            "box2.box1.box2",
            "box6",
        );

        let box3 = Mailbox::new(
            None,
            delimiter.clone(),
            vec![box5, box6],
            true,
            "box2.box1",
            "box3",
        );

        let box4 = Mailbox::new(None, delimiter.clone(), vec![], true, "box2.box2", "box4");

        let box2 = Mailbox::new(
            None,
            delimiter.clone(),
            vec![box3, box4],
            true,
            "box2",
            "box2",
        );

        println!("{}", box2);

        let tree: MailboxList = vec![box2, box1].into();

        println!("{}", tree)
    }
}
