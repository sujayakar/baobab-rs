use std::alloc::Layout;
use std::cmp;
use std::fmt;
use std::mem;
use std::marker::PhantomData;
use std::ops::Range;

use crate::packable::Header;
use crate::packed_node::PackedNode;

#[derive(Debug, Eq, PartialEq)]
pub enum NodeChildrenType {
    Empty,
    Pairs,
    Sparse,
    Dense,
}

impl NodeChildrenType {
    fn from_count(n: usize) -> Self {
        use NodeChildrenType::*;
        match n {
            0 => Empty,
            1..=32 => Pairs,
            33..=192 => Sparse,
            193..=256 => Dense,
            _ => panic!("Invalid number of children: {}", n),
        }
    }
}

pub struct NodeHeader<T> {
    prefix_byte: u8,
    children_byte: u8,
    marker: PhantomData<T>,
}

impl<T> Clone for NodeHeader<T> {
    fn clone(&self) -> Self {
        Self {
            prefix_byte: self.prefix_byte,
            children_byte: self.children_byte,
            marker: PhantomData,
        }
    }
}

impl<T> Copy for NodeHeader<T> {}

impl<T> fmt::Debug for NodeHeader<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("NodeHeader")
            .field("prefix_byte", &self.prefix_byte)
            .field("children_byte", &self.children_byte)
            .finish()
    }
}

impl<T> NodeHeader<T> {
    pub fn new(prefix_len: usize, num_children: usize, has_value: bool) -> Self {
        assert!(prefix_len < 64);
        let mut prefix_byte = prefix_len as u8;
        let children_byte;
        if num_children == 256 {
            prefix_byte |= 1 << 6;
            children_byte = 255;
        } else {
            children_byte = num_children as u8;
        }
        if has_value {
            prefix_byte |= 1 << 7;
        }
        Self { prefix_byte, children_byte, marker: PhantomData }
    }

    pub fn prefix_len(self) -> usize {
        let mask = (1 << 6) - 1;
        (self.prefix_byte & mask) as usize
    }

    pub fn num_children(self) -> usize {
        if self.prefix_byte & (1 << 6) != 0 {
            256
        } else {
            self.children_byte as usize
        }
    }

    fn has_value(self) -> bool {
        self.prefix_byte & (1 << 7) != 0
    }

    pub fn header_range(self) -> Range<usize> {
        0..mem::size_of::<Self>()
    }

    pub fn prefix_range(self) -> Range<usize> {
        let Range {
            end: header_end, ..
        } = self.header_range();
        header_end..(header_end + self.prefix_len())
    }

    pub fn children_type(self) -> NodeChildrenType {
        NodeChildrenType::from_count(self.num_children())
    }

    fn children_len(self) -> usize {
        let (overhead, pointers) = match self.children_type() {
            NodeChildrenType::Empty => (0, 0),
            NodeChildrenType::Pairs => (self.num_children(), self.num_children()),
            NodeChildrenType::Sparse => (32, self.num_children()),
            NodeChildrenType::Dense => (0, 256),
        };
        overhead + mem::size_of::<PackedNode<T>>() * pointers
    }

    pub fn children_range(self) -> Range<usize> {
        let Range {
            end: prefix_end, ..
        } = self.prefix_range();
        prefix_end..(prefix_end + self.children_len())
    }

    pub fn value_range(self) -> Option<Range<usize>> {
        if !self.has_value() {
            return None;
        }
        let Range {
            end: children_end, ..
        } = self.children_range();
        let align = mem::align_of::<T>();
        let value_start = (children_end + align - 1) / align * align;
        Some(value_start..(value_start + mem::size_of::<T>()))
    }

    fn alloc_size(self) -> usize {
        if let Some(value_range) = self.value_range() {
            value_range.end
        } else {
            self.children_range().end
        }
    }
}

impl<T> Header for NodeHeader<T> {
    fn layout(&self) -> Layout {
        let align = cmp::max(mem::align_of::<Self>(), mem::align_of::<T>());
        Layout::from_size_align(self.alloc_size(), align)
            .unwrap_or_else(|_| panic!("Invalid layout for {:?}", self))
    }
}

#[test]
fn test_sizes() {
    assert_eq!(mem::size_of::<NodeHeader<()>>(), 2);
    assert_eq!(mem::align_of::<NodeHeader<()>>(), 1);
}
