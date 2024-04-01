use assignment_2::rc::RCNode;
use assignment_2::{utils::get_ips, Params};
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

fn main() {
    let params = Params::new();
    let (_, ips) = get_ips();
    let id = env::args().nth(1).unwrap().parse().unwrap();
    let node = Arc::new(RCNode::new(id, ips));
    node.clone().spawn(params);
    let mc = node.as_ref().mc.load(std::sync::atomic::Ordering::SeqCst);
    let elap = node.as_ref().init.elapsed().as_millis();
    let mut f = File::create(format!("log/rc/out_{}.log", id)).unwrap();
    write!(f, "{} {}", mc, elap).unwrap();

    // Ok((mc, elap))
}
