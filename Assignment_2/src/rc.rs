use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use either::Either::Right;
use polling::{Event, Events, Poller};
use rand::{distributions::Uniform, thread_rng};

use crate::{
    utils::{get_a_stream, get_msgs, Action, LogEntry, Message, MessageType},
    Params, Region,
};

pub struct RCNode {
    id: u128,
    ips: HashMap<u128, SocketAddr>,
    rx: TcpListener, // Listener
    pub init: Instant,
    seq: AtomicU64,
    pub mc: AtomicU64,
    req_flag: AtomicBool,
    quorum: Mutex<HashMap<u128, (bool, Option<TcpStream>)>>,
}

impl RCNode {
    pub fn new(id: u128, ips: HashMap<u128, SocketAddr>) -> Self {
        let rx = TcpListener::bind(ips.get(&id).unwrap()).unwrap();

        let quorum = Mutex::new(
            (0..id)
                .map(|x| (x, (true, None))) // Good stuff
                .chain((id..ips.len() as u128).map(|x| (x, (false, None)))) // Meh
                .collect(),
        );

        Self {
            id,
            rx,
            ips,
            init: Instant::now(),
            seq: 0.into(),
            req_flag: false.into(),
            quorum,
            mc: 0.into(),
        }
    }

    fn get_requester_poller(&self, streams: &Vec<(u128, TcpStream)>) -> Poller {
        let poller = Poller::new().unwrap();
        streams.iter().enumerate().for_each(|(i, (_, x))| unsafe {
            poller.add(x, Event::readable(i)).unwrap();
        });
        poller
    }

    fn request_cs(&self, streams: &Vec<(u128, TcpStream)>) -> usize {
        self.req_flag.store(true, Ordering::SeqCst);
        let q = self.quorum.lock().unwrap();
        let mut count = 0;
        for (pid, stream) in streams.iter() {
            if q[&pid].0 {
                count += 1;
                self.send(stream, MessageType::Request);
            }
        }
        self.seq.fetch_add(1, Ordering::SeqCst);
        count
    }

    fn log(&self, out: &mut Vec<LogEntry>, act: Action) {
        out.push(LogEntry {
            pid: Right(self.id),
            ts: self.init.elapsed().as_micros(),
            act,
        });
    }

    fn send(&self, mut stream: &TcpStream, typ: MessageType) {
        self.mc.fetch_add(1, Ordering::SeqCst);
        stream
            .write_all(&*<Message as Into<Vec<u8>>>::into(Message::new_rc(
                self.id,
                typ,
                self.seq.load(Ordering::SeqCst) as u128,
            )))
            .unwrap();
    }

    fn get_listener_poller(&self, params: &Params) -> (Poller, Vec<TcpStream>) {
        let mut streams = vec![];

        let poller = Poller::new().unwrap();
        for stream in self.rx.incoming() {
            match stream {
                Ok(x) => {
                    unsafe { poller.add(&x, Event::readable(streams.len())).unwrap() };
                    streams.push(x);
                }
                Err(_) => {
                    // thread::sleep(Duration::from_micros(10));
                }
            }

            // All connections made.
            if streams.len() == params.n {
                break;
            }
        }

        (poller, streams)
    }

    fn get_all_streams(&self, params: &Params) -> Vec<(u128, TcpStream)> {
        (0..params.n as u128)
            // .filter(|x| *x != self.id)
            .map(|x| (x, get_a_stream(&self.ips[&x])))
            .collect()
    }

