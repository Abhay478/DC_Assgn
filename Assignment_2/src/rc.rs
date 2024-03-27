use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener},
    sync::atomic::AtomicU64,
    time::Instant,
};

pub struct RCNode {
    id: (u64, u64), // grid coordinates
    ips: HashMap<(u64, u64), SocketAddr>,
    rx: TcpListener, // Quorum listener
    init: Instant,
    seq: AtomicU64,
}
