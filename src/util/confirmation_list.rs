use crate::util::constants::NUM_PROCS;
use crate::util::process::Process;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct ConfirmationList {
    pub(crate) response_list: [i32; NUM_PROCS],
    pub(crate) ts: [i32; NUM_PROCS],
    pub(crate) handled: bool
}

impl ConfirmationList {
    pub(crate) fn new(dequeue_ts: [i32; NUM_PROCS]) -> Self {
        let mut response_list = [0; NUM_PROCS]; // Initialize response_list with n zeros

        Self {
            response_list,
            ts: dequeue_ts,
            handled: false
        }
    }
}

pub fn propagate_earlier_responses(lists: &mut Vec<ConfirmationList>) {
    for col in 0..lists[0].response_list.len() {
        for row in (1..lists.len()).rev() {
            let response = &lists[row].response_list[col];

            if *response != 0 && lists[row - 1].response_list[col] == 0 {
                lists[row - 1].response_list[col] = *response;
            }
        }
    }
}

pub fn update_unsafes(lists: &mut Vec<ConfirmationList>, row: usize) {
    if row < lists.len() {
        for response in lists[row].response_list.iter_mut() {
            if *response == 2 {
                *response = 1;
            }
        }
    }
}

pub fn print_confirmation_lists(process: &Process) {
    let mut output = String::new();

    writeln!(output, "ConfirmationList for process {}", process.index).unwrap();
    for confirmation_list in &process.lists {
        writeln!(
            output,
            "list {:?} ts {:?}",
            confirmation_list.response_list, confirmation_list.ts
        )
            .unwrap();
    }

    println!("{}", output);
}