use std::mem;
use std::slice;

use crate::bitset::Bitset;
use crate::packable::{PackedBox, Header};
use crate::header::NodeChildrenType;
use crate::node::{Node, NodeChildren};

pub struct PackedNode<T> {
    pub(crate) ptr: Option<PackedBox<Node<T>>>,
}

impl<T> PackedNode<T> {
    pub fn empty() -> Self {
        Self { ptr: None }
    }

    pub fn new(node: Node<T>) -> Self {
        Self {
            ptr: Some(PackedBox::new(node)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ptr.is_none()
    }

    pub fn take(&mut self) -> Node<T> {
        match self.ptr.take() {
            None => Node {
                prefix: vec![],
                children: NodeChildren::Empty,
                value: None,
            },
            Some(p) => p.unpack(),
        }
    }

    pub fn set_value(&mut self, new_value: Option<T>) -> Option<T> {
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

    pub fn add_child(&mut self, key: u8, child: Node<T>) {
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

    pub fn prefix(&self) -> &[u8] {
        match self.ptr {
            None => &[],
            Some(ref p) => &p.slice()[p.header().prefix_range()],
        }
    }

    pub fn has_value(&self) -> bool {
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

    pub fn debug(&self, indent: &str, out: &mut impl std::io::Write) -> Result<(), std::io::Error> {
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
        write!(
            out,
            "Node {{ bytes: {}, prefix: {:?}, has_value: {:?}, children_type: {:?}({}) }}\n",
            heap_usage,
            self.prefix(),
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
                c.debug(&child_indent, out)?;
            }

            let (i, c) = last;
            write!(out, "{} \u{2514} {}: ", indent, i)?;
            let child_indent = format!("{}  ", indent);
            c.debug(&child_indent, out)?;
        }
        Ok(())
    }
}
