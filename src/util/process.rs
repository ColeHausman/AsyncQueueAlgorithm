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
use crate::util::constants::{ UNSAFE, SAFE, ENQ_INVOKE, DEQ_INVOKE, SAFE_UNSAFE };
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
pub struct Process {
    pub(crate) index: Rank, // stores process index
    pub(crate) vector_clock: VectorClock, // stores process vector clock
    pub(crate) lists: Vec<ConfirmationList>, // stores confirmation lists
    pub(crate) enq_count: u16, // stores number of enqueues
    pub(crate) local_queue:VecDeque<(Rank, u16, VectorClock)>, // stores a local copy of the queue sorted by ts
    formatted_strings:Vec<String>, // for debugging
    message_buffer: [OpNextAction; NUM_PROCS], // buffer to hold up to NUM_PROCS incoming messages
}

impl Process {
    pub(crate) fn initialize(world: &SimpleCommunicator) -> Self {
        let enq_count = 0u16; // initialize enq_count to 0

        Self {
            index: world.rank(), // get process ID from MPI world
            vector_clock: VectorClock([0; NUM_PROCS]),
            lists: Vec::new(), // holds confirmation lists
            enq_count, // counts num enqueues
            local_queue: VecDeque::new(), // initialize empty local queue
            formatted_strings: Vec::new(),
            message_buffer: Default::default()
        }
    }

    pub(crate) fn enqueue_local(&mut self, item: (Rank, u16, VectorClock)) {
        let ts_to_insert = &item.2.0;

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
                res = OpNextAction{message: ENQ_REQ, value: op.value, invoker: self.index, ts: self.vector_clock}
            }
            ENQ_REQ => {
                println!("{} got enq req {:?}", self.index, op);
                update_ts(&mut self.vector_clock.0, &op.timestamp.0);
                self.enqueue_local((op.sender, op.value, op.timestamp));

                for confirmationList in &mut self.lists {
                    match compare_ts(&confirmationList.ts, &self.vector_clock.0) {
                        ComparisonResult::Less | ComparisonResult::StrictlyLess // accept less or strictly less
                        => confirmationList.response_list[self.index as usize] = 1,
                        _ => {}
                    }
                }
                res = OpNextAction{message: ENQ_ACK, value: op.value, invoker: op.sender, ts: op.timestamp}
            }
            ENQ_ACK => {
                println!("{} got enq ack", self.index);
                self.enq_count += 1;
                if self.enq_count == NUM_PROCS as u16 {
                    self.enqueue_local((self.index, op.value, op.timestamp));
                    println!("Process {} finished enqueue", self.index);
                }
                res = OpNextAction{message: 9, value: op.value, invoker: self.index, ts: op.timestamp}
            }
            DEQ_INVOKE => {
                self.vector_clock.0[self.index as usize] += 1;
                println!("Process {} DEQ at ts {:?}", self.index, self.vector_clock);
                res = OpNextAction{message: DEQ_REQ, value: op.value, invoker: self.index, ts: self.vector_clock}
            }
            DEQ_REQ => {
                println!("Process {} recv deq_req with ts: {:?} self: {:?}", self.index, op.timestamp.0, self.vector_clock.0);
                update_ts(&mut self.vector_clock.0, &op.timestamp.0);
                match compare_ts_ord(&op.timestamp.0, &self.vector_clock.0) {
                    Ordering::Less  if !contains_all_zeros(&op.timestamp.0)=> {
                        // unsafe
                        res = OpNextAction{message: UNSAFE, value: op.value, invoker: op.sender, ts: op.timestamp}
                    }
                    _ => {  // safe
                        res = OpNextAction{message: SAFE, value: op.value, invoker: op.sender, ts: op.timestamp}
                    }
                }
            }
            SAFE | UNSAFE => {
                if op.message == UNSAFE {
                    println!("Process {} recv UNSAFE from {} at ts {:?}", self.index, op.sender, op.timestamp.0);
                }else{
                    println!("Process {} recv SAFE from {} at ts {:?}", self.index, op.sender, op.timestamp.0);
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
                        // TODO this deq_val needs to be option type to allow bottom
                        let deq_val = self.local_queue.remove(pos).unwrap().1;
                        println!("Process{} got {:?}", self.index, deq_val);
                        update_unsafes(&mut self.lists, i+1);
                        return OpNextAction{message: ret_message, value: deq_val, invoker: self.index, ts: self.vector_clock};
                    }

                }

