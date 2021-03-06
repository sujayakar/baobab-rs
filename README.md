# baobab

Baobabs are deciduous trees found in Africa, Arabia, Australia, and
Madagascar. [*Adansonia grandidieri*](https://en.wikipedia.org/wiki/Adansonia_grandidieri),
the largest of the baobabs, has a long, thick trunk devoid of branches
followed by a wide, flat-topped crown.

`baobab`, on the other hand, is an implementation of an 
[Adaptive Radix Tree](https://db.in.tum.de/~leis/papers/ART.pdf) (ART) in
Rust.  It's designed for memory efficiency and extremely fast queries.  If
you need range queries and your keys can be represented as byte strings,
`baobab` will likely be much faster and more compact than `BTreeMap`.  If your
keys share many prefixes, `baobab` will deduplicate these prefixes and likely
use less memory than `HashMap`.

One of the key optimizations from the ART paper is *path compression*, which
allows a node to contain a "prefix" that precedes its branches.  Having a long
prefix makes a node look like an upsided down baobab tree.

## When are tries applicable?
...


## Benchmarks
...


## Implementation
In addition to the ART paper, I drew heavy inspiration from Redis's [`rax`
library](https://github.com/antirez/rax).

Some other implmentations of similar ideas in Rust are...
