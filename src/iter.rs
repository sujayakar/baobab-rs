use crate::packed_node::PackedNode;
use crate::trie::Trie;

#[derive(Clone, Copy)]
enum State {
    Start,
    Recurse(Option<u8>),
    PopByte(Option<u8>),
}

struct TreeIterator<'a, T> {
    key: Vec<u8>,
    stack: Vec<(&'a PackedNode<T>, State)>,
}

impl<'a, T> Iterator for TreeIterator<'a, T> {
    type Item = (Vec<u8>, &'a T);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (node, state) = self.stack.last_mut()?;
            match *state {
                State::Start => {
                    *state = State::Recurse(Some(0));

                    for &byte in node.prefix() {
                        self.key.push(byte);
                    }
                    if let Some(val) = node.value() {
                        return Some((self.key.clone(), val));
                    }
                }
                State::Recurse(Some(i)) => {
                    let next_ix = i.checked_add(1);
                    if let Some(child) = node.lookup(i) {
                        *state = State::PopByte(next_ix);
                        self.key.push(i);
                        self.stack.push((child, State::Start));
                    } else {
                        *state = State::Recurse(next_ix);
                    }
                }
                State::PopByte(next_ix) => {
                    self.key.pop();
                    *state = State::Recurse(next_ix);
                }
                State::Recurse(None) => {
                    self.key.truncate(self.key.len() - node.prefix().len());
                    self.stack.pop();
                }
            }
        }
    }
}

impl<T> Trie<T> {
    pub fn iter(&self) -> impl Iterator<Item = (Vec<u8>, &T)> {
        TreeIterator {
            key: vec![],
            stack: vec![(&self.root, State::Start)],
        }
    }
}
