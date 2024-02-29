use mpi::environment::Universe;
use mpi::Rank;
use crate::util::constants::{DEQ_INVOKE, DEQ_REQ, ENQ_ACK, ENQ_INVOKE, ENQ_REQ, NUM_PROCS, SAFE, UNSAFE};
use crate::util::message_structs::{OpNextAction, OpRequest, QueueOpReq, VectorClock};
use crate::util::process::Process;


pub struct DequeueFixedLinearization<'universe> {
    pub(crate) invoker: Rank, // stores rank of initial invoker
    pub(crate) message_buffer: u16,
    pub(crate) deq_ts: VectorClock,
    universe: &'universe Universe,
}

impl<'universe> DequeueFixedLinearization<'universe> {
    pub(crate) fn new(u: &'universe Universe) -> Self {
        DequeueFixedLinearization {
            invoker: 0,
            message_buffer: Default::default(),
            deq_ts: Default::default(),
            universe: u,
        }
    }

    pub(crate) fn deq_invoke(&mut self, invoking: Rank, process: &mut Process) {
        self.invoker = invoking;
        if process.index == invoking {
            self.message_buffer = process.execute_async_send_receive(self.universe, QueueOpReq{
                message: DEQ_INVOKE,
                value: 0,
                sender: invoking,
                receiver: invoking,
                timestamp: process.vector_clock
            }).message;
            self.deq_ts = process.vector_clock;
        }else{
            process.execute_async_send_receive(self.universe, QueueOpReq{
                message: DEQ_INVOKE,
                value: 0,
                sender: invoking,
                receiver: invoking,
                timestamp: process.vector_clock
            });
        }
    }

    pub(crate) fn deq_req(&mut self, receiver: Rank, process: &mut Process) {
        if process.index == receiver {
            let response = process.execute_async_send_receive(self.universe, QueueOpReq{
                message: DEQ_REQ,
                value: 0,
                sender: self.invoker,
                receiver,
                timestamp: self.deq_ts
            });
            self.message_buffer = response.message;
            self.deq_ts = response.ts;
        }else {
            process.execute_async_send_receive(self.universe, QueueOpReq{
                message: DEQ_REQ,
                value: 0,
                sender: self.invoker,
                receiver,
                timestamp: self.deq_ts
            });
        }
    }

    pub(crate) fn safe_unsafe(&mut self, sender: Rank, receiver: Rank, process: &mut Process) {
        if process.index == receiver {
            self.message_buffer = process.execute_async_send_receive(self.universe, QueueOpReq{
                message: self.message_buffer,
                value: 0,
                sender,
                receiver,
                timestamp: self.deq_ts
            }).message;
        }else {
            process.execute_async_send_receive(self.universe, QueueOpReq{
                message: self.message_buffer,
                value: 0,
                sender,
                receiver,
                timestamp: self.deq_ts
            });
        }
    }

    pub(crate) fn safe_unsafe_all(&mut self, sender: Rank, process: &mut Process) {
        for i in 0..NUM_PROCS {
            self.safe_unsafe(sender, i as Rank, process);
        }
    }
}

pub struct EnqueueFixedLinearization<'universe> {
    pub(crate) invoker: Rank, // stores rank of initial invoker
    pub(crate) message_buffer: u16,
    pub(crate) enq_ts: VectorClock,
    universe: &'universe Universe,
    value: u16
}

impl<'universe> EnqueueFixedLinearization<'universe> {
    pub(crate) fn new(u: &'universe Universe) -> Self {
        EnqueueFixedLinearization {
            invoker: Default::default(),
            message_buffer: Default::default(),
            enq_ts: Default::default(),
            universe: u,
            value: Default::default()
        }
    }

    pub(crate) fn enq_invoke(&mut self, invoking: Rank, value: u16, process: &mut Process) {
        self.invoker = invoking;
        if process.index == invoking {
            let response = process.execute_async_send_receive(self.universe, QueueOpReq{
                message: ENQ_INVOKE,
                value,
                sender: invoking,
                receiver: invoking,
                timestamp: process.vector_clock
            });
            self.message_buffer = response.message;
            self.value = response.value;
            self.enq_ts = process.vector_clock;
        }else{
            process.execute_async_send_receive(self.universe, QueueOpReq{
                message: ENQ_INVOKE,
                value,
                sender: invoking,
                receiver: invoking,
                timestamp: process.vector_clock
            });
        }
    }

    pub(crate) fn enq_req(&mut self, receiver: Rank, process: &mut Process) {
        if process.index == receiver {
            let response = process.execute_async_send_receive(self.universe, QueueOpReq{
                message: ENQ_REQ,
                value: self.value,
                sender: self.invoker,
                receiver,
                timestamp: self.enq_ts
            });
            self.value = response.value;
            self.message_buffer = response.message;
            self.enq_ts = response.ts;
        }else {
            process.execute_async_send_receive(self.universe, QueueOpReq{
                message: ENQ_REQ,
                value: self.value,
                sender: self.invoker,
                receiver,
                timestamp: self.enq_ts
            });
        }
    }

    pub(crate) fn enq_ack(&mut self, sender: Rank, process: &mut Process) {
        if process.index == self.invoker {
            self.message_buffer = process.execute_async_send_receive(self.universe, QueueOpReq{
                message: ENQ_ACK,
                value: self.value,
                sender,
                receiver: self.invoker,
                timestamp: self.enq_ts
            }).message;
        }else {
            process.execute_async_send_receive(self.universe, QueueOpReq{
                message: ENQ_ACK,
                value: self.value,
                sender,
                receiver: self.invoker,
                timestamp: self.enq_ts
            });
        }
    }
}





