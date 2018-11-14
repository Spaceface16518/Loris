extern crate rand;

use rand::distributions::Standard;
use rand::Rng;
use std::iter::Iterator;

struct Generator<'a, R: Rng> {
    chunk_size: usize,
    rng: &'a mut R,
}

impl<'a, R: Rng> Iterator for Generator<'a, R> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(
            self.rng
                .sample_iter(&Standard)
                .take(self.chunk_size)
                .collect::<Vec<u8>>()
        )
    }
}
