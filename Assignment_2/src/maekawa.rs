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

use either::Either::Left;
use polling::{Event, Events, Poller};
use rand::{distributions::Uniform, thread_rng};

use crate::{
    request::Request,
    utils::{get_a_stream, get_msgs, Action, LogEntry, Message, MessageType},
    Params, Region,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestStatus {
    Pending,
    Granted,
    Inquiring,
    Failed,
}

pub struct MaekawaNode {
    id: (u64, u64), // grid coordinates
    ips: HashMap<(u64, u64), SocketAddr>,
    rx: TcpListener, // Quorum listener
    pub init: Instant,
    seq: AtomicU64, // lamport clock
    pub mc: AtomicU64,
}

impl MaekawaNode {
    pub fn new(id: (u64, u64), ips: HashMap<(u64, u64), SocketAddr>) -> Self {
        Self {
            id,
            rx: TcpListener::bind(ips.get(&id).unwrap()).unwrap(),
            ips,
            init: Instant::now(),
            seq: 0.into(),
            mc: 0.into(),
        }
    }

    /// Sends messages
    fn send(&self, mut stream: &TcpStream, typ: MessageType) {
        self.mc.fetch_add(1, Ordering::SeqCst);
        stream
            .write_all(&*<Message as Into<Vec<u8>>>::into(Message::new_maekawa(
                self.id,
                typ,
                self.seq.load(Ordering::SeqCst) as u128,
            )))
            .unwrap();
    }

    /// Generates log entries
    fn log(&self, out: &mut Vec<LogEntry>, act: Action) {
        out.push(LogEntry {
            pid: Left(self.id),
            ts: self.init.elapsed().as_millis(),
            act,
        });
    }

    fn get_requester_poller(&self, streams: &Vec<TcpStream>) -> Poller {
        let poller = Poller::new().unwrap();
        streams.iter().enumerate().for_each(|(i, x)| unsafe {
            poller.add(x, Event::readable(i)).unwrap();
        });
        poller
    }

    /// Send request to all endpoints in the quorum
    fn request_cs(&self, streams: &Vec<TcpStream>) {
        for stream in streams.iter() {
            self.send(stream, MessageType::Request);
        }
        self.seq.fetch_add(1, Ordering::SeqCst);
    }

    /// Wait for quorum replies
    fn enter_cs(&self, streams: &mut Vec<TcpStream>, poller: &Poller, q: usize) -> Vec<LogEntry> {
        // Send a request to all the nodes in the quorum
        self.request_cs(streams);
        println!("Request sent");

        // wait for the quorum to reply
        let mut out = vec![];
        let mut status = vec![RequestStatus::Pending; 2 * q - 1];

        let mut events = Events::new();
        while status.contains(&RequestStatus::Pending) || status.contains(&RequestStatus::Failed) {
            events.clear();
            poller.wait(&mut events, None).unwrap();

            for ev in events.iter() {
                let mut stream = streams.get(ev.key).unwrap();
                let mut buf = [0; 1024];
                match stream.read(&mut buf) {
                    Ok(ref b) if *b > 0 => {
                        let msg = Message::from(&buf[..*b]);
                        match msg.typ {
                            MessageType::Reply => {
                                // println!("Reply: {:#?}", msg.id);
                                status[ev.key] = RequestStatus::Granted;
                                self.log(&mut out, Action::Reply(msg.id));
                            }
                            MessageType::Failed => {
                                // println!("Failed {:#?}", msg.id);
                                status[ev.key] = RequestStatus::Failed;
                                for (ss, stat) in streams
                                    .iter()
                                    .zip(status.iter_mut())
                                    .filter(|x| x.1 == &RequestStatus::Inquiring)
                                {
                                    self.send(ss, MessageType::Yield);
                                    *stat = RequestStatus::Pending;
                                }
                            }
                            MessageType::Inquire => {
                                // println!("Inquire {:#?}.", msg.id);
                                if status.contains(&RequestStatus::Failed)
                                    && status[ev.key] != RequestStatus::Inquiring
                                {
                                    self.send(stream, MessageType::Yield);
                                    status[ev.key] = RequestStatus::Pending;
                                } else {
                                    status[ev.key] = RequestStatus::Inquiring;
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

        println!("CS acquired");

        out
    }

    /// Send release to all endpoints in the quorum
    fn exit_cs(&self, streams: &Vec<TcpStream>) {
        for stream in streams.iter() {
            self.send(stream, MessageType::Release);
        }
        println!("CS released");
    }

    /// Get the streams for the quorum
    fn get_streams(&self, q: usize) -> Vec<TcpStream> {
        vec![
            vec![get_a_stream(self.ips.get(&(self.id.0, self.id.1)).unwrap())],
            (0..q)
                .filter(|&i| i != self.id.0 as usize)
                .map(|i| get_a_stream(self.ips.get(&(i as u64, self.id.1)).unwrap()))
                .collect::<Vec<TcpStream>>(),
            (0..q)
                .filter(|&i| i != self.id.1 as usize)
                .map(|i| get_a_stream(self.ips.get(&(self.id.0, i as u64)).unwrap()))
                .collect::<Vec<TcpStream>>(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<TcpStream>>()
    }

    /// Indicates alggorithm termination
    fn terminate(&self, streams: &mut Vec<TcpStream>) {
        for stream in streams.iter_mut() {
            self.send(stream, MessageType::Terminate);
            // stream.flush().unwrap();
        }
    }

    /// Simulate CS requests
    fn requester_thread(&self, params: &Params, q: usize) -> Vec<LogEntry> {
        let mut rng = thread_rng();
        let u = Uniform::new(0.0, 1.0);
        let mut out = vec![];

        // All the nodes in the quorum
        let mut streams = self.get_streams(q);
        let poller = self.get_requester_poller(&streams);

        // Send a request to all the nodes in the quorum
        for _i in 0..params.k {
            self.log(&mut out, Action::Internal);
            params.sleep(u, &mut rng, Region::Out);

            let replies = self.enter_cs(&mut streams, &poller, q);
            out.extend(replies);

            self.log(&mut out, Action::Acquire);
            params.sleep(u, &mut rng, Region::In);

            self.exit_cs(&mut streams);
            self.log(&mut out, Action::Exit);
        }

        self.terminate(&mut streams);
        println!("Node {:?} sent terminate.", self.id);

        out
    }

    fn requester_spawn(self: Arc<Self>, params: Params, q: usize) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.requester_thread(&params, q))
    }

    fn get_listener_poller(&self, q: usize) -> (Poller, Vec<TcpStream>) {
        let mut streams = vec![];

        let poller = Poller::new().unwrap();
        for stream in self.rx.incoming() {
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

    /// Listen for incoming messages
    fn listen(&self, poller: Poller, streams: &Vec<TcpStream>, q: usize) -> Vec<LogEntry> {
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
                                    // Lamport clock
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
                                        req.push(Request::new(
                                            msg.ts,
                                            msg.id.expect_left(""),
                                            stream,
                                        ));
                                        // println!("Request queued: {:#?}", msg.id);
                                    } else {
                                        locked = Some(Request::new(
                                            msg.ts,
                                            msg.id.expect_left(""),
                                            stream,
                                        ));
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
                                    self.send(stream, MessageType::Reply);
                                    match locked {
                                        Some(ref t) if t.pid == msg.id.expect_left("") => {
                                            if req.is_empty() {
                                                if msg.ts < self.seq.load(Ordering::SeqCst) as u128
                                                {
                                                    dbg!(msg);
                                                    panic!("Bad yield to {:#?}.", self.id);
                                                }
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
                                            if msg.ts < locked.unwrap().ts {
                                                dbg!(msg);
                                                dbg!(&locked);
                                                panic!("Weird yield to {:#?}.", self.id);
                                            }
                                        } // Old yield?
                                    }
                                }
                                MessageType::Terminate => {
                                    term += 1;
                                    println!(
                                        "Node {:?} received terminate from {:?}.",
                                        self.id,
                                        msg.id.expect_left("")
                                    );
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

    fn listener_thread(&self, q: usize) -> Vec<LogEntry> {
        let (poller, streams) = self.get_listener_poller(q);

        println!("Quorum ready: {}", streams.len());

        self.listen(poller, &streams, q)
    }

    fn listener_spawn(self: Arc<Self>, q: usize) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.listener_thread(q))
    }

    /// Initiates node execution
    pub fn spawn(self: Arc<Self>, params: Params) {
        let mut file =
            File::create(format!("log/maekawa/node_{}_{}.log", self.id.0, self.id.1)).unwrap();
        let q = (params.n as f64).sqrt() as usize;

        // Spawn a new thread to listen for incoming messages
        let listener = self.clone().listener_spawn(q);
        // println!("Listener spawned.");

        // Spawn a new thread to request CS.
        let node = self.clone().requester_spawn(params, q);
        // println!("Requester spawned.");

        // Wait for the threads to finish
        let listener_log = listener.join().unwrap();
        let node_log = node.join().unwrap();

        let mut log = [listener_log, node_log].concat();
        log.sort_by_key(|x| x.ts);

        // TODO: Make a new display function for LogEntry
        for entry in log.iter() {
            writeln!(file, "{}", entry).unwrap();
        }
    }
}
