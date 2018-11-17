use super::thread::ThreadPool;
use std::{
    collections::VecDeque,
    io::Write,
    marker::PhantomData,
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

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
        let queue = Arc::new(Mutex::new(Queue::<Message<Box<[u8]>>>::new()));

        let pool = Arc::new(Mutex::new(ThreadPool::new(size)));

        let mut connections = Vec::new();
        for _ in 0..size {
            connections.push(Connection::new(
                ip.clone(),
                Arc::clone(&queue),
                Arc::clone(&pool),
            ));
        }

        SocketPool { connections, queue }
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
        queue: Arc<Mutex<Queue<Message<Box<[u8]>>>>>, /* BUG: this will not
                                                       * have a static
                                                       * lifetime */
        pool: Arc<Mutex<ThreadPool>>,
    ) -> Connection<S, F> {
        // (*pool.lock().expect("Lock failed"));
        match pool.lock() {
            Ok(i) => {
                i.execute(move || {
                    let mut stream = {
                        let mut loop_count = 0;
                        loop {
                            if loop_count > 5 {
                                println!("Could not connect. Giving up...");
                                return;
                            } else {
                                match TcpStream::connect(ip.clone()) {
                                    Ok(s) => break s,
                                    Err(e) => {
                                        println!(
                                            "Could not connect. Trying \
                                             again..."
                                        );
                                        loop_count += 1;
                                    },
                                }
                            }
                        }
                    };
                    // TcpStream::connect(ip)
                    // .expect("Failed to initialize connection");
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
                                Message::Terminate => break,
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

    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    pub fn pop(&mut self) -> Option<T> { self.inner.pop_back() }

    pub fn push(&mut self, item: T) { self.inner.push_front(item) }
}

enum Message<T> {
    Data(T),
    Terminate,
}
