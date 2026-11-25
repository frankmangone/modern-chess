//! Chance, as a function: the `index`th random number of a match is `draw(seed, index)`.
//!
//! There is no generator to keep in step. The match keeps its seed and how many numbers it has
//! used, so the ways of a turn (worked out before the player answers) and the way that is played
//! agree, and a saved match resumes on the very next number.

/// The most dice one `ROLL` throws. A roll of more makes no way, as a walk too long to follow does.
pub const MAX_DICE: u32 = 10_000;

/// SplitMix64's step: the gap between the inputs of two consecutive numbers.
const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// The `index`th number of the sequence of `seed`: SplitMix64's finalizer over the seed moved
/// along by `index + 1` steps.
pub fn draw(seed: u64, index: u64) -> u64 {
    let mut z = seed.wrapping_add(index.wrapping_add(1).wrapping_mul(GAMMA));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The number in `0..bound` that the draw `x` stands for; 0 when `bound` is 0. It is the high half
/// of `x * bound`, whose bias is below `bound / 2^64`: far under anything a game can notice.
pub fn below(x: u64, bound: u32) -> u32 {
    // The high 64 bits of a 128-bit product of a u64 and a u32 are below `bound`.
    u32::try_from((u128::from(x) * u128::from(bound)) >> 64).unwrap_or(0)
}

/// The seed a restarted game uses after `seed`: another sequence, still a function of the first.
pub fn next_seed(seed: u64) -> u64 {
    draw(seed, u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_and_index_give_the_same_number() {
        for seed in [0, 1, 42, u64::MAX] {
            for index in 0..20 {
                assert_eq!(draw(seed, index), draw(seed, index));
            }
        }
    }

    #[test]
    fn seeds_and_indexes_give_different_numbers() {
        let a: Vec<u64> = (0..50).map(|index| draw(1, index)).collect();
        let b: Vec<u64> = (0..50).map(|index| draw(2, index)).collect();
        assert_ne!(a, b);
        let mut sorted = a.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), a.len(), "no repeats in the first fifty");
    }

    #[test]
    fn the_sequence_of_seed_zero_is_fixed() {
        // The published SplitMix64 outputs for seed 0 (the first is draw 0). A saved match
        // depends on these never changing.
        assert_eq!(draw(0, 0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(draw(0, 1), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(draw(0, 2), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        for bound in [1u32, 2, 6, 52, 1000] {
            let mut seen = vec![false; bound as usize];
            for index in 0..20_000 {
                let value = below(draw(7, index), bound);
                assert!(value < bound);
                if let Some(slot) = seen.get_mut(value as usize) {
                    *slot = true;
                }
            }
            assert!(seen.iter().all(|&hit| hit), "every value of 0..{bound} came up");
        }
        assert_eq!(below(u64::MAX, 0), 0);
    }

    #[test]
    fn a_die_is_close_to_fair() {
        let mut counts = [0u32; 6];
        let rolls = 60_000u64;
        for index in 0..rolls {
            if let Some(slot) = counts.get_mut(below(draw(99, index), 6) as usize) {
                *slot += 1;
            }
        }
        for count in counts {
            // 10 000 expected; 5 standard deviations of the count is about 460.
            assert!((9_500..=10_500).contains(&count), "{counts:?}");
        }
    }

    #[test]
    fn the_next_seed_differs_and_is_fixed() {
        assert_ne!(next_seed(5), 5);
        assert_eq!(next_seed(5), next_seed(5));
        assert_ne!(next_seed(5), next_seed(6));
    }
}
