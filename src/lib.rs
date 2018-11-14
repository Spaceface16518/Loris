extern crate rand;

use rand::{distributions::Standard, thread_rng, Rng};
use std::{io::Write, iter::Iterator};

/// A generator made specifically for the `SlowLoris`.
struct Generator<'a, R: Rng> {
    chunk_size: usize,
    rng: &'a mut R,
}

impl<'a, R: Rng> Generator<'a, R> {
    pub fn new(chunk_size: usize, rng: &'a mut R) -> Generator<'a, R> {
        Generator { chunk_size, rng }
    }
}

impl<'a, R: Rng> Iterator for Generator<'a, R> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(
            self.rng
                .sample_iter(&Standard)
                .take(self.chunk_size)
                .collect::<Vec<u8>>(),
        )
    }
}
