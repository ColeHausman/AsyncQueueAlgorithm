use std::cmp::Ordering;
use std::collections::VecDeque;
use std::{fmt, io};
use std::time::Duration;
use mpi::environment::Universe;
use mpi::Rank;
use mpi::topology::SimpleCommunicator;
use mpi::traits::*;
use mpi::request::WaitGuard;
use chrono::Local;
use crate::util::compare_ts::{ compare_ts, compare_ts_ord, ComparisonResult, contains_all_zeros };
use crate::util::message_structs::{ DeqReq, EnqReq, SafeUnsafeAck, VectorClock };
use crate::util::message_structs::{ QueueOpReq, OpNextAction };
use crate::util::confirmation_list::{ ConfirmationList,propagate_earlier_responses };
use crate::util::confirmation_list::{ update_unsafes, print_confirmation_lists };
use crate::util::constants::{ NUM_PROCS, ENQ_REQ, DEQ_REQ, ENQ_ACK };
use crate::util::constants::{ UNSAFE, SAFE, ENQ_INVOKE, DEQ_INVOKE };
use crate::util::update_ts::update_ts;


const PLACEHOLDER: u16 = 0xFFFC;

#[derive(Debug)]
struct OutOfRangeError;

pub enum HandleDequeue {
    Success((Rank, u16)),
    NoResult,
}

impl fmt::Display for OutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process out of range world size!")
    }
}
impl std::error::Error for OutOfRangeError {}

#[derive(Clone)]
pub struct Process<'universe> {
    pub(crate) index: Rank, // stores process index
    pub(crate) vector_clock: VectorClock, // stores process vector clock
    pub(crate) lists: Vec<ConfirmationList>, // stores confirmation lists
    pub(crate) enq_count: u16, // stores number of enqueues
    pub(crate) local_queue:VecDeque<(Rank, u16, VectorClock)>, // stores a local copy of the queue sorted by ts
    formatted_strings:Vec<String>, // for debugging
    message_buffer: [OpNextAction; NUM_PROCS], // buffer to hold up to NUM_PROCS incoming messages
    universe: &'universe Universe,
}

impl<'universe> Process<'universe> {
    pub(crate) fn initialize(universe: &'universe Universe) -> Self {
        let enq_count = 0u16; // initialize enq_count to 0

        Self {
            index: universe.world().rank(), // get process ID from MPI world
            vector_clock: VectorClock([0; NUM_PROCS]),
            lists: Vec::new(), // holds confirmation lists
            enq_count, // counts num enqueues
            local_queue: VecDeque::new(), // initialize empty local queue
            formatted_strings: Vec::new(),
            message_buffer: Default::default(),
            universe
        }
    }

    pub(crate) fn enqueue_local(&mut self, item: (Rank, u16, VectorClock)) {
        let ts_to_insert = &item.2.0; // 3rd element of the tuple, 1st field of VC

        // Use binary_search_by with the custom comparator
        match self.local_queue.binary_search_by(|&(_, _, ref ts)| compare_ts_ord(&ts.0, ts_to_insert)) {
            Ok(insert_position) => {
                // Insert at the correct position
                self.local_queue.insert(insert_position, item);
            }
            Err(insert_position) => {
                // If binary_search returns Err, insert at the calculated position
                self.local_queue.insert(insert_position, item);
            }
        }
    }

    pub(crate) fn add_confirmation_list(&mut self, confirmation_list: ConfirmationList) {
        let ts_to_insert = &confirmation_list.ts;

        // Use binary_search_by with the custom comparator
        match self.lists.binary_search_by(|cl| compare_ts_ord(&cl.ts, ts_to_insert)) {
            Ok(insert_position) => {
                // Insert at the correct position
                self.lists.insert(insert_position, confirmation_list);
            }
            Err(insert_position) => {
                // If binary_search returns Err, insert at the calculated position
                self.lists.insert(insert_position, confirmation_list);
            }
        }
    }

    pub(crate) fn remove_confirmation_list(&mut self, ts: &VectorClock) {
        for (index, confirmation_list) in self.lists.iter().enumerate() {
            if compare_ts_ord(&ts.0, &confirmation_list.ts) == Ordering::Equal {
                self.lists.remove(index);
                break; // Shouldnt have duplicate ts
            }
        }
    }

    pub(crate) fn print_execution(&mut self) {
        let combined_string = self.formatted_strings.join("\n");
        let execution_output = format!("========== Execution for process {} ===========\n", self.index) + &*combined_string + "================================\n";
        println!("{}", execution_output);
    }

