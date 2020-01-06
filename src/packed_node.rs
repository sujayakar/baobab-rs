use std::mem;
use std::slice;

use crate::bitset::Bitset;
use crate::packable::{PackedBox, Header};
use crate::header::NodeChildrenType;
use crate::node::{Node, NodeChildren};

pub struct PackedNode<T> {
    ptr: Option<PackedBox<Node<T>>>,
}

impl<T> PackedNode<T> {
    pub fn empty() -> Self {
        Self { ptr: None }
    }

    fn new(node: Node<T>) -> Self {
        Self {
            ptr: Some(PackedBox::new(node)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ptr.is_none()
    }

    fn take(&mut self) -> Node<T> {
        match self.ptr.take() {
            None => Node {
                prefix: vec![],
                children: NodeChildren::Empty,
                value: None,
            },
            Some(p) => p.unpack(),
        }
    }

    #[allow(unused)]
    fn debug(&self) {
        let s = std::io::stdout();
        let mut l = s.lock();
        self.debug_(&"", &mut l).unwrap();
    }

    fn debug_(&self, indent: &str, out: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        let num_children = self
            .ptr
            .as_ref()
            .map(|p| p.header().num_children())
            .unwrap_or(0);
        let children_type = self
            .ptr
            .as_ref()
            .map(|p| p.header().children_type())
            .unwrap_or(NodeChildrenType::Pairs);

        let heap_usage = self
            .ptr
            .as_ref()
            .map(|p| p.header().layout().size())
            .unwrap_or(0);
        let full_prefix = self.prefix();
        let prefix = if full_prefix.len() > 4 {
            format!("{:?}...", &full_prefix[..4])
        } else {
            format!("{:?}", full_prefix)
        };
        write!(
            out,
            "Node {{ bytes: {}, prefix: {}, has_value: {:?}, children_type: {:?}({}) }}\n",
            heap_usage,
            prefix,
            self.has_value(),
            children_type,
            num_children
        )?;

        let indices = (0..=255)
            .filter_map(|i| self.lookup(i).map(|c| (i, c)))
            .filter(|(_, c)| !c.is_empty())
            .collect::<Vec<_>>();

        if let Some((last, init)) = indices.split_last() {
            let child_indent = format!("{} \u{2502}", indent);
            for (i, c) in init {
                write!(out, "{} \u{251C} {}: ", indent, i)?;
                c.debug_(&child_indent, out)?;
            }

            let (i, c) = last;
            write!(out, "{} \u{2514} {}: ", indent, i)?;
            let child_indent = format!("{}  ", indent);
            c.debug_(&child_indent, out)?;
        }
        Ok(())
    }

    pub fn prefix(&self) -> &[u8] {
        match self.ptr {
            None => &[],
            Some(ref p) => &p.slice()[p.header().prefix_range()],
        }
    }

    fn has_value(&self) -> bool {
        match self.ptr {
            None => false,
            Some(ref p) => p.header().value_range().is_some(),
        }
    }

    pub fn value(&self) -> Option<&T> {
        match self.ptr {
            None => None,
            Some(ref p) => {
                let header = p.header();
                let value_buf = &p.slice()[header.value_range()?];
                Some(unsafe { &*value_buf.as_ptr().cast() })
            }
        }
    }

    // The original tree...
    // ```
    //         o      prefix: abc
    //         | a             ^ split_at
    //         | b
    //         | c
    //         *      value: old_value
    //       / | \    children: old_children
    // ```
    // becomes...
    // ```
    //         o      prefix: a
    //         | a
    //         *      value: new_value
    //         | b    children: b -> new_child
    //         o      prefix: c
    //         | c
    //         *      value: old_value
    //       / | \    children: old_children
    // ```
    fn split_prefix(&mut self, split_at: usize, new_value: T) {
        let Node {
            prefix,
            children: old_children,
            value: old_value,
        } = self.take();

        let (parent_prefix, suffix) = prefix.split_at(split_at);
        let (&branch, child_prefix) = suffix.split_first().unwrap();

        let new_child = Node {
            prefix: child_prefix.to_owned(),
            children: old_children,
            value: old_value,
        };
        let new_parent = Node {
            prefix: parent_prefix.to_owned(),
            children: NodeChildren::one(branch, PackedNode::new(new_child)),
            value: Some(new_value),
        };
        *self = PackedNode::new(new_parent);
    }

    // The original tree...
    // ```
    //         o      prefix: abc
    //         | a
    //         | b
    //         | c
    //         *      value: old_value
    //       / | \    children: old_children
    // ```
    // becomes...
    // ```
    //           o
    //         a |
    //           *
    //       b /   \ d
    //        o     o
    //      c |     | e
    //        *     | f
    //      / | \   | g
    //              *
    // ```
    fn branch_prefix(
        &mut self,
        split_at: usize,
        key_branch: u8,
        key_remainder: &[u8],
        new_value: T,
    ) {
        let Node {
            prefix,
            children: old_children,
            value: old_value,
        } = self.take();

        let (parent_prefix, suffix) = prefix.split_at(split_at);

        // NB: "first" and "second" here are with respect to the diagram above.
        let (&first_branch, first_prefix) = suffix.split_first().unwrap();
        let (second_branch, second_prefix) = (key_branch, key_remainder);

        let first_child = Node {
            prefix: first_prefix.to_owned(),
            children: old_children,
            value: old_value,
        };
        let second_child = Node {
            prefix: second_prefix.to_owned(),
            children: NodeChildren::Empty,
            value: Some(new_value),
        };
        let new_parent = Node {
            prefix: parent_prefix.to_owned(),
            children: NodeChildren::two(
                first_branch,
                PackedNode::new(first_child),
                second_branch,
                PackedNode::new(second_child),
            ),
            value: None,
        };
        *self = PackedNode::new(new_parent);
    }

    fn set_value(&mut self, new_value: Option<T>) -> Option<T> {
        let Node {
            prefix,
            children,
            value: old_value,
        } = self.take();
        let new_node = Node {
            prefix,
            children,
            value: new_value,
        };
        *self = PackedNode::new(new_node);
        old_value
    }

    fn add_child(&mut self, key: u8, child: Node<T>) {
        let Node {
            prefix,
            children,
            value,
        } = self.take();
        let mut pairs = children.into_pairs();
        assert!(pairs.insert(key, PackedNode::new(child)).is_none());
        let new_node = Node {
            prefix,
            value,
            children: NodeChildren::from_pairs(pairs),
        };
        *self = PackedNode::new(new_node);
    }

    pub fn insert(&mut self, key: &[u8], value: T) -> Option<T> {
        // TODO: Why is it easy to write this recursively but hard to get the
        // borrow checker to accept the iterative loop version?
        // See https://users.rust-lang.org/t/how-do-you-remove-the-last-node-from-a-singly-linked-list/31805
        // The straightforward switch to a loop works with `-Z polonius`.
        let mut key_iter = key.iter();
        for (i, &byte) in self.prefix().iter().enumerate() {
            match key_iter.next() {
                // Split current node into a branching node with two children.
                Some(&key_byte) if key_byte != byte => {
                    self.branch_prefix(i, key_byte, key_iter.as_slice(), value);
                    return None;
                }
                // Split current node into a branching node with one child.
                None => {
                    self.split_prefix(i, value);
                    return None;
                }
                Some(..) => continue,
            }
        }
        let branch_byte = match key_iter.next() {
            // Set value on current node.
            None => return self.set_value(Some(value)),
            Some(&k) => k,
        };
        match self.lookup_mut(branch_byte) {
            None => {
                let new_child = Node {
                    prefix: key_iter.as_slice().to_owned(),
                    children: NodeChildren::Empty,
                    value: Some(value),
                };
                self.add_child(branch_byte, new_child);
                None
            }
            Some(next_node) => next_node.insert(key_iter.as_slice(), value),
        }
    }

    pub fn remove(&mut self, key: &[u8]) -> Option<T> {
        let mut key_iter = key.iter();

        for &byte in self.prefix() {
            match key_iter.next() {
                Some(&key_byte) if key_byte == byte => continue,
                _ => return None,
            }
        }
        let branch_byte = match key_iter.next() {
            None => return self.set_value(None),
            Some(&k) => k,
        };
        match self.lookup_mut(branch_byte) {
            None => return None,
            Some(next_node) => next_node.remove(key_iter.as_slice()),
        }
    }

    pub fn lookup_mut(&mut self, byte: u8) -> Option<&mut PackedNode<T>> {
        self.lookup(byte)
            .map(|r| unsafe { &mut *(r as *const _ as *mut _) })
    }

    pub fn lookup(&self, byte: u8) -> Option<&PackedNode<T>> {
        let ptr = match self.ptr {
            None => return None,
            Some(ref p) => p,
        };
        let header = ptr.header();
        let children_buf = &ptr.slice()[header.children_range()];
        match header.children_type() {
            NodeChildrenType::Empty => None,
            NodeChildrenType::Pairs => {
                let n = header.num_children();
                let values_len = n * mem::size_of::<PackedNode<T>>();
                let values_slice = &children_buf[n..][..values_len];
                let values: &[PackedNode<T>] = unsafe {
                    slice::from_raw_parts(values_slice.as_ptr().cast(), n)
                };
                for (i, &k) in children_buf[..n].iter().enumerate() {
                    if k == byte {
                        return Some(&values[i]);
                    }
                }
                None
            }
            NodeChildrenType::Sparse => {
                let bitset_len = mem::size_of::<Bitset>();
                let bitset: &Bitset = unsafe { &*children_buf[..bitset_len].as_ptr().cast() };
                let values_len = header.num_children() * mem::size_of::<PackedNode<T>>();
                let values_slice = &children_buf[bitset_len..][..values_len];
                let values: &[PackedNode<T>] = unsafe {
                    slice::from_raw_parts(values_slice.as_ptr().cast(), header.num_children())
                };
                let rank = bitset.query(byte)?;
                Some(&values[rank])
            }
            NodeChildrenType::Dense => {
                let table_len = mem::size_of::<[PackedNode<T>; 256]>();
                let table: &[PackedNode<T>; 256] =
                    unsafe { &*children_buf[..table_len].as_ptr().cast() };
                Some(&table[byte as usize])
            }
        }
    }
}
