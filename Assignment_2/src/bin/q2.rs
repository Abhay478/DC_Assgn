use assignment_2::{maekawa::MaekawaNode, utils::get_ips, Params};
use std::env;
use std::sync::Arc;

fn main() {
    let params = Params::new();
    let ips = get_ips();
    let id = (
        env::args().nth(1).unwrap().parse().unwrap(),
        env::args().nth(2).unwrap().parse().unwrap(),
    );
    let node = Arc::new(MaekawaNode::new(id, ips));
    node.spawn(params);
}
