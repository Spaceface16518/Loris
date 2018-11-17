extern crate rand;

use generator::rand::{distributions::Standard, Rng};
use std::{
    cmp::min,
    io::{self, Read},
    iter::FromIterator,
    marker::PhantomData,
    ops::Index,
};

struct Generator<'a, R, I> {
    chunk_size: usize,
    rng: &'a mut R,
    _marker: PhantomData<I>,
}

impl<'a, R, I> Generator<'a, R, I> {
    pub fn new(chunk_size: usize, rng: &'a mut R) -> Generator<'a, R, I> {
        Generator {
            chunk_size,
            rng,
            _marker: PhantomData,
        }
    }
}

impl<'a, R: Rng, I: FromIterator<u8>> Iterator for Generator<'a, R, I> {
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        Some(
            self.rng
                .sample_iter(&Standard)
                .take(self.chunk_size)
                .collect::<I>(),
        )
    }
}

impl<'a, R: Rng, I: FromIterator<u8> + Index<usize, Output = u8>> Read
    for Generator<'a, R, I>
{
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
