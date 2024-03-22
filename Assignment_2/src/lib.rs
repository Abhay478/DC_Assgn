#![allow(dead_code)]

use polling::{Event, Events, Poller};
use rand::{
    distributions::{Distribution, Uniform},
    thread_rng,
};
use serde_derive::{Deserialize, Serialize};
use serde_json::to_vec;

use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::{Write, Read},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
pub struct Params {
    n: usize,
    k: usize,
    out_l: f64,
    in_l: f64,
}

impl Params {
    pub fn new() -> Self {
        let mut file = File::open("inp-params.txt").unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let q = buf
            .split_whitespace()
            .map(|x| x.parse::<f64>().unwrap())
            .collect::<Vec<f64>>();

        Self {
            n: q[0] as usize,
            k: q[1] as usize,
            out_l: q[2],
            in_l: q[3],
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Action {
    Internal,
    Query,
    Reply,
    Request, // Message Type
    Grant,   // Message Type
    Acquire,
    Release,   // Message Type
    Terminate, // Message Type
}
impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Self::Internal => "Internal",
                Self::Acquire => "Acquire",
                Self::Release => "Release",
                Self::Query => "Query",
                Self::Reply => "Reply",
                Self::Request => "Request",
                Self::Grant => "Grant",
                Self::Terminate => "Terminate",
            }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: (usize, usize),
    pub act: Action,
}

impl Message {
    pub fn new(id: (usize, usize), act: Action) -> Self {
        Self { id, act }
    }
}

/// Logging unit
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub pid: (usize, usize),
    pub ts: Instant,
    pub act: Action,
}

pub fn get_ips() -> HashMap<(usize, usize), SocketAddr> {
    // Read all ip addresses from a file
    let mut file = File::open("ips.txt").unwrap();
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    let mut out = HashMap::new();
    for l in buf.lines() {
        let mut it = l.split_whitespace();
        let id = (
            it.next().unwrap().parse::<usize>().unwrap(),
            it.next().unwrap().parse::<usize>().unwrap(),
        );
        let ip = it.next().unwrap();
        let port = it.next().unwrap();
        let addr = format!("{}:{}", ip, port).parse::<SocketAddr>().unwrap();
        out.insert(id, addr);
    }
    out
}

pub struct MaekawaNode {
    id: (usize, usize), // grid coordinates
    ips: HashMap<(usize, usize), SocketAddr>,
    rx: TcpListener, // Quorum listener
}

impl MaekawaNode {
    pub fn new(id: (usize, usize), ips: HashMap<(usize, usize), SocketAddr>) -> Self {
        Self {
            id,
            rx: TcpListener::bind(ips.get(&id).unwrap()).unwrap(),
            ips,
        }
    }

    fn request_cs(&self, streams: &mut Vec<TcpStream>) {
        for stream in streams.iter_mut() {
            stream
                .write_all(
                    &to_vec(&Message {
                        id: self.id,
                        act: Action::Request,
                    })
                    .unwrap()[..],
                )
                .unwrap();
        }
    }

    fn enter_cs(&self, streams: &mut Vec<TcpStream>, q: usize) {
        // Send a request to all the nodes in the quorum
        self.request_cs(streams);
        println!("Request sent");

        let poller = Poller::new().unwrap();
        streams.iter().enumerate().for_each(|(i, x)| unsafe {
            poller.add(x, Event::readable(i)).unwrap();
        });

        // wait for the quorum to reply
        let mut replies = 0;
        let mut events = Events::new();
        while replies < 2 * q - 1 as usize {
            events.clear();
            poller.wait(&mut events, None).unwrap();
            // println!("Reply.");
            for ev in events.iter() {
                let stream = streams.get_mut(ev.key).unwrap();
                let mut buf = [0; 1024];
                match stream.read(&mut buf) {
                    Ok(ref b) if *b > 0 => {
                        let msg: Message = match serde_json::from_slice(&buf[..*b]) {
                            Ok(x) => {
                                println!(".");
                                x
                            }
                            Err(_e) => {
                                panic!("Heh?");
                            }
                        };
                        match msg.act {
                            Action::Grant => {
                                replies += 1;
                            }
                            _ => {
                                panic!("Unexpected message")
                            }
                        }
                    }
                    _ => {
                        thread::sleep(Duration::from_micros(500));
                    } // Maybe sleep for a while
                }
                poller.modify(stream, Event::readable(ev.key)).unwrap();
            }
        }

        println!("CS acquired");
    }

