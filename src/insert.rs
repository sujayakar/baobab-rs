use crate::node::{Node, NodeChildren};
use crate::packed_node::PackedNode;

impl<T> PackedNode<T> {
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

        let new_child = Node::new(child_prefix.to_owned(), old_children, old_value);
        let new_parent = Node::new(
            parent_prefix.to_owned(),
            NodeChildren::one(branch, PackedNode::new(new_child)),
            Some(new_value),
        );
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

        let first_child = Node::new(
            first_prefix.to_owned(),
            old_children,
            old_value,
        );
        let second_child = Node::new(
            second_prefix.to_owned(),
            NodeChildren::Empty,
            Some(new_value),
        );
        let new_parent = Node::new(
            parent_prefix.to_owned(),
            NodeChildren::two(
                first_branch,
                PackedNode::new(first_child),
                second_branch,
                PackedNode::new(second_child),
            ),
            None,
        );
        *self = PackedNode::new(new_parent);
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
                let new_child = Node::new(
                    key_iter.as_slice().to_owned(),
                    NodeChildren::Empty,
                    Some(value),
                );
                self.add_child(branch_byte, new_child);
                None
            }
            Some(next_node) => next_node.insert(key_iter.as_slice(), value),
        }
    }
}
