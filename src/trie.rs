use crate::packed_node::PackedNode;

pub struct Trie<T> {
    root: PackedNode<T>,
}

impl<T> Trie<T> {
    pub fn new() -> Self {
        Self {
            root: PackedNode::empty(),
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&T> {
        let mut cur = &self.root;
        let mut key_iter = key.iter();
        loop {
            for byte in cur.prefix() {
                match key_iter.next() {
                    Some(key_byte) if key_byte != byte => return None,
                    None => return None,
                    Some(..) => continue,
                }
            }
            let branch_byte = match key_iter.next() {
                None => return cur.value(),
                Some(&k) => k,
            };
            cur = cur.lookup(branch_byte)?;
        }
    }

    pub fn get_mut(&self, key: &[u8]) -> Option<&mut T> {
        self.get(key)
            .map(|p| unsafe { &mut *(p as *const _ as *mut _) })
    }

    pub fn insert(&mut self, key: &[u8], value: T) -> Option<T> {
        self.root.insert(key, value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Vec<u8>, &T)> {
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
        TreeIterator {
            key: vec![],
            stack: vec![(&self.root, State::Start)],
        }
    }

    pub fn remove(&mut self, key: &[u8]) -> Option<T> {
        self.root.remove(key)
    }
}