    fn enter_cs(&self, streams: &mut Vec<(u128, TcpStream)>, poller: &Poller) -> Vec<LogEntry> {
        // Send a request to all the nodes in the quorum
        let c = self.request_cs(streams);
        // println!("Request sent: {c}");

        // wait for the quorum to reply
        let mut replies = 0;
        let mut out = vec![];

        let mut events = Events::new();
        while replies < c {
            events.clear();
            poller.wait(&mut events, None).unwrap();

            for ev in events.iter() {
                let (_, stream) = streams.get_mut(ev.key).unwrap();
                let mut buf = [0; 1024];
                match stream.read(&mut buf) {
                    Ok(ref b) if *b > 0 => {
                        let mut msg = Message::from(&buf[..*b]);
                        msg.flip();
                        match msg.typ {
                            MessageType::Reply => {
                                replies += 1;
                                self.log(&mut out, Action::Reply(msg.id));
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

        // We are in CS now. Set everything to false.
        for x in self.quorum.lock().unwrap().iter_mut() {
            x.1 .0 = false;
        }

        // println!("CS acquired");

        out
    }

    fn exit_cs(&self) {
        self.req_flag.store(false, Ordering::SeqCst);
        let mut q = self.quorum.lock().unwrap();
        for (_, maybe) in q.iter_mut() {
            if let Some(out) = &maybe.1 {
                self.send(out, MessageType::Reply);
                maybe.1 = None;
            }
        }
        // println!("CS exited: {c}");
        // todo!()
    }

    fn terminate(&self, streams: &mut Vec<(u128, TcpStream)>) {
        for (_, stream) in streams.iter() {
            self.send(stream, MessageType::Terminate);
        }
    }

    fn listen(&self, poller: Poller, streams: &Vec<TcpStream>, params: &Params) -> Vec<LogEntry> {
        let mut out = vec![];
        let mut term = 0;
        let mut events = Events::new();

        while term < params.n {
            events.clear();
            poller.wait(&mut events, None).unwrap();
            for ev in events.iter() {
                let mut stream = streams.get(ev.key).unwrap();
                let mut buf = [0; 128];
                match stream.read(&mut buf) {
                    Ok(ref b) if *b > 0 => {
                        let msgs = get_msgs(&mut buf, b);

                        for mut msg in msgs {
                            msg.flip();
                            let id = msg.id.expect_right("");
                            match msg.typ {
                                MessageType::Request => {
                                    self.log(&mut out, Action::Query(msg.id));
                                    // Lamport clock
                                    if msg.ts > self.seq.load(Ordering::SeqCst) as u128 {
                                        self.seq.store((msg.ts + 1) as u64, Ordering::SeqCst);

                                        // Unfulfilled request
                                        self.quorum.lock().unwrap().get_mut(&id).unwrap().1 =
                                            Some(stream.try_clone().unwrap());
                                    } else if self.req_flag.load(Ordering::SeqCst) {
                                        // Request is queued
                                        self.send(stream, MessageType::Reply);
                                        self.quorum.lock().unwrap().get_mut(&id).unwrap().0 = true;
                                    } else {
                                        self.send(stream, MessageType::Reply);
                                        self.quorum.lock().unwrap().get_mut(&id).unwrap().0 = true;
                                    }
                                }
                                MessageType::Reply => {
                                    // println!("Nope.");
                                    dbg!(msg);
                                    // self.log(&mut out, Action::Reply(msg.id));
                                    // if id != self.id {self.quorum.lock().unwrap().get_mut(&msg.id.expect_right("")).unwrap().0 = true;}
                                }
                                MessageType::Terminate => {
                                    term += 1;
                                }
                                _ => {
                                    dbg!(msg);
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

    fn listener_thread(&self, params: Params) -> Vec<LogEntry> {
        let (poller, streams) = self.get_listener_poller(&params);

        self.listen(poller, &streams, &params)
    }

    fn requester_thread(
        &self,
        params: Params,
        mut streams: Vec<(u128, TcpStream)>,
    ) -> Vec<LogEntry> {
        let mut rng = thread_rng();
        let u = Uniform::new(0.0, 1.0);
        let mut out = vec![];

        // All the nodes in the quorum
        // let mut streams = self.get_all_streams(&params);
        let poller = self.get_requester_poller(&streams);

        // Send a request to all the nodes in the quorum
        for _i in 0..params.k {
            self.log(&mut out, Action::Internal);
            params.sleep(u, &mut rng, Region::Out);

            let replies = self.enter_cs(&mut streams, &poller);
            out.extend(replies);

            self.log(&mut out, Action::Acquire);
            params.sleep(u, &mut rng, Region::In);

            self.exit_cs();
            self.log(&mut out, Action::Exit);
        }

        self.terminate(&mut streams);

        out
        // }
    }

    fn listener_spawn(self: Arc<Self>, params: Params) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.listener_thread(params))
    }

    fn requester_spawn(
        self: Arc<Self>,
        params: Params,
        streams: Vec<(u128, TcpStream)>,
    ) -> JoinHandle<Vec<LogEntry>> {
        thread::spawn(move || self.requester_thread(params, streams))
    }

    pub fn spawn(self: Arc<Self>, params: Params) {
        let mut file = File::create(format!("log/rc/node_{}.log", self.id)).unwrap();

        // let init = Instant::now();
        // Spawn a new thread to listen for incoming messages
        let listener = self.clone().listener_spawn(params);

        let streams = self.clone().get_all_streams(&params);

        println!("Connections established.");

        // Spawn a new thread to request CS.
        let node = self.clone().requester_spawn(params, streams);

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
