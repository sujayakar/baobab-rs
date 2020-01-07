use std::collections::BTreeMap;
use std::mem;

use crate::bitset::Bitset;
use crate::header::{NodeHeader, NodeChildrenType};
use crate::packable::PackableStruct;
use crate::packed_node::PackedNode;

pub struct Node<T> {
    pub prefix: Vec<u8>,
    pub children: NodeChildren<T>,
    pub value: Option<T>,
}

impl<T> PackableStruct for Node<T> {
    type Header = NodeHeader<T>;

    fn header(&self) -> NodeHeader<T> {
        NodeHeader::new(self.prefix.len(), self.children.len(), self.value.is_some())
    }

    fn pack(self, header: NodeHeader<T>, buf: &mut [u8]) {
        let Self {
            prefix,
            children,
            value,
        } = self;

        unsafe {
            buf[header.header_range()]
                .as_mut_ptr()
                .cast::<NodeHeader<T>>()
                .write(header);
        }

        buf[header.prefix_range()].copy_from_slice(&prefix[..]);

        #[allow(unused_assignments)]
        {
            let mut children_buf = &mut buf[header.children_range()];
            assert_eq!(children.structure_type(), header.children_type());
            match children {
                NodeChildren::Empty => (),
                NodeChildren::Pairs { keys, values } => {
                    assert_eq!(keys.len(), values.len());
                    for k in keys {
                        children_buf[0] = k;
                        children_buf = &mut children_buf[1..];
                    }
                    for v in values {
                        unsafe {
                            children_buf.as_mut_ptr().cast::<PackedNode<T>>().write(v);
                        }
                        children_buf = &mut children_buf[mem::size_of::<PackedNode<T>>()..];
                    }
                }
                NodeChildren::Sparse { bitset, values } => {
                    let bitset_len = mem::size_of::<Bitset>();
                    unsafe {
                        children_buf[..bitset_len]
                            .as_mut_ptr()
                            .cast::<Bitset>()
                            .write(bitset);
                    };
                    children_buf = &mut children_buf[bitset_len..];
                    for v in values {
                        unsafe {
                            children_buf.as_mut_ptr().cast::<PackedNode<T>>().write(v);
                        }
                        children_buf = &mut children_buf[mem::size_of::<PackedNode<T>>()..];
                    }
                }
                NodeChildren::Dense { table } => {
                    let table_len = mem::size_of::<[PackedNode<T>; 256]>();
                    unsafe {
                        children_buf[..table_len]
                            .as_mut_ptr()
                            .cast::<[PackedNode<T>; 256]>()
                            .write(table);
                    }
                    children_buf = &mut children_buf[table_len..];
                }
            }
        }

        if let Some(value) = value {
            let value_slice = &mut buf[header.value_range().unwrap()];
            let value_ptr = value_slice.as_mut_ptr();
            unsafe { value_ptr.cast::<T>().write(value) };
        }
    }

    fn unpack(header: NodeHeader<T>, buf: &[u8]) -> Self {
        let prefix = buf[header.prefix_range()].to_owned();

        let children_buf = &buf[header.children_range()];
        let children = match header.children_type() {
            NodeChildrenType::Empty => NodeChildren::Empty,
            NodeChildrenType::Pairs => {
                let keys = children_buf[0..header.num_children()].to_owned();
                let mut values = Vec::with_capacity(keys.len());
                let ptr_size = mem::size_of::<PackedNode<T>>();
                for vs in children_buf[header.num_children()..].chunks(ptr_size) {
                    let p = unsafe { vs.as_ptr().cast::<PackedNode<T>>().read() };
                    values.push(p);
                }
                assert_eq!(keys.len(), values.len());
                NodeChildren::Pairs { keys, values }
            }
            NodeChildrenType::Sparse => {
                let bitset_len = mem::size_of::<Bitset>();
                let bitset = unsafe { children_buf[..bitset_len].as_ptr().cast::<Bitset>().read() };
                let mut values = Vec::with_capacity(header.num_children());
                let ptr_size = mem::size_of::<PackedNode<T>>();
                for vs in children_buf[bitset_len..].chunks(ptr_size) {
                    let p = unsafe { vs.as_ptr().cast::<PackedNode<T>>().read() };
                    values.push(p);
                }
                assert_eq!(values.len(), header.num_children());
                NodeChildren::Sparse { bitset, values }
            }
            NodeChildrenType::Dense => {
                let table_len = mem::size_of::<[PackedNode<T>; 256]>();
                let table = unsafe {
                    children_buf[..table_len]
                        .as_ptr()
                        .cast::<[PackedNode<T>; 256]>()
                        .read()
                };
                NodeChildren::Dense { table }
            }
        };

        let value = header.value_range().map(|range| {
            let value_buf = &buf[range];
            unsafe { value_buf.as_ptr().cast::<T>().read() }
        });

        Self {
            prefix,
            children,
            value,
        }
    }
}

