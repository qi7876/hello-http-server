use std::{
    sync::{Arc, Mutex, mpsc},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);
        let mut workers = Vec::with_capacity(size);
        let (sender, receiver) = mpsc::channel();
        let sender = Some(sender);
        let receiver = Arc::new(Mutex::new(receiver));
        for i in 0..size {
            workers.push(Worker::new(i, Arc::clone(&receiver)));
        }

        Self { workers, sender }
    }

    /// Execute a job.
    ///
    /// # Panics
    ///
    /// The `execute` function will panic if sending the job to workers fails.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Job = Box::new(f);
        self.sender
            .as_ref()
            .unwrap()
            .send(job)
            .expect("Errors occur when the thread pool sends a job to the workers!");
    }
}

impl Drop for ThreadPool {
    /// Graceful shutdown and cleanup.
    fn drop(&mut self) {
        self.sender = None;
        for worker in self.workers.drain(..) {
            println!("Shutting down worker {}!", worker.id);
            worker.thread.join().unwrap();
        }
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || {
            loop {
                let message = receiver.lock().unwrap().recv();

                match message {
                    Ok(job) => {
                        println!("Worker {id} got a job!");
                        job();
                        println!("Worker {id} finish a job!");
                    }
                    Err(_) => {
                        println!("Worker {id} disconnected!");
                        break;
                    }
                }
            }
        });
        Worker { id, thread }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[test]
    #[should_panic]
    fn rejects_zero_workers() {
        ThreadPool::new(0);
    }

    #[test]
    fn execute_a_job() {
        let pool = ThreadPool::new(2);
        let (tx, rx) = mpsc::channel();
        let message = String::from("Complete the job!");

        pool.execute(move || tx.send(message).unwrap());

        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("Time out for rx!");

        assert_eq!(result, "Complete the job!");
    }
}
