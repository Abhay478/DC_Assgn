use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    io::Read,
    mem,
    net::{SocketAddr, TcpStream},
};

use either::Either::{self, Left, Right};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Internal,

    Query(Either<(u64, u64), u128>),
    Request(Either<(u64, u64), u128>),

    Grant(Either<(u64, u64), u128>),
    Reply(Either<(u64, u64), u128>),

    Acquire,

    Release(Either<(u64, u64), u128>),
    Exit,

    Terminate,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Internal => write!(f, "executed an internal action"),

            Action::Query(Left(x)) => write!(f, "received request from Process {:?}", x),
            Action::Query(Right(x)) => write!(f, "received request from Process {}", x),
            Action::Request(Left(x)) => write!(f, "sent request to process {:?}", x),
            Action::Request(Right(x)) => write!(f, "sent request to process {}", x),

            Action::Grant(Left(x)) => write!(f, "sent reply to Process {:?}", x),
            Action::Grant(Right(x)) => write!(f, "sent reply to Process {}", x),
            Action::Reply(Left(x)) => write!(f, "received reply from Process {:?}", x),
            Action::Reply(Right(x)) => write!(f, "received reply from Process {}", x),

            Action::Acquire => write!(f, "acquired the CS"),

            Action::Release(Left(x)) => write!(f, "received release from process {:?}", x),
            Action::Release(Right(x)) => write!(f, "received release from process {}", x),

            Action::Exit => write!(f, "exited the critical section"),

            Action::Terminate => write!(f, "terminated"),
        }
    }
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
    pub id: Either<(u64, u64), u128>,
    pub typ: MessageType,
    pub ts: u128,
}

impl Message {
    pub fn new_maekawa(id: (u64, u64), typ: MessageType, ts: u128) -> Self {
        Self {
            id: Left(id),
            typ,
            ts,
        }
    }

    pub fn new_rc(id: u128, typ: MessageType, ts: u128) -> Self {
        Self {
            id: Right(id),
            typ,
            ts,
        }
    }

    pub fn flip(&mut self) {
        self.id = match self.id {
            Left(x) => Right(unsafe { mem::transmute(x) }),
            Right(x) => Left(unsafe { mem::transmute(x) }),
        };
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
            _ => unreachable!("{}", x[16]),
        };
        Self {
            id: Left(id),
            typ,
            ts,
        }
    }
}

/// Logging unit
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub pid: Either<(u64, u64), u128>,
    pub ts: u128,
    pub act: Action,
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pid = match self.pid {
            Left(x) => format!("{:?}", x),
            Right(x) => format!("{:?}", x),
        };
        write!(f, "Process {:?} {} at time {:?}", pid, self.act, self.ts,)
    }
}

pub fn get_ips() -> (HashMap<(u64, u64), SocketAddr>, HashMap<u128, SocketAddr>) {
    // Read all ip addresses from a file
    let mut file = File::open("ips.txt").unwrap();
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    let mut out = (HashMap::new(), HashMap::new());
    for (i, l) in buf.lines().enumerate() {
        let mut it = l.split_whitespace();
        let id = (
            it.next().unwrap().parse().unwrap(),
            it.next().unwrap().parse().unwrap(),
        );
        let ip = it.next().unwrap();
        let port = it.next().unwrap();
        let addr = format!("{}:{}", ip, port).parse::<SocketAddr>().unwrap();
        out.0.insert(id, addr);
        out.1.insert(i as u128, addr);
    }
    out
}

pub fn get_a_stream(addr: &SocketAddr) -> TcpStream {
    loop {
        match TcpStream::connect(addr) {
            Ok(x) => {
                // println!("Connected to {}", addr);
                break x;
            }
            Err(_) => {
                // thread::sleep(Duration::from_micros(100));
            }
        }
    }
}

pub fn get_msgs(buf: &mut [u8], b: &usize) -> Vec<Message> {
    let msgsz = 33;
    let msgc = *b / msgsz;
    let out = (0..msgc)
        .map(|i| Message::from(&buf[i * msgsz..(i + 1) * msgsz]))
        .collect::<Vec<Message>>();
    buf.fill(0);
    dbg!(&out);
    out
}
