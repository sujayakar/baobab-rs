// There are two invariants about our tree we'd like to maintain inductively.
// 1. Allocated nodes have either children or a value.
// 2. No node without a value has only a single child.  These nodes must be
//    merged into their child as prefix.
//
// Say we're deleting a key `k` that's present in our trie.  We traverse the
// trie, advancing through bytes in `k` to find the node with the value.  The
// most naive deletion algorithm just removes this value from the node and
// returns.
//
// This does not strictly put the trie into an invalid state, but it can newly
// break both invariants 1 and 2.  First, if our node has no children, we must
// deallocate the node and remove it from its parent to preserve invariant 1.
// Second, if our node has just one child, removing the value breaks invariant
// 2, and we must merge into the child.
//
// Now, removing ourselves from our parent to preserve the first invariant may
// now break either of these invariants for our parent if it doesn't have a
// value itself.  Therefore, we must continue up the parent chain, inductively
// patching up our invariants.

use crate::node::{Node, NodeChildren};
use crate::packed_node::PackedNode;

impl<T> PackedNode<T> {
    pub fn remove(&mut self, key: &[u8]) -> Option<T> {
        let mut key_iter = key.iter();

        for &byte in self.prefix() {
            match key_iter.next() {
                Some(&key_byte) if key_byte == byte => continue,
                _ => return None,
            }
        }
        let branch_byte = match key_iter.next() {
            None => {
                if !self.has_value() {
                    return None;
                }
                let Node { mut prefix, children, value } = self.take();
                let value = value.unwrap();
                let pairs = children.into_pairs();
                match pairs.len() {
                    0 => {
                        // Leave `self` as an empty node and let our parent handle unlinking us.
                        return Some(value);
                    },
                    1 => {
                        let (child_byte, mut packed_child) = pairs.into_iter().next().unwrap();
                        let child = packed_child.take();

                        prefix.push(child_byte);
                        prefix.extend_from_slice(child.prefix());

                        let new_node = Node::new(prefix, child.children, child.value);
                        *self = PackedNode::new(new_node);
                        return Some(value);
                    },
                    _ => {
                        let children = NodeChildren::from_pairs(pairs);
                        *self = PackedNode::new(Node::new(prefix, children, None));
                        return Some(value);
                    },
                }
            },
            Some(&k) => k,
        };
        let next_node = self.lookup_mut(branch_byte)?;
        let removed_value = next_node.remove(key_iter.as_slice())?;

        if !next_node.is_empty() {
            return Some(removed_value);
        }

        let Node { mut prefix, children, value } = self.take();
        let pairs = children.into_pairs();
        match (value.is_some(), pairs.len()) {
            (false, 0) => {
                // Leave ourselves as empty to let the parent cleanup.
                return Some(removed_value);
            },
            (false, 1) => {
                let (child_byte, mut packed_child) = pairs.into_iter().next().unwrap();
                let child = packed_child.take();

                prefix.push(child_byte);
                prefix.extend_from_slice(child.prefix());

                let new_node = Node::new(prefix, child.children, child.value);
                *self = PackedNode::new(new_node);
                return Some(removed_value);
            },
            // If we have a value, we can't deallocate ourselves or merge ourselves into a child.
            (true, _) | (false, _) => {
                assert!(!pairs.contains_key(&branch_byte));
                let children = NodeChildren::from_pairs(pairs);
                let new_node = Node::new(prefix, children, value);
                *self = PackedNode::new(new_node);
                return Some(removed_value);
            }
        }
    }
}
