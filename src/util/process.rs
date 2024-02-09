use std::cmp::Ordering;
use std::collections::VecDeque;
use mpi::datatype::UserDatatype;
use mpi::environment::Universe;
use mpi::{Count, Rank, Address};
use mpi::topology::SimpleCommunicator;
use crate::util::compare_ts::{compare_ts, compare_ts_ord, ComparisonResult, contains_all_zeros};
use crate::util::message_structs::{DeqReq, EnqReq, SafeUnsafeAck, VectorClock};
use crate::util::confirmation_list::ConfirmationList;
use crate::util::constants::{NUM_PROCS, ENQ_REQ, DEQ_REQ, ENQ_ACK, UNSAFE, SAFE};
use mpi::traits::*;
use crate::util::update_ts::update_ts;
use std::fmt;
use std::mem::size_of;
use crate::util::print_confirmation_lists::print_confirmation_lists;
use crate::util::propagate_earlier_responses::propagate_earlier_responses;

const PLACEHOLDER: u16 = 0xFFFC;

// Define a custom error type for the out of range case
#[derive(Debug)]
struct OutOfRangeError;

impl fmt::Display for OutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process out of range world size!")
    }
}

// Implement the Error trait for the custom error type
impl std::error::Error for OutOfRangeError {}

pub struct Process {
    pub(crate) index: Rank, // stores process index
    pub(crate) vector_clock: VectorClock, // stores process vector clock
    pub(crate) lists: Vec<ConfirmationList>, // stores confirmation lists
    pub(crate) enq_count: u16, // stores number of enqueues
    local_queue:VecDeque<(Rank, u16)>
}

impl Process {
    pub(crate) fn initialize(world: &SimpleCommunicator) -> Self {
        let enq_count = 0u16; // initialize enq_count to 0

        Self {
            index: world.rank(), // get process ID from MPI world
            vector_clock: VectorClock([0; NUM_PROCS]),
            lists: Vec::new(), // holds confirmation lists
            enq_count, // counts num enqueues
            local_queue: VecDeque::new() // initialize empty local queue
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

            println!("Root {} broadcasting value: {:?}.", self.index, new_enq_req);
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
        println!("Rank {} recv: {:?}.", world.rank(), new_enq_req);

        // for debug purposes we enq the rank with the value
        self.local_queue.push_back((new_enq_req.rank, new_enq_req.value));
        update_ts(&mut self.vector_clock.0, &mut new_enq_req.timestamp.0);

        for confirmationList in &mut self.lists {
            match compare_ts(&confirmationList.ts, &self.vector_clock.0) {
                ComparisonResult::Less | ComparisonResult::StrictlyLess // accept less or strict less
                => confirmationList.response_list[self.index as usize] = 1,
                _ => {} // im not handling the other orderings, ignore
            }
        }

        let i = ENQ_ACK; // Have all processes send EnqAck to the root
        if self.index == invoking {
            let mut recv_enq_responses:[u16; NUM_PROCS] = [0; NUM_PROCS];

            // Note gather does some work for us here and waits until all processes respond
            world.process_at_rank(self.index).gather_into_root(&i, &mut recv_enq_responses[..]);

            let formatted = vec![String::from("ENQ_ACK"); recv_enq_responses.len()-1];
            println!("process {} received {:?}", self.index, formatted);
        }else {
            world.process_at_rank(invoking).gather_into(&i);
        }

        println!("Process {} finished", self.index);
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

    pub(crate) fn dequeue(&mut self, universe: &Universe, invoking: Rank) {
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

        println!(" Process {} recv: {:?} with curr ts {:?}", self.index, new_deq_req, self.vector_clock);

        self.lists.push(
            ConfirmationList::new(new_deq_req.timestamp.0)
        );

        let mut response: SafeUnsafeAck;
        if self.index != invoking {
            update_ts(&mut self.vector_clock.0, &mut new_deq_req.timestamp.0);
        }

        match compare_ts(&new_deq_req.timestamp.0, &self.vector_clock.0) {
            ComparisonResult::StrictlyLess if !contains_all_zeros(&new_deq_req.timestamp.0) => {
                // Send unsafe
                response = SafeUnsafeAck{
                    message:UNSAFE,
                    rank: self.index,
                    timestamp: new_deq_req.timestamp};
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
        let mut safe_unsafe_responses: [SafeUnsafeAck; NUM_PROCS] = [response; NUM_PROCS]; // holds safe unsafe responses
        world.all_gather_into(&response, &mut safe_unsafe_responses[..]);
        println!("Process {} recv {:?}", self.index, safe_unsafe_responses);

        if !safe_unsafe_responses.iter().any(|item| item.message == UNSAFE){
            // all responses safe, dequeue
            if let Some(value) = self.local_queue.pop_front() {
                println!("Process {} dequeued value {:?}", self.index, value);
            }
        }

        for response in safe_unsafe_responses.iter(){
            self.handle_unsafe(*response);
        }
    }

    fn handle_unsafe(&mut self, safe_unsafe_response: SafeUnsafeAck) {
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
                if let Some(value) = self.local_queue.remove(pos){
                    println!("Process {} dequeuing {:?}", self.index, value);
                } else{
                    println!("Process {} failed to dequeue index {}", self.index, pos);
                }

            }
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