                return OpNextAction{message: ret_message, value: op.value, invoker: self.index, ts: self.vector_clock};

            }
            _ => {
                res = OpNextAction::default();
            }
        }
        res
    }

    pub(crate) fn execute_async_send_receive(&mut self, universe: &Universe, op: QueueOpReq) -> OpNextAction {
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

    pub(crate) fn do_enq_with_q(&mut self, universe: &Universe, queue: &mut VecDeque<QueueOpReq>) {
        while let Some(mut op) = queue.pop_front() {
            let buff_index: usize;
            match op.message {
                ENQ_INVOKE => {
                    self.message_buffer[op.sender as usize].value = op.value;
                    buff_index = op.sender as usize;
                }
                ENQ_REQ | DEQ_REQ => {
                    buff_index = op.sender as usize;
                }
                ENQ_ACK => {
                    buff_index = op.receiver as usize;
                }
                DEQ_INVOKE => {
                    buff_index = op.sender as usize;
                }
                SAFE_UNSAFE => {
                    buff_index = op.value as usize; // TODO fix this
                    op.message = if self.message_buffer[buff_index].message == UNSAFE
                    {UNSAFE} else {SAFE}
                }
                _ => {
                    buff_index = 0;
                }
            }

            if self.index == op.receiver { // you are the proc being messaged
                self.message_buffer[buff_index] = self.execute_async_send_receive(universe, QueueOpReq{
                    message: op.message,
                    value: self.message_buffer[buff_index].value,
                    sender: op.sender,
                    receiver: op.receiver,
                    timestamp: self.message_buffer[buff_index].ts,
                });
            } else {
                self.execute_async_send_receive(universe, QueueOpReq{
                    message: op.message,
                    value: self.message_buffer[buff_index].value,
                    sender: op.sender,
                    receiver: op.receiver,
                    timestamp: self.message_buffer[buff_index].ts,
                });
            }
        }
        println!("{} {:?}", self.index, self.local_queue);
        //println!("{} {:?}", self.index, self.message_buffer);
    }


    fn enqueue(&mut self, universe: &Universe, invoking: Rank, val: u16) -> Result<(), Box<dyn std::error::Error>> {
        let world = universe.world();

        if invoking > world.size() {
            return Err(Box::new(OutOfRangeError));
        }

        let root_process = world.process_at_rank(invoking);

        let mut new_enq_req: EnqReq;
        if world.rank() == invoking {
            self.vector_clock.0[self.index as usize] += 1;  // Im not naming this structs' only var
            self.enq_count += 1;

            new_enq_req = EnqReq{
                message: ENQ_REQ,
                value: val,
                rank: self.index,
                timestamp: self.vector_clock};

            //println!("Root {} Requesting to enqueue {} at ts {:?}",
                     //self.index, new_enq_req.value, new_enq_req.timestamp);
            self.formatted_strings.push(format!("Root {} Requesting to enqueue {} at ts {:?}",self.index, new_enq_req.value, new_enq_req.timestamp));
        } else {
            new_enq_req = EnqReq{
                message: PLACEHOLDER,
                value: 0,
                rank: -1,
                timestamp: VectorClock([-1; NUM_PROCS])
            };
        }

        // Send
        mpi::request::scope(|scope| {
            root_process.immediate_broadcast_into(scope, &mut new_enq_req).wait();
        });

        // Recv
        //println!("Rank {} recv: {:?}. at time: {}", world.rank(), new_enq_req, Local::now());
        self.formatted_strings.push(format!("Rank {} recv: {:?}. at time: {}", world.rank(), new_enq_req, Local::now()));

        // for debug purposes we enq the rank with the value
        //self.local_queue.push_back((new_enq_req.rank, new_enq_req.value));
        update_ts(&mut self.vector_clock.0, &new_enq_req.timestamp.0);

        for confirmationList in &mut self.lists {
            match compare_ts(&confirmationList.ts, &self.vector_clock.0) {
                ComparisonResult::Less | ComparisonResult::StrictlyLess // accept less or strictly less
                => confirmationList.response_list[self.index as usize] = 1,
                _ => {}
            }
        }

        let i = ENQ_ACK; // Have all processes send EnqAck to the root
        if self.index == invoking {
            let mut recv_enq_responses:[u16; NUM_PROCS] = [0; NUM_PROCS];

            // Note gather does some work for us here and waits until all processes respond
            world.process_at_rank(self.index).gather_into_root(&i, &mut recv_enq_responses[..]);
        }else {
            world.process_at_rank(invoking).gather_into(&i);
        }

        Ok(())
    }

    // Wrapper for enqueue to handle errors
    pub(crate) fn Enqueue(&mut self, universe: &Universe, invoking: Rank, val: u16) {
        let result = self.enqueue(universe, invoking, val);
        match result {
            Ok(()) => {},
            Err(err) => eprintln!("Error: {}", err)
        }
    }

    fn dequeue(&mut self, universe: &Universe, invoking: Rank) -> Result<HandleDequeue, io::Error> {
        let world = universe.world();
        let root_process = world.process_at_rank(invoking);

        let mut new_deq_req:DeqReq;
        if self.index == invoking {
            self.vector_clock.0[self.index as usize] += 1;
            new_deq_req = DeqReq{
                message: DEQ_REQ,
                rank: self.index,
                timestamp: self.vector_clock
            };
        }else{
            new_deq_req = DeqReq{
                message: PLACEHOLDER,
                rank: -1,
                timestamp: VectorClock([-1; NUM_PROCS])
            };
        }

        // Send
        mpi::request::scope(|scope| {
            root_process.immediate_broadcast_into(scope, &mut new_deq_req).wait();
        });

        //println!(" Process {} recv: {:?} with curr ts {:?} at time {}", self.index, new_deq_req, self.vector_clock, Local::now());
        self.formatted_strings.push(format!(" Process {} recv: {:?} with curr ts {:?} at time {}", self.index, new_deq_req, self.vector_clock, Local::now()));

        self.lists.push(
            ConfirmationList::new(new_deq_req.timestamp.0)
        );

        let mut response: SafeUnsafeAck;
        if self.index != invoking {
            update_ts(&mut self.vector_clock.0, &new_deq_req.timestamp.0);
        }

        match compare_ts(&new_deq_req.timestamp.0, &self.vector_clock.0) {
            ComparisonResult::StrictlyLess if !contains_all_zeros(&new_deq_req.timestamp.0) => {
                // Send unsafe
                response = SafeUnsafeAck{
                    message:UNSAFE,
                    rank: self.index,
                    timestamp: new_deq_req.timestamp
                };
            }
            _ => {  // Otherwise send safe
                response = SafeUnsafeAck{
                    message:SAFE,
                    rank: self.index,
                    timestamp: self.vector_clock // this value doesnt matter for safes
                };
            }
        }

        // Send all unsafe/safe lists to all
        let mut safe_unsafe_responses: [SafeUnsafeAck; NUM_PROCS] = [response; NUM_PROCS];
        world.all_gather_into(&response, &mut safe_unsafe_responses[..]);
        //println!("Process {} recv {:?} at time {}", self.index, safe_unsafe_responses, Local::now());
        self.formatted_strings.push(format!("Process {} recv {:?} at time {}", self.index, safe_unsafe_responses, Local::now()));

        if !safe_unsafe_responses.iter().any(|item| item.message == UNSAFE){
            // all responses safe, dequeue
            //if let Some((rank, value)) = self.local_queue.pop_front() {
                //self.remove_confirmation_list(&new_deq_req.timestamp);
                //return Ok::<HandleDequeue, io::Error>(HandleDequeue::Success((rank, value)));
            //}
        }

        for response in safe_unsafe_responses.iter(){
            match self.handle_unsafe(*response) {
                Ok::<HandleDequeue, io::Error>(HandleDequeue::Success((rank, value))) => {
                    return Ok::<HandleDequeue, io::Error>(HandleDequeue::Success((rank, value)));
                }
                Err(E) => {
                    return Err(io::Error::new(io::ErrorKind::Other, E));
                }
                _ => {}
            }
        }

        Ok::<HandleDequeue, io::Error>(HandleDequeue::NoResult)
    }

    fn handle_unsafe(&mut self, safe_unsafe_response: SafeUnsafeAck) -> Result<HandleDequeue, io::Error> {
        if safe_unsafe_response.message == UNSAFE {
            let mut contains_req = false;
            for confirmation_list in &mut self.lists.iter() {
                match compare_ts_ord(&confirmation_list.ts, &safe_unsafe_response.timestamp.0) {
                    Ordering::Equal => {
                        contains_req = true;
                    }
                    _ => {}
                }
            }
            if !contains_req { // we dont have this ts in our confirmation list
                self.add_confirmation_list(ConfirmationList::new(safe_unsafe_response.timestamp.0))
            }

            for mut confirmation_list in self.lists.iter_mut() {
                match compare_ts_ord(&confirmation_list.ts, &safe_unsafe_response.timestamp.0) {
                    Ordering::Equal => {
                        confirmation_list.response_list[safe_unsafe_response.rank as usize] = 2;
                    }
                    _ => {}
                }
            }

            propagate_earlier_responses(&mut self.lists);

            // TODO update unsafes after deq finishes
        }else{
            for mut confirmation_list in self.lists.iter_mut() {
                match compare_ts(&confirmation_list.ts, &safe_unsafe_response.timestamp.0) {
                    ComparisonResult::Less | ComparisonResult::StrictlyLess => {
                        confirmation_list.response_list[safe_unsafe_response.rank as usize] = 1;
                    },
                    _ => {}
                }
            }
            // TODO can we update previous for safes?
        }

        for confirmation_list in self.lists.iter() {
            if !confirmation_list.response_list.contains(&0) {
                let mut pos: usize = 0;
                for response in confirmation_list.response_list.iter() {
                    if *response == 2 {
                        pos += 1;
                    }
                }
                /*
                return if let Some((rank, value)) = self.local_queue.remove(pos) {
                    Ok::<HandleDequeue, io::Error>(HandleDequeue::Success((rank, value)))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Process {} failed to dequeue index {}, local queue: {:?}",
                                self.index, pos, self.local_queue))
                    )
                }

                 */

            }
        }
        Ok(HandleDequeue::NoResult)
    }

    pub(crate) fn Dequeue(&mut self, universe: &Universe, invoking: Rank) -> Option<(Rank, u16)> {
        match self.dequeue(universe, invoking) {
            Ok::<HandleDequeue, io::Error>(HandleDequeue::Success((rank, value))) => {
                Some((rank, value))
            }
            Err(E) => {
                eprintln!("{}", E);
                None
            }
            _ => None
        }
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ Process: {}, local_queue: {:?}, ts: {:?}, enq_count: {} }}",
            self.index, self.local_queue, self.vector_clock, self.enq_count
        )
    }
}