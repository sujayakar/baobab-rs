// TODO:
// # Algorithm
// [ ] Add values optimization
// [ ] Make removals patch up the tree if needed.
// [ ] Add SIMD support
// [ ] Add in place mutations
// [ ] Unrolled loop for up to four pairs
//
// # Cleanup
// [ ] Add prefix len bound
// [ ] Dedup code to determine child variant
// [ ] Split up into nice modules
// [ ] Use a macro to get rid of the unsafety in lookup
// [ ] Remove the into/from pairs stuff
// [ ] Trie debug skips test capture
//
// # Performance
// [X] Pack header tighter
// [ ] Can we avoid cloning the key in the iterator?
// [ ] Add SIMD prefix comparison + length short circuit
//
// # API
// [ ] Add iter_mut
// [ ] Add range iteration
// [ ] Add into_iter
// [ ] Add .keys() and .values()
// [ ] Add random sampling
// [ ] Min/max APIs
// [ ] Entry API
// [ ] Clear API
// [ ] Merge two tries?
// [ ] Split a trie?
// [ ] Node annotation?
// [ ] Implement clone
//
// # Testing
// [ ] Add memory report (w/external fragmentation?)
// [ ] Add benchmarks with representative data, compare to other structures
// [ ] Fuzz testing
// [ ] Add invariant checks (re: prefix optimization, child lengths, value optimization...)
// [ ] Better unit tests lol
// [ ] Add microbenchmarking suite
// [ ] Seems like we're probably memory bound for latency anyways :(
//
// # Docs
// [ ] Principles behind this library: memory-bound, unaligned loads fast on x86, SIMD is free,
//     SparseTable is *great* w/appropriate tuning.
// [ ] Naming: baobab?
//
// # Packable
// [ ] Better handle panics within user code
// [ ] Add dealloc in place perhaps?
// [ ] DSL for specifying packed structures?  See packed2.rs
//
// [ ] License under apache or mit at convenience
// [ ] contributions under apache
#![feature(test)]

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod bitset;
mod header;
mod iter;
mod insert;
mod node;
mod packable;
mod packed_node;
mod remove;
mod trie;

#[cfg(test)]
mod qc_tests;

pub use trie::Trie;
