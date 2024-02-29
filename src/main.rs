mod util;

use std::collections::VecDeque;
use mpi::traits::*;
use std::error::Error;
use crate::util::constants::{DEQ_INVOKE, DEQ_REQ, ENQ_ACK, ENQ_INVOKE, ENQ_REQ, NUM_PROCS, SAFE_UNSAFE};
use crate::util::message_structs::{QueueOpReq, VectorClock};
use crate::util::process::{Process};
use mpi::datatype::{Equivalence};
use crate::util::execution_linearizer::{DequeueFixedLinearization, EnqueueFixedLinearization};

fn main() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();

    let mut p_i = Process::initialize(&world);

    let mut e1 = EnqueueFixedLinearization::new(&universe);
    let mut e2 = EnqueueFixedLinearization::new(&universe);
    let mut op1 = DequeueFixedLinearization::new(&universe);
    let mut op2 = DequeueFixedLinearization::new(&universe);
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
}