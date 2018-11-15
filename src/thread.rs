extern crate num_cpus;

use std::{
    default::Default,
    process,
    sync::{mpsc, Arc, Mutex},
    thread as std_thread,
};

// MARK: ThreadPool

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        debug_assert!(size > 0); // Creating a threadpool with 0 threads would just be useless

        // Start up the message passing system
        let (sender, receiver) = mpsc::channel();

        // Wrap up the receiver for thread safety
        let receiver = Arc::new(Mutex::new(receiver));

        // Create a vector of workers to... well, work with
        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            // Push out the new workers with atomic references to the receiver.
            // This receiver will be shared by all of the workers
            workers.push(Worker::new(Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }

    pub fn size(&self) -> usize { self.workers.len() }

    pub fn execute<F>(&self, f: F)
    where
        F: Send + FnOnce() + 'static,
    {
        // Box the function in preparation for message passing
        let new_job = Box::new(f);

        // Send the job as an "Execute" message
        self.sender
            .send(Message::Execute(new_job))
            .expect("Could not send a job")
    }
}

impl Default for ThreadPool {
    fn default() -> ThreadPool {
        // Number of threads defaults to the number of cpus
        ThreadPool::new(num_cpus::get())
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Instructing all workers to terminate...");
        // First send the Terminate message to all the workers
        for _ in 0..self.workers.len() {
            self.sender
                .send(Message::Terminate)
                .expect("Error sending message");
        }

        // Then join all of the threads/workers into the main thread
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id());

            // `take()` replaces the thread with a None value
            if let Some(thread) = worker.thread.take() {
                thread.join().expect(
                    format!("Could not join thread {}", worker.id()).as_str(),
                );
            }
        }
    }
}

// MARK: Worker

struct Worker {
    thread: Option<std_thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        // Spawn a thread that loops, looking for messages
        let thread = std_thread::spawn(move || {
            loop {
                let message = receiver
                    .lock()
                    .expect("Could not lock receiver")
                    .recv()
                    .expect("Error receiving message");
                match message {
                    Message::Execute(job) => {
                        println!(
                            "[Worker {}] Got a job. Executing...",
                            process::id()
                        );
                        job.call_box();
                    },
                    Message::Terminate => {
                        println!(
                            "[Worker {}] Instructed to terminate. Breaking \
                             loop...",
                            process::id()
                        );
                        break; // Breaks out of the loop to prevent endless blocking on
                               // thread join
                    },
                }
            }
        });
        Worker {
            thread: Some(thread),
        }
    }

    pub fn id(&self) -> u32 { process::id() }
}

// MARK: Job, etc.

type Job = Box<dyn FnBox + Send + 'static>;

enum Message {
    Execute(Job),
    Terminate,
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) { (*self)() }
}