    fn exit_cs(&self, streams: &mut Vec<TcpStream>) {
        for stream in streams.iter_mut() {
            stream
                .write_all(
                    &to_vec(&Message {
                        id: self.id,
                        act: Action::Release,
                    })
                    .unwrap()[..],
                )
                .unwrap();
        }
    }

    fn get_streams(&self, q: usize, ips: &HashMap<(usize, usize), SocketAddr>) -> Vec<TcpStream> {
        (0..q).filter(|&i| i != self.id.1)
            .map(|i| {
                [
                    loop {
                        match TcpStream::connect(
                            ips.get(&(i, self.id.1)).expect("IP file incomplete"),
                        ) {
                            Ok(x) => {
                                println!("Connected to {}", ips.get(&(i, self.id.1)).unwrap());
                                break x;
                            }
                            Err(_) => {
                                // panic!("Node {} not found", ips.get(&(i, self.id.1)).unwrap())
                                thread::sleep(Duration::from_micros(100));
                            }
                        }
                    },
                    loop {
                        match TcpStream::connect(
                            ips.get(&(self.id.0, i)).expect("IP file incomplete"),
                        ) {
                            Ok(x) => {
                                println!("Connected to {}", ips.get(&(self.id.0, i)).unwrap());
                                break x;
                            }
                            Err(_) => {
                                // panic!("Node {} not found", ips.get(&(self.id.0, i)).unwrap())
                                thread::sleep(Duration::from_micros(100));
                            }
                        }
                    },
                ]
            })
            .flatten()
            .collect::<Vec<TcpStream>>()
    }

    fn terminate(&self, streams: &mut Vec<TcpStream>) {
        for stream in streams.iter_mut() {
            stream
                .write_all(
                    &to_vec(&Message {
                        id: self.id,
                        act: Action::Terminate,
                    })
                    .unwrap()[..],
                )
                .unwrap();
        }
    }

    fn requester_thread(&self, params: &Params, q: usize) -> Vec<LogEntry> {
        // let ips = get_ips();
        let mut rng = thread_rng();
        let u = Uniform::new(0.0, 1.0);
        let mut out = vec![];
        // loop {

        // All the nodes in the quorum
        let mut streams = self.get_streams(q, &self.ips);

        // Send a request to all the nodes in the quorum
        for _i in 0..params.k {
            out.push(LogEntry {
                pid: self.id,
                ts: Instant::now(),
                act: Action::Internal,
            });
            let ts = -(u.sample(&mut rng) as f64).ln() * params.out_l as f64;
            thread::sleep(Duration::from_micros(ts as u64));

            self.enter_cs(&mut streams, q);
            out.push(LogEntry {
                pid: self.id,
                ts: Instant::now(),
                act: Action::Acquire,
            });
            let ts = -(u.sample(&mut rng) as f64).ln() * params.in_l as f64;
            thread::sleep(Duration::from_micros(ts as u64));

            self.exit_cs(&mut streams);
            out.push(LogEntry {
                pid: self.id,
                ts: Instant::now(),
                act: Action::Release,
            });
        }

        self.terminate(&mut streams);

        out
        // }
    }