    pub(crate) fn handle_queue_op(&mut self, op: QueueOpReq) -> OpNextAction{
        let mut res: OpNextAction = OpNextAction::default();
        match op.message {
            ENQ_INVOKE => {
                self.enq_count = 0;
                self.vector_clock.0[self.index as usize] += 1;
                self.enq_count += 1;
                println!("{} enquing at ts {:?}", self.index, self.vector_clock.0);
                res = OpNextAction{
                    message: ENQ_REQ, value: Option::from(op.value),
                    invoker: self.index, ts: self.vector_clock
                }
            }
            ENQ_REQ => {
                update_ts(&mut self.vector_clock.0, &op.timestamp.0);
                self.enqueue_local((op.sender, op.value, op.timestamp));

                for confirmationList in &mut self.lists {
                    match compare_ts(&confirmationList.ts, &self.vector_clock.0) {
                        ComparisonResult::Less | ComparisonResult::StrictlyLess // accept less or strictly less
                        => confirmationList.response_list[self.index as usize] = 1,
                        _ => {}
                    }
                }
                res = OpNextAction{
                    message: ENQ_ACK, value: Option::from(op.value),
                    invoker: op.sender, ts: op.timestamp
                }
            }
            ENQ_ACK => {
                println!("{} got enq ack", self.index);
                self.enq_count += 1;
                if self.enq_count == NUM_PROCS as u16 {
                    self.enqueue_local((self.index, op.value, op.timestamp));
                    println!("Process {} finished enqueue", self.index);
                }
                res = OpNextAction{
                    message: 9, value: Option::from(op.value),
                    invoker: self.index, ts: op.timestamp
                }
            }
            DEQ_INVOKE => {
                self.vector_clock.0[self.index as usize] += 1;
                println!("Process {} DEQ at ts {:?}", self.index, self.vector_clock);
                res = OpNextAction{
                    message: DEQ_REQ, value: Option::from(op.value),
                    invoker: self.index, ts: self.vector_clock
                }
            }
            DEQ_REQ => {
                println!("Process {} recv deq_req with ts: {:?} self: {:?}",
                         self.index, op.timestamp.0, self.vector_clock.0);
                update_ts(&mut self.vector_clock.0, &op.timestamp.0);
                match compare_ts_ord(&op.timestamp.0, &self.vector_clock.0) {
                    Ordering::Less  if !contains_all_zeros(&op.timestamp.0)=> {
                        // unsafe
                        res = OpNextAction{
                            message: UNSAFE, value: Option::from(op.value),
                            invoker: op.sender, ts: op.timestamp
                        }
                    }
                    _ => {  // safe
                        res = OpNextAction{
                            message: SAFE, value: Option::from(op.value),
                            invoker: op.sender, ts: op.timestamp
                        }
                    }
                }
            }
            SAFE | UNSAFE => {
                if op.message == UNSAFE {
                    println!("Process {} recv UNSAFE from {} at ts {:?}",
                             self.index, op.sender, op.timestamp.0);
                }else{
                    println!("Process {} recv SAFE from {} at ts {:?}",
                             self.index, op.sender, op.timestamp.0);
                }

                let mut contains_req = false;
                for confirmation_list in &mut self.lists.iter() {
                    match compare_ts_ord(&confirmation_list.ts, &op.timestamp.0) {
                        Ordering::Equal => {
                            contains_req = true;
                        }
                        _ => {}
                    }
                }
                if !contains_req { // we dont have this ts in our confirmation list
                    self.add_confirmation_list(ConfirmationList::new(op.timestamp.0))
                }

                for mut confirmation_list in self.lists.iter_mut() {
                    match compare_ts_ord(&confirmation_list.ts, &op.timestamp.0) {
                        Ordering::Equal => {
                            confirmation_list.response_list[op.sender as usize] =
                                if op.message == UNSAFE {2} else {1};
                        }
                        _ => {}
                    }
                }

                propagate_earlier_responses(&mut self.lists);
                let ret_message:u16 = if op.message == UNSAFE {UNSAFE} else {SAFE};

                for (i, confirmation_list) in self.lists.iter_mut().enumerate() {
                    if !confirmation_list.response_list.contains(&0) && !confirmation_list.handled {
                        let mut pos: usize = 0;
                        for response in confirmation_list.response_list.iter() {
                            if *response == 2 {
                                pos += 1;
                            }
                        }
                        confirmation_list.handled = true;
                        let deq_val: Option<_> = self.local_queue
                            .remove(pos).map(|(_, val,_)| val);
                        match deq_val {
                            Some(val) => println!("Process{} got {}", self.index, val),
                            None => println!("Process{} got ⊥", self.index),
                        }
                        update_unsafes(&mut self.lists, i+1);
                        return OpNextAction{
                            message: ret_message,
                            value: deq_val,
                            invoker: self.index,
                            ts: self.vector_clock
                        };
                    }

                }

                return OpNextAction{
                    message: ret_message,
                    value: Option::from(op.value),
                    invoker: self.index,
                    ts: self.vector_clock
                };

            }
            _ => {
                res = OpNextAction::default();
            }
        }
        res
    }

