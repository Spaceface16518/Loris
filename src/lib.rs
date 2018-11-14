extern crate rand;
extern crate smallvec;

use rand::{distributions::Standard, Rng};
use smallvec::SmallVec;
use std::{
    cmp::min,
    io::{self, BufRead, Read},
    iter::Iterator,
};

const BUF_SIZE: usize = 512;

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

impl<'a, R: Rng> Read for Generator<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let cap = min(buf.len(), self.chunk_size);
        // COMBAK: take, enumerate and insert each instead?
        for i in 0..cap {
            match self.next() {
                Some(x) => buf[i] = x[i],
                _ => (), /* Technically redundant since `Generator` is an
                          * infinite iterator */
            }
        }
        Ok(cap)
    }
}

struct BufferedGenerator<'a, R: Rng> {
    inner: Generator<'a, R>,
    buf: SmallVec<[u8; BUF_SIZE]>,
    pos: usize,
    cap: usize,
}

impl<'a, R: Rng> BufferedGenerator<'a, R> {
    pub fn from_generator(inner: Generator<'a, R>) -> BufferedGenerator<'a, R> {
        BufferedGenerator {
            inner,
            buf: SmallVec::<[u8; BUF_SIZE]>::new(),
            pos: 0,
            cap: 0,
        }
    }

    pub fn new(chunk_size: usize, rng: &'a mut R) -> BufferedGenerator<'a, R> {
        BufferedGenerator::from_generator(Generator::new(chunk_size, rng))
    }
}

impl<'a, R: Rng> BufRead for BufferedGenerator<'a, R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.cap {
            debug_assert_eq!(self.pos, self.cap);
            self.cap = self.inner.read(&mut self.buf).unwrap();
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = min(self.pos + amt, self.cap)
    }
}

impl<'a, R: Rng> Read for BufferedGenerator<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
