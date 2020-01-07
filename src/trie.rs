use crate::packed_node::PackedNode;
use std::io;

pub struct Trie<T> {
    pub(crate) root: PackedNode<T>,
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

    pub fn remove(&mut self, key: &[u8]) -> Option<T> {
        self.root.remove(key)
    }

    pub fn debug(&self, out: &mut impl io::Write) -> io::Result<()> {
        self.root.debug("", out)
    }
}

#[cfg(test)]
mod tests {
    use super::Trie;
    use std::io;


    #[test]
    fn test_insert() {
        let mut t = Trie::new();
        t.insert(&[1, 2, 3], ());
        t.insert(&[1, 2, 4], ());
        t.insert(&[1, 2, 3, 5], ());
        t.insert(&[1, 2], ());

        for (k, v) in t.iter() {
            eprintln!("{:?} -> {:?}", k, v);
        }

        let n = 35;
        for i in 2..n {
            t.insert(&[i], ());
        }

        assert!(t.get(&[1, 2, 3]).is_some());
        assert!(t.get(&[1, 2, 4]).is_some());
        assert!(t.get(&[1, 2, 5]).is_none());
        assert!(t.get(&[1, 2, 3, 4]).is_none());
        assert!(t.get(&[1, 2, 3, 5]).is_some());
        assert!(t.get(&[1, 2]).is_some());
        assert!(t.get(&[]).is_none());

        for i in 2..n {
            assert!(t.get(&[i]).is_some());
        }
        assert!(t.get(&[n + 1]).is_none());

        eprintln!("root {:?}", t.debug(&mut io::stdout().lock()));
    }
}
