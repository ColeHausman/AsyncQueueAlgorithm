//! These define the equivalent MPI datatype which we have to do in order
//! to send arrays which we need to do because vector timestamps
//! `Equivalence` is the only required trait so that is all I implemented

use std::mem::size_of;
use mpi::datatype::{Equivalence, UserDatatype};
use mpi::{Address, Count, Rank};
use crate::util::numeric_encodings::req_encoding_to_string;
use crate::util::constants::NUM_PROCS;

#[derive(Copy, Clone)]
pub(crate) struct VectorClock(pub [i32; NUM_PROCS]);

unsafe impl Equivalence for VectorClock {
    type Out = UserDatatype;

    fn equivalent_datatype() -> Self::Out {
        let mut displacements = Vec::with_capacity(NUM_PROCS);
        let size_of_i32 = size_of::<i32>() as Address;
        for i in 0..NUM_PROCS {
            displacements.push(size_of_i32 * (NUM_PROCS - i - 1) as Address);
        }

        UserDatatype::structured(
            &[1; NUM_PROCS],
            &displacements,
            &[i32::equivalent_datatype(); NUM_PROCS],
        )
    }
}

impl std::fmt::Debug for VectorClock { // Define debug format for vector clocks
fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
        f,
        "{:?}",
        self.0
    )
}
}
#[derive(Clone, Copy)]
pub struct EnqReq {
    pub message: u16,
    pub value: u16,
    pub rank: Rank,
    pub timestamp: VectorClock,
}

unsafe impl Equivalence for EnqReq {
    type Out = UserDatatype;

    fn equivalent_datatype() -> Self::Out {
        let ts_equivalent = VectorClock::equivalent_datatype(); //Store to use ref

        let counts = [
            1 as Count, // One EnqReq
            1 as Count, // One u16 for message
            1 as Count, // One u8 for value
            1 as Count, // One Rank
            1 as Count, // One VectorClock
        ];

        let displacements = [ // Define memory offsets
            0 as Address, // Start at 0
            size_of::<u16>() as Address,
            size_of::<u16>() as Address,
            size_of::<Rank>() as Address,
            size_of::<u16>() as Address + size_of::<u16>()
                as Address + size_of::<Rank>() as Address, // Offset struct
        ];

        let types = [
            Count::equivalent_datatype(),
            u16::equivalent_datatype(),
            u16::equivalent_datatype(),
            Rank::equivalent_datatype(),
            ts_equivalent.as_ref(), // use temporary reference for lifetime requirements
        ];

        UserDatatype::structured(&counts, &displacements, &types)
    }
}

impl std::fmt::Debug for EnqReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ message: {}, value: {}, rank: {}, ts: {:?} }}",
            req_encoding_to_string(self.message), self.value, self.rank, self.timestamp
        )
    }
}

pub struct DeqReq {
    pub message: u16,
    pub rank: Rank,
    pub timestamp: VectorClock
}

unsafe impl Equivalence for DeqReq {
    type Out = UserDatatype;

    fn equivalent_datatype() -> Self::Out {
        let ts_equivalent = VectorClock::equivalent_datatype();

        let counts = [
            1 as Count, // One DeqReq
            1 as Count, // One u16
            1 as Count, // One Rank
            1 as Count // One VectorClock
        ];

        let displacements = [ // Define memory offsets
            0 as Address, // Start at 0
            size_of::<u16>() as Address,
            size_of::<Rank>() as Address,
            size_of::<u16>()
                as Address + size_of::<Rank>() as Address, // Offset struct
        ];

        let types = [
            Count::equivalent_datatype(),
            u16::equivalent_datatype(),
            Rank::equivalent_datatype(),
            ts_equivalent.as_ref()
        ];

        UserDatatype::structured(&counts, &displacements, &types)
    }
}

impl std::fmt::Debug for DeqReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ message: {}, rank: {}, ts: {:?} }}",
            req_encoding_to_string(self.message), self.rank, self.timestamp
        )
    }
}

#[derive(Copy, Clone)]
pub struct SafeUnsafeAck {
    pub message: u16,
    pub rank: Rank,
    pub timestamp: VectorClock
}

unsafe impl Equivalence for SafeUnsafeAck {
    type Out = UserDatatype;

    fn equivalent_datatype() -> Self::Out {
        let ts_equivalent = VectorClock::equivalent_datatype();

        let counts = [
            1 as Count, // One UnsafeAck
            1 as Count, // One u16
            1 as Count, // One Rank
            1 as Count // One VectorClock
        ];

        let displacements = [ // Define memory offsets
            0 as Address, // Start at 0
            size_of::<u16>() as Address,
            size_of::<Rank>() as Address,
            size_of::<u16>()
                as Address + size_of::<Rank>() as Address, // Offset struct
        ];

        let types = [
            Count::equivalent_datatype(),
            u16::equivalent_datatype(),
            Rank::equivalent_datatype(),
            ts_equivalent.as_ref()
        ];

        UserDatatype::structured(&counts, &displacements, &types)
    }
}

impl std::fmt::Debug for SafeUnsafeAck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.message {
            3 => {
                write!(
                    f,
                    "{{ message: {}, rank: {}, ts: {:?} }}",
                    req_encoding_to_string(self.message), self.rank, self.timestamp
                )
            },
            4 => {
                write!(
                    f,
                    "{{ message: {}, rank: {} }}",
                    req_encoding_to_string(self.message), self.rank
                )
            },
            _ => { // should never get here
                write!(
                    f,
                    ""
                )
            }
        }
    }
}