mod util;

use std::collections::VecDeque;
use mpi::traits::*;
use std::error::Error;
use crate::util::constants::{ENQ_ACK, ENQ_INVOKE, ENQ_REQ, NUM_PROCS};
use crate::util::message_structs::{QueueOpReq, VectorClock, CompletionSignal, MyArc};
use std::sync::{Arc, Mutex, Condvar};
use crate::util::process::{OpNextAction, Process};
use mpi::{Address, Count, Rank};
use mpi::datatype::{Equivalence, UserDatatype};
use mpi::environment::Universe;


fn main() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let rank = world.rank();

    let mut p_i = Process::initialize(&world);

    //p_i.do_enq(&universe, 0, 3);

    let mut queue: VecDeque<QueueOpReq> = VecDeque::new();
    /*
    queue.push_back(QueueOpReq{message: ENQ_INVOKE, value: 3, sender: 0, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 0, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 0, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 2, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_INVOKE, value: 6, sender: 1, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 1, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 1, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 1, receiver: 3, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 0, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 2, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 3, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 1, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 0, receiver: 3, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 3, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});


     */



    queue.push_back(QueueOpReq{message: ENQ_INVOKE, value: 3, sender: 0, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 0, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 0, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 2, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_INVOKE, value: 6, sender: 1, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 1, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 1, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 1, receiver: 3, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_INVOKE, value: 9, sender: 2, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 0, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 2, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 3, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 2, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 2, receiver: 1, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 2, receiver: 3, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 1, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_REQ, value: 0, sender: 0, receiver: 3, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 3, receiver: 0, timestamp: VectorClock([0;NUM_PROCS])});

    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 0, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 1, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});
    queue.push_back(QueueOpReq{message: ENQ_ACK, value: 0, sender: 3, receiver: 2, timestamp: VectorClock([0;NUM_PROCS])});



    p_i.do_enq_with_q(&universe, &mut queue);

    //p_i.testing(&universe);


    /*
    p_i.Enqueue(&universe, 1, 12);
    p_i.Enqueue(&universe, 1, 9);
    p_i.Enqueue(&universe, 0, 3);
     */

    /*
    if rank == 0 {
        let x = QueueOpReq{message: 5, value: 1, rank: rank, timestamp: VectorClock([0;NUM_PROCS])};
        p_i.handle_queue_op(x);
    }

    let enq_req ;
    if rank == 0 {
        enq_req = QueueOpReq{message: 0, value: 0, rank: 0, timestamp: p_i.vector_clock};
    }else{
        enq_req = QueueOpReq{message: 0, value: 0, rank: 0, timestamp: VectorClock([0;NUM_PROCS])};
    }

    p_i.async_send_receive(&universe, 0, 1, enq_req);
    p_i.async_send_receive(&universe, 0, 2, enq_req);
    p_i.async_send_receive(&universe, 0, 3, enq_req);

    let enq_ack;
    if rank != 0 {
        enq_ack = QueueOpReq{message: 2, value: 0, rank: rank, timestamp: p_i.vector_clock};
    } else {
        enq_ack= QueueOpReq{message: 0, value: 0, rank: 0, timestamp: VectorClock([0;NUM_PROCS])};
    }

    p_i.async_send_receive(&universe, 1, 0, enq_ack);
    p_i.async_send_receive(&universe, 2, 0, enq_ack);
    p_i.async_send_receive(&universe, 3, 0, enq_ack);
    */

    // TODO look here

    /*
    if rank == 0 {
        let x = QueueOpReq{message: ENQ_INVOKE, value: 1, rank: rank, timestamp: VectorClock([0;NUM_PROCS])};
        action = p_i.handle_queue_op(x);
    }else{
        action = OpNextAction{message: 0, send_to_all: false, process_to_send: 0, ts: VectorClock([0;NUM_PROCS])}
    }

    println!("{}: {:?}", rank, action);

    p_i.execute_async_send_receive(&universe, 0, 1, QueueOpReq {
        message: action.message,
        value: 0,
        rank: action.process_to_send,
        timestamp: action.ts,
    });

    p_i.execute_async_send_receive(&universe, 0, 2, QueueOpReq {
        message: action.message,
        value: 0,
        rank: action.process_to_send,
        timestamp: action.ts,
    });

    action = p_i.execute_async_send_receive(&universe, 0, 3, QueueOpReq {
        message: action.message,
        value: 0,
        rank: action.process_to_send,
        timestamp: action.ts,
    });

    println!("{}: {:?}", rank, action);

    println!("{}: {}", rank, action.process_to_send);
    */



    /*

    // Execute the first async_send_receive function
    p_i.execute_async_send_receive(&universe, 0, 3, QueueOpReq { message: 42, value: 10, rank: 0, timestamp: VectorClock([0; NUM_PROCS]) });


    // Execute the second async_send_receive function
    p_i.execute_async_send_receive(&universe, 1, 2, QueueOpReq { message: 24, value: 20, rank: 1, timestamp: VectorClock([0; NUM_PROCS]) });

    // Execute the third async_send_receive function
    p_i.execute_async_send_receive(&universe, 2, 1, QueueOpReq { message: 36, value: 30, rank: 2, timestamp: VectorClock([0; NUM_PROCS]) });



     */

// Create a single completion signal
   // let completion_signal = Arc::new(CompletionSignal::new());


    /*
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
    */
}