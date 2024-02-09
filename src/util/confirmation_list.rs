use crate::util::constants::NUM_PROCS;

#[derive(Debug, Clone)]
pub struct ConfirmationList {
    pub(crate) response_list: [i32; NUM_PROCS],
    pub(crate) ts: [i32; NUM_PROCS],
}

impl ConfirmationList {
    pub(crate) fn new(dequeue_ts: [i32; NUM_PROCS]) -> Self {
        let mut response_list = [0; NUM_PROCS]; // Initialize response_list with n zeros

        Self {
            response_list,
            ts: dequeue_ts,
        }
    }
}