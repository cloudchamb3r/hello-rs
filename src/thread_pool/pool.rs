use super::pool_creation_err::PoolCreationError;

use std::sync::mpsc;
use std::sync::Arc; // atomic reference counter
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;

type Job = Box<dyn FnOnce() + Send + 'static>;


/// why created this message? 
/// ThreadPool::Drop does not occur in normal case 
/// because each thread always waiting for a job
/// so by wrapping job as Message that is consist of `Job` and `Terminate`, 
/// we can make Worker Drop`
enum Message {
    NewJob(Job), 
    Terminate, 
}

///
/// The struct `Worker` does the `Job`
///
/// each `Worker` is identified by unique id
/// each `Worker` mapped to one thread that performing `Job`
///
struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        println!("worker with id {} created!", id);
        let thread = thread::spawn(move || {
            // mutex scope is while ~ job line  -> not actually multi-threading
            // while let Ok(job) = receiver.lock().unwrap().recv()  {
            //     println!("Worker {} got a job; executing.", id);
            //     job();
            // }
            // mutex guard released
            loop {
                // single line mutex guard scope
                // let job = receiver.lock().unwrap().recv().unwrap();
                // mutex guard released

                match receiver.lock().unwrap().recv().unwrap() {
                    Message::NewJob(job) => {
                        println!("[+] Worker {} got a job; executing.", id);
                        job();
                    },
                    Message::Terminate => {
                        println!("[-] Worker {} was told to terminate", id);
                        break;
                    },
                }
            }
        });
        Worker { id, thread: Some(thread) }
    }
}

///
/// The struct `ThreadPool` handles set of `Worker`s
///
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Result<ThreadPool, PoolCreationError> {
        if size == 0 {
            return Err(PoolCreationError);
        }

        let (sender, receiver) = mpsc::channel();

        // in principle, receiver is owned by one object.
        // but to implement thread pool we need to share this ownership with multiple threads 
        // by using Atomic Reference counter with Mutex
        // we can assure sharing resource thread safely (A) and sharing ownership safely (Rc)
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        
        Ok(ThreadPool { workers, sender })
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in  &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);


            // join() takes ownership 
            // but in this case we borrowed worker as &mut 
            // so convert worker.thread's type into Option( OrgType )
            // we can take ownership from borrowed value, and replace those value to None 
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