    /// Listen for incoming messages
    /// Maybe un-indent some of this?
    fn quorum_thread(&self, q: usize) -> Vec<LogEntry> {
        let mut req = vec![];
        let mut taken = false;
        let mut out = vec![];
        let mut term = 0;
        let mut streams = vec![];

        let poller = Poller::new().unwrap();
        for stream in self.rx.incoming() {
            println!("-");
            match stream {
                Ok(x) => {
                    unsafe { poller.add(&x, Event::readable(streams.len())).unwrap() };
                    streams.push(x);
                }
                Err(_) => {thread::sleep(Duration::from_micros(100));}
            }
            if streams.len() == 2*q - 1 {
                break;
            }
        }

        println!("Quorum ready: {}", streams.len());

        let mut events = Events::new();

        // Maybe spawn more threads for this? Or use tokio?
        while term < 2 * q - 1 {
            events.clear();
            poller.wait(&mut events, None).unwrap();
            for ev in events.iter() {
                let mut stream = streams.get(ev.key).unwrap();
                let mut buf = [0; 1024];
                match stream.read(&mut buf) {
                    Ok(ref b) if *b > 0 => {
                        let msg: Message = serde_json::from_slice(&buf[..*b]).unwrap();
                        // dbg!(&msg);
                        match msg.act {
                            Action::Request => {
                                out.push(LogEntry {
                                    pid: self.id,
                                    ts: Instant::now(),
                                    act: Action::Request,
                                });
                                if taken {
                                    req.push(stream);
                                } else {
                                    taken = true;
                                    out.push(LogEntry {
                                        pid: self.id,
                                        ts: Instant::now(),
                                        act: Action::Grant,
                                    });
                                    stream
                                        .write_all(
                                            &to_vec(&Message {
                                                id: self.id,
                                                act: Action::Grant,
                                            })
                                            .unwrap()[..],
                                        )
                                        .unwrap();
                                }
                            }
                            Action::Release => {
                                out.push(LogEntry {
                                    pid: self.id,
                                    ts: Instant::now(),
                                    act: Action::Release,
                                });
                                if !taken {
                                    panic!("Tf?");
                                }
                                // req.retain(|x| x != &msg.id);
                                if req.is_empty() {
                                    taken = false;
                                } else {
                                    let mut next = req.pop().unwrap();
                                    next.write_all(
                                        &to_vec(&Message {
                                            id: self.id,
                                            act: Action::Grant,
                                        })
                                        .unwrap()[..],
                                    )
                                    .unwrap();
                                }
                            }
                            Action::Terminate => {
                                term += 1;
                            }
                            _ => {
                                panic!("Unexpected message")
                            }
                        }
                    }
                    _ => {thread::sleep(Duration::from_micros(100))} // Maybe sleep for a while
                }
            }
            // let mut buf = [0; 1024];
            /* for stream in self.rx.incoming() {
                let mut buf = [0; 1024];
                match stream {
                    Ok(mut stream) => {
                        match stream.read(&mut buf) {
                            Ok(_) => {
                                println!("recvd");
                                let msg: Message = serde_json::from_slice(&buf).unwrap();
                                dbg!(&msg);
                                match msg.act {
                                    Action::Request => {
                                        out.push(LogEntry {
                                            pid: self.id,
                                            ts: Instant::now(),
                                            act: Action::Request,
                                        });
                                        if taken {
                                            req.push(stream);
                                        } else {
                                            taken = true;
                                            out.push(LogEntry {
                                                pid: self.id,
                                                ts: Instant::now(),
                                                act: Action::Grant,
                                            });
                                            stream
                                                .write_all(
                                                    &to_vec(&Message {
                                                        id: self.id,
                                                        act: Action::Grant,
                                                    })
                                                    .unwrap()[..],
                                                )
                                                .unwrap();
                                        }
                                    }
                                    Action::Release => {
                                        out.push(LogEntry {
                                            pid: self.id,
                                            ts: Instant::now(),
                                            act: Action::Release,
                                        });
                                        if !taken {
                                            panic!("Tf?");
                                        }
                                        // req.retain(|x| x != &msg.id);
                                        if req.is_empty() {
                                            taken = false;
                                        } else {
                                            let mut next = req.pop().unwrap();
                                            next.write_all(
                                                &to_vec(&Message {
                                                    id: self.id,
                                                    act: Action::Grant,
                                                })
                                                .unwrap()[..],
                                            )
                                            .unwrap();
                                        }
                                    }
                                    Action::Terminate => {
                                        term += 1;
                                    }
                                    _ => {
                                        panic!("Unexpected message")
                                    }
                                }
                            }
                            Err(_) => {} // Maybe sleep for a while
                        }
                    }
                    Err(_) => {} // Maybe sleep for a while
                }
            }
             */
        }

        out
    }

    fn quorum_spawn(self: Arc<Self>, q: usize) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.quorum_thread(q))
    }

    fn requester_spawn(self: Arc<Self>, params: Params, q: usize) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.requester_thread(&params, q))
    }

    pub fn listen(self: Arc<Self>, params: Params) {
        let mut file = File::create(format!("node_{}_{}.log", self.id.0, self.id.1)).unwrap();
        let q = (params.n as f64).sqrt() as usize;

        let init = Instant::now();
        // Spawn a new thread to listen for incoming messages
        let quorum = self.clone().quorum_spawn(q);

        // Spawn a new thread to request CS.
        let node = self.clone().requester_spawn(params, q);

        // Wait for the threads to finish
        let quorum_log = quorum.join().unwrap();
        let node_log = node.join().unwrap();

        let mut log = [quorum_log, node_log].concat();
        log.sort_by_key(|x| x.ts);

        // TODO: Make a new display function for LogEntry
        for entry in log.iter() {
            writeln!(file, "{:?}", entry).unwrap();
        }
    }
}
