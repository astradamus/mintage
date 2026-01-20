use rand::Rng;

pub const NEIGHBORS_8: [(isize, isize); 8] = [
    (-1, -1), (0, -1), (1, -1),
    (-1,  0),          (1,  0),
    (-1,  1), (0,  1), (1,  1),
];

pub const NEIGHBORS_4: [(isize, isize); 4] = [
              (0, -1),
    (-1,  0),          (1,  0),
              (0,  1),
];

/// Iterate over all neighbors in a random order, returning true if a match is found.
pub fn try_random_dirs<F, R>(rng: &mut R, use_4: bool, mut try_dir: F) -> bool
where
    F: FnMut((isize, isize)) -> bool,
    R: Rng,
{
    let mut rem = [0, 1, 2, 3, 4, 5, 6, 7];
    let mut len = if (use_4) { 4 } else { 8 };

    while len > 0 {
        let r = rng.random_range(0..len);
        let i = rem[r];

        len -= 1;
        rem.swap(r, len);

        let n = if (use_4) { NEIGHBORS_4[i] } else { NEIGHBORS_8[i] };
        if try_dir(n) {
            return true;
        }
    }

    false
}

/// Iterate over all cells in a random direction, firing the given function
/// for each. This doesn't cost much--turning it off only gains ~14% TPS--and
/// it is basically mandatory to prevent bias/artifacts.
pub fn rand_iter_dir<F, R>(rng : &mut R, w: usize, h: usize, mut iter_fn:F)
where
    F: FnMut(usize, usize),
    R: Rng,
{
    let r = rng.random_range(0..4) as usize;

    // Do loops in different directions to prevent bias, chosen randomly each frame.
    if (r == 0) {
        for y in 0..h {
            for x in 0..w {
                iter_fn(x, y);
            }
        }
    }
    else if (r == 1) {
        for y in (0..h).rev() {
            for x in (0..w) {
                iter_fn(x, y);
            }
        }
    }
    else if (r == 2) {
        for y in (0..h).rev() {
            for x in (0..w).rev() {
                iter_fn(x, y);
            }
        }
    }
    else if (r == 3) {
        for y in (0..h) {
            for x in (0..w).rev() {
                iter_fn(x, y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_xoshiro::Xoshiro256PlusPlus;

    #[test]
    fn test_try_random_dirs_completeness() {
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(89);
        let mut history = Vec::new();

        // Try 4 directions and record them
        try_random_dirs(&mut rng, true, |dir| {
            history.push(dir);
            false // return false so it keeps going
        });

        // Ensure we have visited exactly 4 directions.
        assert_eq!(history.len(), 4);

        // If each direction was visited, and we only visited 4, duplicates are impossible.
        for dir in NEIGHBORS_4 {
            assert!(history.contains(&dir));
    }

        // Clear history for 8 directions test
        history.clear();

        // Try 8 directions and record them
        try_random_dirs(&mut rng, false, |dir| {
            history.push(dir);
            false // return false so it keeps going
        });

        // Ensure we have visited exactly 8 directions.
        assert_eq!(history.len(), 8);

        // If each direction was visited, and we only visited 8, duplicates are impossible.
        for dir in NEIGHBORS_8 {
            assert!(history.contains(&dir));
        }
    }
}
