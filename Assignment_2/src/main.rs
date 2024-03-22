use assignment_2::*;
use std::env;
use std::sync::Arc;

fn main() {
    let params = Params::new();
    dbg!(&params);
    let ips = get_ips();
    let id = (
        env::args().nth(1).unwrap().parse::<usize>().unwrap(),
        env::args().nth(2).unwrap().parse::<usize>().unwrap(),
    );
    let node = Arc::new(MaekawaNode::new(id, ips));
    node.listen(params);
}
