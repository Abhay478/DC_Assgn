#![allow(dead_code)]

use rand::{
    distributions::{Distribution, Uniform},
    rngs::ThreadRng,
};

use std::{fs::File, io::Read, thread, time::Duration};

#[derive(Debug, Clone, Copy)]
pub struct Params {
    n: usize,
    k: usize,
    out_l: f64,
    in_l: f64,
}

impl Params {
    pub fn new() -> Self {
        let mut file = File::open("inp-params.txt").unwrap();
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let q = buf
            .split_whitespace()
            .map(|x| x.parse::<f64>().unwrap())
            .collect::<Vec<f64>>();

        Self {
            n: q[0] as usize,
            k: q[1] as usize,
            out_l: q[2],
            in_l: q[3],
        }
    }

    fn sleep(&self, u: Uniform<f64>, rng: &mut ThreadRng, which: Region) {
        let ts = -(u.sample(rng) as f64).ln()
            * match which {
                Region::Out => self.out_l,
                Region::In => self.in_l,
            } as f64;
        thread::sleep(Duration::from_micros(ts as u64));
    }
}

pub enum Region {
    Out,
    In,
}

pub mod maekawa;
pub mod rc;
pub mod request;
pub mod utils;
