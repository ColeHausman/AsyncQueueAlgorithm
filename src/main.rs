mod util;

use mpi::traits::*;
use std::error::Error;
use crate::util::process::{Process};
use mpi::datatype::{Equivalence};
use crate::util::constants::NUM_PROCS;
use crate::util::execution_linearizer::{DequeueFixedLinearization, EnqueueFixedLinearization};

fn main() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();

    if world.rank() == 0 {
        print_rectangle(format!("Starting Execution with {} Processes",NUM_PROCS));
    }


    let mut p_i = Process::initialize(&universe);

    let mut e1 = EnqueueFixedLinearization::new(&universe);
    let mut e2 = EnqueueFixedLinearization::new(&universe);
    let mut op1 = DequeueFixedLinearization::new(&universe);
    let mut op2 = DequeueFixedLinearization::new(&universe);

    p_i.enqueue(1, 16);

    e1.enq_invoke(0, 1, &mut p_i);
    e1.enq_req(1, &mut p_i);
    e1.enq_req(2, &mut p_i);

    e1.enq_ack(1, &mut p_i);
    e1.enq_ack(2, &mut p_i);

    e2.enq_invoke(0, 2, &mut p_i);
    e2.enq_req(1, &mut p_i);
    e2.enq_req(2, &mut p_i);

    e2.enq_ack(1, &mut p_i);
    e2.enq_ack(2, &mut p_i);

    println!("{} {:?}", p_i.index, p_i.local_queue);

    op1.deq_invoke(1, &mut p_i);

    op1.deq_req(1, &mut p_i);
    op1.deq_req(2, &mut p_i);

    op1.safe_unsafe_all(1, &mut p_i);

    op1.safe_unsafe_all(2, &mut p_i);

    op2.deq_invoke(2, &mut p_i);
    op2.deq_req(0, &mut p_i);
    op2.deq_req(1, &mut p_i);
    op2.deq_req(2, &mut p_i);

    op1.deq_req(0, &mut p_i);

    op1.safe_unsafe_all(0, &mut p_i);

    op2.safe_unsafe_all(0, &mut p_i);
    op2.safe_unsafe_all(1, &mut p_i);
    op2.safe_unsafe_all(2, &mut p_i);

    let result = p_i.dequeue(0);
    match result {
        Some(value) => println!("Got value: {}", value),
        None => println!("Got value: ⊥")
    }

    let result = p_i.dequeue(2);
    match result {
        Some(value) => println!("Got value: {}", value),
        None => println!("Got value: ⊥")
    }

}

fn print_rectangle(text: String) {
    let text_width = text.len();
    let rectangle_width = text_width + 2;

    // Top border
    println!("┌{}┐", "─".repeat(rectangle_width));

    // Text with side borders
    println!("┊ {} ┊", text);

    // Bottom border
    println!("└{}┘", "─".repeat(rectangle_width));
}