    pub(crate) fn sync_send_receive(&mut self, universe: &Universe, op: QueueOpReq) -> OpNextAction {
        if (op.receiver == op.sender)  && self.index == op.sender {
            return self.handle_queue_op(op); // dont need to use MPI to send to self
        } else if op.receiver == op.sender { // do nothing
            return OpNextAction::default();
        }

        let mut recv_op = QueueOpReq::default();

        let world = universe.world();
        mpi::request::scope(|scope| {
            if self.index == op.sender {
                let mut sreq = world.process_at_rank(op.receiver)
                    .immediate_send(scope, &op);
                loop {
                    match sreq.test() {
                        Ok(_) => break,
                        Err(req) => sreq = req,
                    }
                }
            } else if world.rank() == op.receiver {
                let rreq = WaitGuard::from(world.process_at_rank(op.sender)
                    .immediate_receive_into(scope, &mut recv_op));
                drop(rreq);
            }
        });

        world.barrier(); // All processes reach the barrier
        if self.index == op.receiver {
            return self.handle_queue_op(recv_op);
        }

        OpNextAction::default()
    }

    pub(crate) fn async_send_receive(&mut self, universe: &Universe, op: QueueOpReq) -> OpNextAction {
        if (op.receiver == op.sender)  && self.index == op.sender {
            return self.handle_queue_op(op); // dont need to use MPI to send to self
        } else if op.receiver == op.sender { // do nothing
            return OpNextAction::default();
        }

        let mut recv_op = QueueOpReq::default();

        let world = universe.world();
        mpi::request::scope(|scope| {
            if self.index == op.sender {
                let mut sreq = world.process_at_rank(op.receiver)
                    .immediate_send(scope, &op);
                loop {
                    match sreq.test() {
                        Ok(_) => break,
                        Err(req) => sreq = req,
                    }
                }
            } else if world.rank() == op.receiver {
                let rreq = WaitGuard::from(world.process_at_rank(op.sender)
                    .immediate_receive_into(scope, &mut recv_op));
                drop(rreq);
            }
        });

        if self.index == op.receiver {
            return self.handle_queue_op(recv_op);
        }

        OpNextAction::default()
    }

    pub(crate) fn enqueue(&mut self, invoking: Rank, val: u16) {
        let mut enq_ts = Default::default();
        if self.index == invoking {
            enq_ts = self.async_send_receive(self.universe, QueueOpReq{
                message: ENQ_INVOKE,
                value: val,
                sender: invoking,
                receiver: invoking,
                timestamp: self.vector_clock,
            }).ts;
        }

        for i in 0..NUM_PROCS {
            if i != invoking as usize {
                if self.index as usize == i { // receiver
                    enq_ts = self.async_send_receive(self.universe, QueueOpReq{
                        message: ENQ_REQ,
                        value: val,
                        sender: invoking,
                        receiver: i as Rank,
                        timestamp: enq_ts,
                    }).ts;
                }else {
                    self.async_send_receive(self.universe, QueueOpReq{
                        message: ENQ_REQ,
                        value: val,
                        sender: invoking,
                        receiver: i as Rank,
                        timestamp: enq_ts,
                    });
                }
            }
        }

        for i in 0..NUM_PROCS {
            if i != invoking as usize {
                self.async_send_receive(self.universe, QueueOpReq{
                    message: ENQ_ACK,
                    value: val,
                    sender: i as Rank,
                    receiver: invoking,
                    timestamp: enq_ts,
                });
            }
        }
    }

    pub(crate) fn dequeue(&mut self, invoking: Rank) -> Option<u16> {
        let mut message_buffer: u16 = 0;
        let mut deq_ts = self.async_send_receive(self.universe, QueueOpReq{
            message: DEQ_INVOKE,
            value: 0,
            sender: invoking,
            receiver: invoking,
            timestamp: self.vector_clock,
        }).ts;


        for i in 0..NUM_PROCS {
            if self.index as usize == i {
                 let res = self.async_send_receive(self.universe, QueueOpReq{
                    message: DEQ_REQ,
                    value: 0,
                    sender: invoking,
                    receiver: i as Rank,
                    timestamp: deq_ts,
                });
                deq_ts = res.ts;
                message_buffer = res.message;

            } else {
                self.async_send_receive(self.universe, QueueOpReq{
                    message: DEQ_REQ,
                    value: 0,
                    sender: invoking,
                    receiver: i as Rank,
                    timestamp: deq_ts,
                });
            }
        }

        let mut ret_val = OpNextAction::default();

        for i in 0..NUM_PROCS {
            for j in 0..NUM_PROCS {
                let res = self.async_send_receive(self.universe, QueueOpReq{
                    message: message_buffer,
                    value: 0,
                    sender: i as Rank,
                    receiver: j as Rank,
                    timestamp: deq_ts,
                });

                match res.value{
                    Some(val) => ret_val = res,
                    _=>{}
                }
            }
        }

        return ret_val.value;
    }
}

impl<'universe> fmt::Debug for Process<'universe> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ Process: {}, local_queue: {:?}, ts: {:?}, enq_count: {} }}",
            self.index, self.local_queue, self.vector_clock, self.enq_count
        )
    }
}