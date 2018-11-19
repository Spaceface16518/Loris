extern crate smallvec;

use self::smallvec::SmallVec;
use super::thread::ThreadPool;
use std::{
    collections::VecDeque,
    io::{self, Write},
    marker::PhantomData,
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

const BUF_SIZE: usize = 64;
const CHUNK_SIZE: usize = 4; // in bytes

pub struct SocketWriter<S, F> {
    pool: SocketPool<S, F>,
    buf: SmallVec<[u8; BUF_SIZE]>,
}

impl<S, F> Write for SocketWriter<S, F>
where
    S: ToSocketAddrs + Send + Clone + 'static,
    F: FnOnce() + 'static,
{
    #[inline] // Should this be inlined?
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.reserve(buf.len());
        let mut byte_count = 0;
        for &byte in buf.iter() {
            self.buf.push(byte);
            byte_count += 1;
        }
        if self.buf.len() > BUF_SIZE {
            self.flush().expect("Could not flush buffer");
        }
        Ok(byte_count)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        for i in self.buf.windows(CHUNK_SIZE) {
            // This is expensive; needs to be optimized
            self.pool.enqueue(i.to_vec().into_boxed_slice());
        }
        self.buf.clear();
        Ok(())
    }
}

pub struct SocketPool<S, F> {
    connections: Vec<Connection<S, F>>,
    queue: Arc<Mutex<Queue<Message<Box<[u8]>>>>>,
}

impl<S, F> SocketPool<S, F>
where
    S: ToSocketAddrs + Send + 'static + Clone,
    F: FnOnce() + 'static,
{
    pub fn new(size: usize, ip: S) -> SocketPool<S, F> {
        debug_assert!(size > 0);
        // Initialize the queue
        let queue = Arc::new(Mutex::new(Queue::<Message<Box<[u8]>>>::new()));

        // Initialize the thread pool
        let pool = Arc::new(Mutex::new(ThreadPool::new(size)));

        // Push out some Connections, with Arc clones of the queue and thread
        // pool
        let mut connections = Vec::with_capacity(size);
        for _ in 0..size {
            connections.push(Connection::new(
                // TODO: connect to multiple socket (ie all of them?)
                ip.clone(),
                Arc::clone(&queue),
                Arc::clone(&pool),
            ));
        }

        SocketPool { connections, queue }
    }

    pub fn size(&self) -> usize { self.connections.len() }

    pub fn enqueue(&mut self, buf: Box<[u8]>) {
        // Lock onto the queue before pushing to it
        (*self.queue.lock().expect("Could not get a lock on queue"))
            .push(Message::Data(buf))
    }
}

impl<S, F> Drop for SocketPool<S, F> {
    fn drop(&mut self) {
        for _ in 0..self.connections.len() {
            (*self.queue.lock().unwrap()).push(Message::Terminate);
        }
        // TODO: make this work properly
    }
}

struct Connection<S, F> {
    _fmarker: PhantomData<F>,
    _smarker: PhantomData<S>,
}

impl<S, F> Connection<S, F>
where
    S: ToSocketAddrs + Send + 'static + Clone,
    F: FnOnce() + 'static,
{
    pub fn new(
        ip: S,
        queue: Arc<Mutex<Queue<Message<Box<[u8]>>>>>,
        pool: Arc<Mutex<ThreadPool>>,
    ) -> Connection<S, F> {
        match pool.lock() {
            Ok(i) => {
                // Ask the thread pool to execute this
                i.execute(move || {
                    // Try to establish a TCP connection
                    let mut stream = {
                        let mut loop_count = 0;
                        loop {
                            // Five tries
                            if loop_count > 5 {
                                // Give up
                                error!("Could not connect. Giving up...");
                                return;
                            } else {
                                match TcpStream::connect(ip.clone()) {
                                    Ok(s) => break s,
                                    Err(e) => {
                                        warn!("Could not connect. Trying again...");
                                        loop_count += 1;
                                    },
                                }
                            }
                        }
                    };
                    loop {
                        // If there's a message in the queue, depending on the
                        // type...
                        if let Some(i) = (*queue.lock().unwrap()).pop() {
                            match i {
                                // either write its data to the stream...
                                Message::Data(d) => {
                                    stream
                                        .write(&*d)
                                        .expect("Could not write to stream");
                                },
                                // or terminate this connection.
                                Message::Terminate => break, // Should be `return;`?
                            }
                        }
                    }
                })
            },
            Err(e) => (), /* Do nothing for now. If this connection can't get
                           * a lock, then it can just give up and try again
                           * (or another connection will try) */
        }
        Connection {
            _fmarker: PhantomData,
            _smarker: PhantomData,
        }
    }
}

struct Queue<T> {
    inner: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Queue<T> {
        Queue {
            inner: VecDeque::<T>::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Queue<T> {
        Queue {
            inner: VecDeque::<T>::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    #[inline]
    pub fn pop(&mut self) -> Option<T> { self.inner.pop_front() }

    #[inline]
    pub fn push(&mut self, item: T) { self.inner.push_back(item) }
}

enum Message<T> {
    Data(T),
    Terminate,
}