pub enum NodeChildren<T> {
    Empty,
    Pairs {
        keys: Vec<u8>,
        values: Vec<PackedNode<T>>,
    },
    Sparse {
        bitset: Bitset,
        values: Vec<PackedNode<T>>,
    },
    Dense {
        table: [PackedNode<T>; 256],
    },
}

impl<T> NodeChildren<T> {
    pub fn one(k: u8, ptr: PackedNode<T>) -> Self {
        NodeChildren::Pairs {
            keys: vec![k],
            values: vec![ptr],
        }
    }

    pub fn two(k1: u8, ptr1: PackedNode<T>, k2: u8, ptr2: PackedNode<T>) -> Self {
        NodeChildren::Pairs {
            keys: vec![k1, k2],
            values: vec![ptr1, ptr2],
        }
    }

    fn structure_type(&self) -> NodeChildrenType {
        match self {
            NodeChildren::Empty => NodeChildrenType::Empty,
            NodeChildren::Pairs { .. } => NodeChildrenType::Pairs,
            NodeChildren::Sparse { .. } => NodeChildrenType::Sparse,
            NodeChildren::Dense { .. } => NodeChildrenType::Dense,
        }
    }

    pub fn len(&self) -> usize {
        match self {
            NodeChildren::Empty => 0,
            NodeChildren::Pairs { keys, .. } => keys.len(),
            NodeChildren::Sparse { values, .. } => values.len(),
            NodeChildren::Dense { table } => table.iter().filter(|v| !v.is_empty()).count(),
        }
    }

    pub fn into_pairs(self) -> BTreeMap<u8, PackedNode<T>> {
        let mut out = BTreeMap::new();
        match self {
            NodeChildren::Empty => (),
            NodeChildren::Pairs { keys, values } => {
                for (k, v) in keys.into_iter().zip(values.into_iter()) {
                    if !v.is_empty() {
                        assert!(out.insert(k, v).is_none());
                    }
                }
            }
            NodeChildren::Sparse { bitset, values } => {
                for (k, v) in bitset.iter().zip(values.into_iter()) {
                    if !v.is_empty() {
                        assert!(out.insert(k as u8, v).is_none());
                    }
                }
            }
            NodeChildren::Dense { mut table } => {
                for (k, v) in table.iter_mut().enumerate() {
                    let v = mem::replace(v, PackedNode::empty());
                    if !v.is_empty() {
                        assert!(out.insert(k as u8, v).is_none());
                    }
                }
            }
        }
        out
    }

    pub fn from_pairs(pairs: BTreeMap<u8, PackedNode<T>>) -> Self {
        match pairs.len() {
            0 => NodeChildren::Empty,
            1..=32 => {
                let mut keys = vec![];
                let mut values = vec![];
                for (k, v) in pairs {
                    keys.push(k);
                    values.push(v);
                }
                NodeChildren::Pairs { keys, values }
            }
            33..=192 => {
                let mut bitset = Bitset::new();
                let mut values = vec![];
                for (i, node) in pairs {
                    bitset.set(i);
                    values.push(node);
                }
                NodeChildren::Sparse { bitset, values }
            }
            192..=256 => {
                let mut table: [PackedNode<T>; 256] = unsafe { mem::zeroed() };
                for i in 0..256 {
                    table[i] = PackedNode::empty();
                }
                for (i, node) in pairs {
                    table[i as usize] = node;
                }
                NodeChildren::Dense { table }
            }
            n => panic!("Invalid length: {}", n),
        }
    }
}
