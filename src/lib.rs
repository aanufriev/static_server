use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::fs;

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender }
    }
}

impl ThreadPool {
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.send(Message::NewJob(job)).expect("couldnt't send job to workers");
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending terminate message to all workers.");

        for _ in &self.workers {
            self.sender.send(Message::Terminate).expect("couldn't stop worker");
        }

        println!("Shutting down all workers.");

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().expect("couldn't finish thread");
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver
                        .lock()
                        .expect("couldn't lock mutex")
                        .recv()
                        .expect("couldn't get job from channel");

            match message {
                Message::NewJob(job) => {
                    println!("Worker {} got a job; executing.", id);

                    job();
                }
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);

                    break;
                }
            }
        });

        Worker {
            id: id,
            thread: Some(thread)
        }
    }
}

const CONFIG_FILE: &str = "/etc/httpd.conf";

const DEFAULT_THREAD_LIMIT: usize = 4;

pub struct Config {
    pub thread_limit: usize,
    pub document_root: String,
}

pub fn read() -> Result<Config, std::io::Error> {
    let mut config = Config {
        thread_limit: DEFAULT_THREAD_LIMIT,
        document_root: String::from(""),
    };

    let config_str = fs::read_to_string(CONFIG_FILE)?;
    let lines: Vec<&str> = config_str.split("\n").collect();

    for line in lines.iter() {
        let key_value: Vec<&str> = line.splitn(2, " ").collect();
        if key_value.len() < 2 {
            continue;
        }
        let name = key_value[0];
        let value = key_value[1];
        match name {
            "thread_limit" => config.thread_limit = value.parse().unwrap(),
            "document_root" => config.document_root = String::from(value),
            _ => (),
        }
    }

    Ok(config)
}
