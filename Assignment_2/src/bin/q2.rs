use assignment_2::rc::RCNode;
use assignment_2::{utils::get_ips, Params};
use std::env;
use std::sync::Arc;

fn main() {
    let params = Params::new();
    let (_, ips) = get_ips();
    let id = env::args().nth(1).unwrap().parse().unwrap();
    let node = Arc::new(RCNode::new(id, ips));
    node.spawn(params);
}
