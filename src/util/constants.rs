/// For now you gotta hardcode num processes at compile time (Fuck)
pub const NUM_PROCS: usize = 4;
pub const ENQ_REQ: u16 = 0;
pub const DEQ_REQ: u16 = 1;
pub const ENQ_ACK: u16 = 2;
pub const UNSAFE: u16 = 3;
pub const SAFE: u16 = 4;
pub const ENQ_INVOKE: u16 = 5;
pub const DEQ_INVOKE: u16 = 6;