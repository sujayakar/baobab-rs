use crate::Trie;

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::ops::Neg;
use std::panic;
use rand::{Rng, SeedableRng};
use rand::distributions::Exp;
use rand::rngs::StdRng;
use rand::seq::{IteratorRandom, SliceRandom};

#[derive(Clone, Copy)]
enum Action {
    Insert,
    Overwrite,
    QueryExisting,
    QueryNonexistent,
    Iter,
    RemoveExisting,
    RemoveNonexistent,
}

struct Simulation<R: Rng> {
    model: BTreeMap<Vec<u8>, ()>,
    trie: Trie<()>,

    rng: R,
}

impl<R: Rng> Simulation<R> {
    fn new(rng: R) -> Self {
        Self {
            model: BTreeMap::new(),
            trie: Trie::new(),
            rng
        }
    }
    fn sample(&mut self) -> Action {
        use Action::*;

        // Let the probability of inserting a new key be e^{-keys.len()}
        let pr_insertion = (self.model.len() as f64).neg().exp();
        if self.rng.gen::<f64>() < pr_insertion || self.model.is_empty() {
            Insert
        } else {
            let choices = &[
                Overwrite,
                QueryExisting,
                QueryNonexistent,
                Iter,
                RemoveExisting,
                RemoveNonexistent,
            ];
            *choices.choose(&mut self.rng).unwrap()
        }
    }

    fn step(&mut self) {
        use Action::*;
        let r = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            // eprintln!("Trie:");
            // self.trie.debug(&mut io::stderr().lock()).unwrap();
            match self.sample() {
                Insert => {
                    let key = self.nonexistent_key();
                    // eprintln!("Inserting key {:?}", key);
                    assert!(self.model.insert(key.clone(), ()).is_none());
                    assert!(self.trie.insert(&key, ()).is_none());
                },
                Overwrite => {
                    let key = self.sample_key();
                    // eprintln!("Overwriting existing key {:?}", key);
                    assert!(self.model.insert(key.clone(), ()).is_some());
                    assert!(self.trie.insert(&key, ()).is_some());
                },
                QueryExisting => {
                    let key = self.sample_key();
                    // eprintln!("Querying existing key {:?}", key);
                    assert!(self.model.get(&key).is_some());
                    assert!(self.trie.get(&key).is_some());
                },
                QueryNonexistent => {
                    let key = self.nonexistent_key();
                    // eprintln!("Querying nonexistent key {:?}", key);
                    assert!(self.model.get(&key).is_none());
                    assert!(self.trie.get(&key).is_none());
                },
                Iter => {
                    assert!(self.trie.iter().map(|(k, _)| k).eq(self.model.keys().cloned()));
                },
                RemoveExisting => {
                    let key = self.sample_key();
                    // eprintln!("Removing existing key {:?}", key);
                    assert!(self.model.remove(&key).is_some());
                    assert!(self.trie.remove(&key).is_some());
                },
                RemoveNonexistent => {
                    let key = self.nonexistent_key();
                    // eprintln!("Removing nonexistent key {:?}", key);
                    assert!(self.model.remove(&key).is_none());
                    assert!(self.trie.remove(&key).is_none());
                },
            }
        }));
        if let Err(e) = r {
            // eprintln!("Trie:");
            self.trie.debug(&mut io::stderr().lock()).unwrap();
            // eprintln!("Model: {:?}", self.model);
            panic!("{:?}", e);
        }
    }

    fn sample_key(&mut self) -> Vec<u8> {
        self.model.keys().choose(&mut self.rng).unwrap().clone()
    }

    fn nonexistent_key(&mut self) -> Vec<u8> {
        loop {
            // Expected key length is ~4.
            let key_length = self.rng.sample(Exp::new(0.25)) as usize;
            let mut key = vec![0; key_length];
            self.rng.fill(&mut key[..]);

            if self.model.contains_key(&key) {
                // eprintln!("Skipping duplicate key {:?}, retrying...", key);
                continue;
            }
            return key;
        }
    }
}

#[test]
fn test_simulation() {
    for i in 0.. {
        let seed = rand::thread_rng().gen();
        // let seed = [139, 152, 242, 158, 126, 69, 216, 91, 103, 148, 39, 223, 90, 83, 109, 211, 66, 208, 166, 48, 130, 40, 34, 228, 13, 186, 56, 4, 249, 246, 124, 53];

        if i % 100 == 0 {
            eprintln!("Using seed {:?}", seed);
        }
        let mut s = Simulation::new(StdRng::from_seed(seed));
        for i in 0..100 {
            s.step();
        }
    }
}


#[quickcheck]
fn qc_insert_utf8(keys: Vec<char>) -> bool {
    let mut t = Trie::new();
    let mut ks = vec![];
    for k in keys {
        let mut v = vec![0; 4];
        let l = k.encode_utf8(&mut v[..]).len();
        v.truncate(l);
        t.insert(&v[..], ());
        ks.push(v);
    }
    ks.into_iter().all(|k| t.get(&k[..]).is_some())
}

#[quickcheck]
fn qc_iter_utf8(keys: Vec<char>) -> bool {
    let mut t = Trie::new();
    let mut s = BTreeSet::new();
    for k in keys {
        let mut v = vec![0; 4];
        let l = k.encode_utf8(&mut v[..]).len();
        v.truncate(l);
        t.insert(&v[..], ());
        s.insert(v);
    }

    t.iter().map(|(k, _)| k).collect::<BTreeSet<_>>() == s
}
