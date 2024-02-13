mod util;

use mpi::traits::*;
use std::error::Error;
use crate::util::process::Process;

fn main() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let rank = world.rank();

    let mut p_i = Process::initialize(&world);

    p_i.Enqueue(&universe, 1, 12);
    p_i.Enqueue(&universe, 1, 9);
    p_i.Enqueue(&universe, 0, 3);


    if let Some(value) = p_i.Dequeue(&universe, 0) {
        println!("Process {} got {:?}", rank, value);
    }

    if let Some(value) = p_i.Dequeue(&universe, 0) {
        println!("Process {} got {:?}", rank, value);
    }

    if let Some(value) = p_i.Dequeue(&universe, 3) {
        println!("Process {} got {:?}", rank, value);
    }

    p_i.print_execution();
}