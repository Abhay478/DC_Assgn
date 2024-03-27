use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    mem,
    net::{SocketAddr, TcpStream},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Internal,

    Query((u64, u64)),
    Request((u64, u64)),

    Grant((u64, u64)),
    Reply((u64, u64)),

    Acquire,

    Release((u64, u64)),
    Exit,

    Terminate,
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Request,
    Reply,
    Release,
    Failed,
    Inquire,
    Yield,
    Terminate,
}
#[derive(Debug, Clone)]
pub struct Message {
    pub id: (u64, u64),
    pub typ: MessageType,
    pub ts: u128,
}

impl Message {
    pub fn new(id: (u64, u64), typ: MessageType, ts: u128) -> Self {
        Self { id, typ, ts }
    }
}

impl Into<Vec<u8>> for Message {
    fn into(self) -> Vec<u8> {
        let mut out = vec![];
        let id: [u8; 16] = unsafe { mem::transmute_copy(&self.id) };
        let ts: [u8; 16] = unsafe { mem::transmute_copy(&self.ts) };
        let typ: u8 = match self.typ {
            MessageType::Request => 1,
            MessageType::Reply => 2,
            MessageType::Release => 3,
            MessageType::Failed => 4,
            MessageType::Inquire => 5,
            MessageType::Yield => 6,
            MessageType::Terminate => 7,
        };
        out.extend(id);
        out.push(typ);
        out.extend(ts);
        out
    }
}

impl From<&[u8]> for Message {
    fn from(x: &[u8]) -> Self {
        let mut id = [0; 16];
        let mut ts = [0; 16];
        id.copy_from_slice(&x[..16]);
        ts.copy_from_slice(&x[17..33]);
        let id: (u64, u64) = unsafe { mem::transmute_copy(&id) };
        let ts: u128 = unsafe { mem::transmute_copy(&ts) };
        let typ = match x[16] {
            1 => MessageType::Request,
            2 => MessageType::Reply,
            3 => MessageType::Release,
            4 => MessageType::Failed,
            5 => MessageType::Inquire,
            6 => MessageType::Yield,
            7 => MessageType::Terminate,
            _ => unreachable!(),
        };
        Self { id, typ, ts }
    }
}

/// Logging unit
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub pid: (u64, u64),
    pub ts: Instant,
    pub act: Action,
}

pub fn get_ips() -> HashMap<(u64, u64), SocketAddr> {
    // Read all ip addresses from a file
    let mut file = File::open("ips.txt").unwrap();
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    let mut out = HashMap::new();
    for l in buf.lines() {
        let mut it = l.split_whitespace();
        let id = (
            it.next().unwrap().parse().unwrap(),
            it.next().unwrap().parse().unwrap(),
        );
        let ip = it.next().unwrap();
        let port = it.next().unwrap();
        let addr = format!("{}:{}", ip, port).parse::<SocketAddr>().unwrap();
        out.insert(id, addr);
    }
    out
}

pub fn get_a_stream(addr: &SocketAddr) -> TcpStream {
    loop {
        match TcpStream::connect(addr) {
            Ok(x) => {
                println!("Connected to {}", addr);
                break x;
            }
            Err(_) => {
                thread::sleep(Duration::from_micros(100));
            }
        }
    }
}

pub fn get_msgs(buf: &[u8], b: &usize) -> Vec<Message> {
    let msgsz = 33;
    let msgc = *b / msgsz;
    (0..msgc)
        .map(|i| Message::from(&buf[i * msgsz..(i + 1) * msgsz]))
        .collect::<Vec<Message>>()
}
