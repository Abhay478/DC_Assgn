use std::{
    collections::{BinaryHeap, HashMap},
    fs::File,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use polling::{Event, Events, Poller};
use rand::{distributions::Uniform, thread_rng};

use crate::{
    request::Request,
    utils::{get_a_stream, get_msgs, Action, LogEntry, Message, MessageType},
    Params, Region,
};

pub struct MaekawaNode {
    id: (u64, u64), // grid coordinates
    ips: HashMap<(u64, u64), SocketAddr>,
    rx: TcpListener, // Quorum listener
    init: Instant,
    seq: AtomicU64,
}

impl MaekawaNode {
    pub fn new(id: (u64, u64), ips: HashMap<(u64, u64), SocketAddr>) -> Self {
        Self {
            id,
            rx: TcpListener::bind(ips.get(&id).unwrap()).unwrap(),
            ips,
            init: Instant::now(),
            seq: 0.into(),
        }
    }

    fn request_cs(&self, streams: &Vec<TcpStream>) -> Poller {
        for stream in streams.iter() {
            self.send(stream, MessageType::Request);
        }
        self.seq.fetch_add(1, Ordering::SeqCst);
        let poller = Poller::new().unwrap();
        streams.iter().enumerate().for_each(|(i, x)| unsafe {
            poller.add(x, Event::readable(i)).unwrap();
        });
        poller
    }

    fn enter_cs(&self, streams: &mut Vec<TcpStream>, q: usize) -> Vec<LogEntry> {
        // Send a request to all the nodes in the quorum
        let poller = self.request_cs(streams);
        // println!("Request sent");

        // wait for the quorum to reply
        let mut replies = 0;
        let mut failed = false;
        let mut out = vec![];
        let mut inqs = vec![];

        let mut events = Events::new();
        while replies < 2 * q - 1 {
            events.clear();
            poller.wait(&mut events, None).unwrap();

            for ev in events.iter() {
                let stream = streams.get_mut(ev.key).unwrap();
                let mut buf = [0; 1024];
                match stream.read(&mut buf) {
                    Ok(ref b) if *b > 0 => {
                        let msg = Message::from(&buf[..*b]);
                        match msg.typ {
                            MessageType::Reply => {
                                // println!("Reply: {:#?}", msg.id);
                                replies += 1;
                                self.log(&mut out, Action::Reply(msg.id));
                            }
                            MessageType::Failed => {
                                // println!("Failed {:#?}", msg.id);
                                failed = true;
                                for stream in inqs.iter() {
                                    self.send(stream, MessageType::Yield);
                                }
                            }
                            MessageType::Inquire => {
                                // println!("Inquire {:#?}.", msg.id);
                                if failed {
                                    self.send(stream, MessageType::Yield);
                                } else {
                                    inqs.push(stream.try_clone().expect("Clone failed."));
                                }
                            }
                            _ => {
                                panic!("Unexpected message")
                            }
                        }
                    }
                    _ => {
                        // thread::sleep(Duration::from_micros(10));
                    }
                }
                buf.fill(0);
                poller.modify(stream, Event::readable(ev.key)).unwrap();
            }
        }

        // println!("CS acquired");

        out
    }

    fn exit_cs(&self, streams: &Vec<TcpStream>) {
        for stream in streams.iter() {
            self.send(stream, MessageType::Release);
        }
        // println!("CS released");
    }

    fn get_streams(&self, q: usize, ips: &HashMap<(u64, u64), SocketAddr>) -> Vec<TcpStream> {
        vec![
            vec![get_a_stream(ips.get(&(self.id.0, self.id.1)).unwrap())],
            (0..q)
                .filter(|&i| i != self.id.0 as usize)
                .map(|i| get_a_stream(ips.get(&(i as u64, self.id.1)).unwrap()))
                .collect::<Vec<TcpStream>>(),
            (0..q)
                .filter(|&i| i != self.id.1 as usize)
                .map(|i| get_a_stream(ips.get(&(self.id.0, i as u64)).unwrap()))
                .collect::<Vec<TcpStream>>(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<TcpStream>>()
    }

    fn terminate(&self, streams: &Vec<TcpStream>) {
        for stream in streams.iter() {
            self.send(stream, MessageType::Terminate);
        }
    }

    fn log(&self, out: &mut Vec<LogEntry>, act: Action) {
        out.push(LogEntry {
            pid: self.id,
            ts: Instant::now(),
            act,
        });
    }

    fn requester_thread(&self, params: &Params, q: usize) -> Vec<LogEntry> {
        let mut rng = thread_rng();
        let u = Uniform::new(0.0, 1.0);
        let mut out = vec![];

        // All the nodes in the quorum
        let mut streams = self.get_streams(q, &self.ips);

        // Send a request to all the nodes in the quorum
        for _i in 0..params.k {
            self.log(&mut out, Action::Internal);
            params.sleep(u, &mut rng, Region::Out);

            let replies = self.enter_cs(&mut streams, q);
            out.extend(replies);

            self.log(&mut out, Action::Acquire);
            params.sleep(u, &mut rng, Region::In);

            self.exit_cs(&mut streams);
            self.log(&mut out, Action::Exit);
        }

        self.terminate(&mut streams);

        out
        // }
    }

    fn get_quorum_poller(&self, q: usize) -> (Poller, Vec<TcpStream>) {
        let mut streams = vec![];

        let poller = Poller::new().unwrap();
        for stream in self.rx.incoming() {
            // println!("-");
            match stream {
                Ok(x) => {
                    unsafe { poller.add(&x, Event::readable(streams.len())).unwrap() };
                    streams.push(x);
                    // println!("New connection.");
                }
                Err(_) => {
                    // thread::sleep(Duration::from_micros(10));
                }
            }

            // All connections acquired
            if streams.len() == 2 * q - 1 {
                break;
            }
        }

        (poller, streams)
    }

    fn send(&self, mut stream: &TcpStream, typ: MessageType) {
        stream
            .write_all(&*<Message as Into<Vec<u8>>>::into(Message::new(
                self.id,
                typ,
                self.seq.load(Ordering::SeqCst) as u128,
            )))
            .unwrap();
    }

    fn listen(&self, poller: Poller, streams: &Vec<TcpStream>, q: usize) -> Vec<LogEntry> {
        // let mut req = vec![];
        let mut req = BinaryHeap::new();
        let mut locked: Option<Request> = None;
        let mut inq = false; // Have we sent an inquire message already?
        let mut out = vec![];
        let mut term = 0;
        let mut events = Events::new();

        while term < 2 * q - 1 {
            events.clear();
            poller.wait(&mut events, None).unwrap();
            for ev in events.iter() {
                let mut stream = streams.get(ev.key).unwrap();
                let mut buf = [0; 128];
                match stream.read(&mut buf) {
                    Ok(ref b) if *b > 0 => {
                        let msgs = get_msgs(&buf, b);

                        for msg in msgs {
                            match msg.typ {
                                MessageType::Request => {
                                    self.log(&mut out, Action::Query(msg.id));
                                    if msg.ts > self.seq.load(Ordering::SeqCst) as u128 {
                                        self.seq.store((msg.ts + 1) as u64, Ordering::SeqCst);
                                    }
                                    if let Some(ref t) = locked {
                                        if msg.ts > t.ts
                                            || req.iter().fold(false, |acc, x: &Request| {
                                                acc || x.ts < msg.ts
                                            })
                                        {
                                            // If the request is younger than the locked request
                                            self.send(stream, MessageType::Failed);
                                        } else if !inq {
                                            self.send(t.stream, MessageType::Inquire);
                                            inq = true;
                                        }

                                        // Need to put in queue anyway.
                                        req.push(Request::new(msg.ts, msg.id, stream));
                                        // println!("Request queued: {:#?}", msg.id);
                                    } else {
                                        locked = Some(Request::new(msg.ts, msg.id, stream));
                                        self.log(&mut out, Action::Grant(msg.id));
                                        inq = false;
                                        self.send(stream, MessageType::Reply);
                                        // println!("Grant sent {:#?}", msg.id);
                                    }
                                }
                                MessageType::Release => {
                                    self.log(&mut out, Action::Release(msg.id));
                                    if locked.is_none() {
                                        // dbg!(msg);
                                        panic!("Tf?");
                                    }
                                    inq = false;
                                    if req.is_empty() {
                                        locked = None;
                                    } else {
                                        let next = req.pop().unwrap();
                                        // println!("Grant: {:#?}", next.pid);
                                        self.log(&mut out, Action::Grant(msg.id));
                                        self.send(next.stream, MessageType::Reply);
                                        locked = Some(next);
                                    }
                                }
                                MessageType::Yield => {
                                    // self.send(stream, MessageType::Reply);
                                    match locked {
                                        Some(ref t) if t.pid == msg.id => {
                                            if req.is_empty() {
                                                // dbg!(msg);
                                                panic!("Bad yield.");
                                            } else {
                                                // println!("Yielded: {:#?}", msg.id);
                                                let next = req.pop().unwrap();
                                                req.push(locked.take().unwrap());
                                                // println!("Grant: {:#?}", next.pid);
                                                self.log(&mut out, Action::Grant(msg.id));
                                                self.send(next.stream, MessageType::Reply);
                                                locked = Some(next);
                                            }
                                        }
                                        None => {
                                            panic!("Yielding when locked empty.");
                                        }
                                        _ => {
                                            // println!("Heh?");
                                        } // Old yield?
                                    }
                                }
                                MessageType::Terminate => {
                                    term += 1;
                                }
                                _ => {
                                    panic!("Unexpected message")
                                }
                            }
                        }
                    }
                    _ => {
                        // thread::sleep(Duration::from_micros(10))
                    }
                }
                // Clear buffer
                buf.fill(0);
                // Reset poller
                poller.modify(stream, Event::readable(ev.key)).unwrap();
            }
        }

        out
    }

    /// Listen for incoming messages
    fn quorum_thread(&self, q: usize) -> Vec<LogEntry> {
        let (poller, streams) = self.get_quorum_poller(q);

        // println!("Quorum ready: {}", streams.len());

        self.listen(poller, &streams, q)
    }

    fn quorum_spawn(self: Arc<Self>, q: usize) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.quorum_thread(q))
    }

    fn requester_spawn(self: Arc<Self>, params: Params, q: usize) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.requester_thread(&params, q))
    }

    pub fn spawn(self: Arc<Self>, params: Params) {
        let mut file = File::create(format!("log/node_{}_{}.log", self.id.0, self.id.1)).unwrap();
        let q = (params.n as f64).sqrt() as usize;

        // let init = Instant::now();
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
