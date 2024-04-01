use assignment_2::{maekawa::MaekawaNode, utils::get_ips, Params};
use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

fn main() {
    let params = Params::new();
    let (ips, _) = get_ips();
    let id = (
        env::args().nth(1).unwrap().parse().unwrap(),
        env::args().nth(2).unwrap().parse().unwrap(),
    );
    let node = Arc::new(MaekawaNode::new(id, ips));
    let mut f = File::create(format!("log/maekawa/out_{}_{}.log", id.0, id.1)).unwrap();
    println!("Node {:?} spawned", id);
    node.clone().spawn(params);
    println!("Node {:?} terminated.", id);
    let mc = node.as_ref().mc.load(std::sync::atomic::Ordering::SeqCst);
    let elap = node.as_ref().init.elapsed().as_millis();
    write!(f, "{} {}", mc, elap).unwrap();
}
