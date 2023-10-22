#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Node<T: Default> {
    Branch { data: T, children: Vec<Node<T>> },
    Root(Vec<Node<T>>),
    Leaf(T),
}

impl<T: Default> Default for Node<T> {
    fn default() -> Self {
        Self::Leaf(T::default())
    }
}

impl<T: Default> From<T> for Node<T> {
    fn from(value: T) -> Self {
        Self::Leaf(value)
    }
}

pub trait Find<T> {
    fn find(&self, item: &T) -> bool;
}

impl<T: Default> Node<T> {
    pub fn branch<C: IntoIterator<Item = Node<T>>>(data: T, children: C) -> Self {
        Node::Branch {
            data,
            children: children.into_iter().collect(),
        }
    }

    pub fn empty_root() -> Self {
        Node::Root(Vec::new())
    }

    pub fn empty_branch(data: T) -> Self {
        Node::branch(data, Vec::new())
    }

    pub fn leaf(data: T) -> Self {
        Node::Leaf(data)
    }

    pub fn data(&self) -> Option<&T> {
        match &self {
            Node::Branch { data, .. } | Node::Leaf(data) => Some(data),
            _ => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut T> {
        match self {
            Node::Branch { data, .. } | Node::Leaf(data) => Some(data),
            _ => None,
        }
    }

    pub fn into_data(self) -> Option<T> {
        match self {
            Node::Branch { data, .. } | Node::Leaf(data) => Some(data),
            _ => None,
        }
    }

    /// Inserts a node if this node is capable of containing children
    pub fn insert(&mut self, node: Node<T>) -> bool {
        match self {
            Node::Root(children) | Node::Branch { children, .. } => {
                children.push(node);

                true
            }
            _ => false,
        }
    }

    /// Turns all (sub)branches with no children into leaves
    pub fn create_leaves(self) -> Self {
        match self {
            Node::Root(mut children) if children.len() == 1 => {
                Node::create_leaves(children.remove(0))
            }
            Node::Branch { data, children } if children.is_empty() => Node::Leaf(data),
            Node::Branch { children, data } => Node::Branch {
                data,
                children: children.into_iter().map(Node::create_leaves).collect(),
            },
            Node::Root(children) => {
                Node::Root(children.into_iter().map(Node::create_leaves).collect())
            }
            _ => self,
        }
    }

    pub fn find<P: Find<T>>(&self, predicate: &P) -> Option<&Self> {
        match self {
            Node::Leaf(data) | Node::Branch { data, .. } if predicate.find(data) => Some(self),
            Node::Root(children) | Node::Branch { children, .. } => {
                for child in children {
                    if let Some(found) = Self::find(child, predicate) {
                        return Some(found);
                    }
                }

                None
            }
            _ => None,
        }
    }

    pub fn find_mut<P: Find<T>>(&mut self, predicate: &P) -> Option<&mut Self> {
        match self {
            Node::Leaf(data) | Node::Branch { data, .. } if predicate.find(&data) => Some(self),
            Node::Root(children) | Node::Branch { children, .. } => {
                for child in children {
                    if let Some(found) = Self::find_mut(child, predicate) {
                        return Some(found);
                    }
                }

                None
            }
            _ => None,
        }
    }

    pub fn into_find<P: Find<T>>(self, predicate: &P) -> Option<Self> {
        match self {
            Node::Leaf(ref data) | Node::Branch { ref data, .. } if predicate.find(data) => {
                Some(self)
            }
            Node::Root(children) | Node::Branch { children, .. } => {
                for child in children {
                    if let Some(found) = Self::into_find(child, predicate) {
                        return Some(found);
                    }
                }

                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct GreaterThanThree;

    impl Find<i32> for GreaterThanThree {
        fn find(&self, item: &i32) -> bool {
            *item > 3
        }
    }

    struct GreaterThanFour;

    impl Find<i32> for GreaterThanFour {
        fn find(&self, item: &i32) -> bool {
            *item > 4
        }
    }

    #[test]
    fn test_find() {
        let test_tree = Node::branch(1, vec![2.into(), 3.into(), Node::branch(4, Vec::new())]);

        assert_eq!(Some(&4), test_tree.find(&GreaterThanThree).unwrap().data());

        assert_eq!(None, test_tree.find(&GreaterThanFour));
    }
}
