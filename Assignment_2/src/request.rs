use std::net::TcpStream;

pub struct Request<'a> {
    pub ts: u128,
    pub pid: (u64, u64),
    pub stream: &'a TcpStream,
}

impl<'a> Request<'a> {
    pub fn new(ts: u128, pid: (u64, u64), stream: &'a TcpStream) -> Self {
        Self { ts, pid, stream }
    }
}

impl<'a> PartialEq for Request<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.ts == other.ts && self.pid == other.pid
    }
}

impl<'a> PartialOrd for Request<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // We want a minheap, coz least timestamp gets priority
        other.ts.partial_cmp(&self.ts)
    }
}

impl<'a> Eq for Request<'a> {}

impl<'a> Ord for Request<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.ts != other.ts {
            other.ts.cmp(&self.ts)
        } else {
            self.pid.cmp(&other.pid)
        }
    }
}
