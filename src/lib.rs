extern crate rand;

use rand::{distributions::Standard, Rng};
use std::{
    cmp::min,
    io::{self, Read},
    iter::Iterator,
};

/// A generator made specifically for the `SlowLoris`.
struct Generator<'a, R> {
    chunk_size: usize,
    rng: &'a mut R,
}

impl<'a, R> Generator<'a, R> {
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

impl<'a, R: Rng> Read for Generator<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Only go up to the chunk size, or to the limit of the buffer
        let cap = min(buf.len(), self.chunk_size);

        // COMBAK: take, enumerate and insert each instead?
        match self.next() {
            Some(v) => {
                for i in 0..cap {
                    buf[i] = v[i]
                }
            },
            _ => (),
        }
        Ok(cap)
    }
}
