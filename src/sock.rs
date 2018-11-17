use super::thread::ThreadPool;
use std::{
    collections::VecDeque,
    io::Write,
    marker::PhantomData,
    net::{TcpStream, ToSocketAddrs},
    sync::{Arc, Mutex},
};

struct SocketPool<S, F> {
    connections: Vec<Connection<S, F>>,
    queue: Arc<Mutex<Queue<Message<Box<[u8]>>>>>,
}

impl<S: ToSocketAddrs + Send + 'static + Clone, F: FnOnce() + 'static>
    SocketPool<S, F>
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

impl<S: ToSocketAddrs + Send + 'static + Clone, F: FnOnce() + 'static>
    Connection<S, F>
{
    pub fn new(
        ip: S,
        queue: Arc<Mutex<Queue<Message<Box<[u8]>>>>>, /* BUG: this will not
                                                       * have a static
                                                       * lifetime */
        pool: Arc<Mutex<ThreadPool>>,
    ) -> Connection<S, F> {
        (*pool.lock().expect("Lock failed")).execute(move || {
            let mut stream = TcpStream::connect(ip)
                .expect("Failed to initialize connection");
            loop {
                // If there's a message in the queue, depending on the type...
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
        });
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
