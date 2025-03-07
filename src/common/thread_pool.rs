// src/common/thread_pool.rs

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tokio::task::JoinHandle;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl ThreadPool {
    /// Create a new ThreadPool with the specified number of threads.
    ///
    /// # Errors
    /// Returns an error if num_threads is 0.
    pub fn new(num_threads: usize) -> Result<ThreadPool, &'static str> {
        if num_threads == 0 {
            return Err("Thread pool size must be greater than 0");
        }

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(num_threads);

        for id in 0..num_threads {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Ok(ThreadPool {
            workers,
            sender: Some(sender),
        })
    }

    /// Execute a function in the thread pool and return a handle to its result.
    ///
    /// # Errors
    /// Returns an error if the thread pool has been shut down.
    pub fn execute<F, T>(&self, f: F) -> Result<JoinHandle<T>, &'static str>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let sender = self.sender.as_ref().ok_or("ThreadPool has been shut down")?;

        let (promise_sender, promise_receiver) = tokio::sync::oneshot::channel();

        sender.send(Box::new(move || {
            let result = f();
            let _ = promise_sender.send(result);
        })).map_err(|_| "Failed to send job to thread pool")?;

        Ok(tokio::spawn(async move {
            promise_receiver.await.expect("Task panicked")
        }))
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Drop the sender to signal workers to shut down
        drop(self.sender.take());

        // Join all worker threads
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().expect("Failed to join worker thread");
            }
        }
    }
}

impl Worker {
    fn new(_id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = {
                let receiver = receiver.lock().expect("Mutex was poisoned");
                receiver.recv()
            };

            match message {
                Ok(job) => {
                    job();
                }
                Err(_) => {
                    // Channel closed, time to exit
                    break;
                }
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn test_thread_pool_execution() {
        let pool = ThreadPool::new(4).unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        for _ in 0..10 {
            let counter = Arc::clone(&counter);
            let handle = pool.execute(move || {
                thread::sleep(Duration::from_millis(10));
                counter.fetch_add(1, Ordering::SeqCst);
            }).unwrap();
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[tokio::test]
    async fn test_thread_pool_shutdown() {
        let counter = Arc::new(AtomicUsize::new(0));
        {
            let pool = ThreadPool::new(4).unwrap();
            let counter_clone = Arc::clone(&counter);
            let handle = pool.execute(move || {
                thread::sleep(Duration::from_millis(50));
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }).unwrap();
            
            // Wait for the task to complete
            handle.await.unwrap();
            // Pool will be dropped here
        }
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
