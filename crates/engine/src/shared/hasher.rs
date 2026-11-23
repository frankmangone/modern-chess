//! A fast hash for maps keyed by interned names.
//!
//! A `Position` or `Symbol` hashes as the address of its text, one machine word, so the default
//! hasher's work (built for text from untrusted sources) is wasted on it. This one multiplies the
//! word by a large odd number and folds the high half over the low, so both the bits a table
//! picks its slot from and the bits it keeps for a quick check are mixed. The order a map
//! iterates in is never part of the contract.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Clone, Copy, Default)]
pub struct IdHasher {
    hash: u64,
}

impl IdHasher {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for IdHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            for (slot, byte) in word.iter_mut().zip(chunk) {
                *slot = *byte;
            }
            self.add(u64::from_le_bytes(word));
        }
    }

    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }

    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash ^ (self.hash >> 32)
    }
}

/// A map keyed by interned names, hashed by `IdHasher`.
pub type IdMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;

/// A set of interned names, hashed by `IdHasher`.
pub type IdSet<K> = HashSet<K, BuildHasherDefault<IdHasher>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{Position, Symbol};

    #[test]
    fn maps_find_names_by_text() {
        let mut map: IdMap<Position, u32> = IdMap::default();
        for x in 0..40u8 {
            for y in 0..40u8 {
                map.insert(
                    Position::from_coordinates(&[x, y]),
                    u32::from(x) * 100 + u32::from(y),
                );
            }
        }
        assert_eq!(map.len(), 1600);
        assert_eq!(map.get(&Position::new("7,9")), Some(&709));
        let mut set: IdSet<Symbol> = IdSet::default();
        set.insert(Symbol::new("KING"));
        assert!(set.contains(&Symbol::new("KING")));
        assert!(!set.contains(&Symbol::new("QUEEN")));
    }
}
