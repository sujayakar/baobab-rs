/// This specialized bitset stores exactly 256 bits, all defaulting to zero.
#[repr(packed)]
pub struct Bitset {
    bits: [u64; 4],
}

impl Bitset {
    /// Construct a new `Bitset`.
    pub fn new() -> Self {
        Self { bits: [0; 4] }
    }

    /// Set the `byte`th bit (zero-indexed) to one.
    pub fn set(&mut self, byte: u8) {
        self.bits[byte as usize / 64] |= 1 << (byte % 64);
    }

    /// If the `byte`th bit is set, return the number of bits set to the left of `byte`.
    pub fn query(&self, byte: u8) -> Option<usize> {
        if self.bits[byte as usize / 64] & (1 << (byte % 64)) == 0 {
            return None;
        }
        let mut rank = 0;
        for i in 0..4 {
            let m = if i < byte / 64 { 1 } else { 0 };
            rank += self.bits[i as usize].count_ones() as usize * m;
        }
        let mask = (1u64 << (byte % 64)) - 1;
        let block_rank = (self.bits[byte as usize / 64] & mask).count_ones() as usize;
        Some(rank + block_rank)
    }

    /// Iterate over the set bits in the bitset.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=u8> + 'a {
        (0..4)
            .flat_map(|i| (0..64).map(move |j| (i, j)))
            .filter(move |&(i, j)| self.bits[i] & (1 << j) != 0)
            .map(|(i, j)| i as u8 * 64 + j as u8)
    }
}
