extern crate num_cpus;

use std::thread as std_thread;
use std::sync::mspc;
use std::sync::Arc;
use std::sync::Mutex;
use std::process;
use std::default::Default;

// MARK: ThreadPool

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mspc::Sender<Message>
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        debug_assert!(size > 0); // Redundant? :(

        // Start up the message passing system
        let (sender, receiver) = mspc::channel();

        // Wrap up the receiver for thread safety
        let receiver = Arc::new(Mutex::new(receiver));

        // Create a vector of workers to... well, work with
        let mut workers = Vec::with_capacity(size)
        for i in 0..size {
            // Push out the new workers with atomic clones of the receiver
            workers.push(Worker::new(Arc::clone(&receiver)));
        }
    }

    pub fn size(&self) -> usize {
        self.workers.len()
    }

    pub fn execute<F>(&self, f: F) where F: Send + FnOnce() + 'static {
        // Box the function in preparation for message passing
        let new_job = Box::new(f);

        // Send the job as an "Execute" message
        self.sender.send(Message::Execute(new_job)).expect("Could not send a job")
    }
}

impl Default for ThreadPool {
    fn default() -> ThreadPool {
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
                    format!("Could not join thread {}", worker.id()),
                );
            }
        }
    }
}

