use crate::util::confirmation_list::ConfirmationList;

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