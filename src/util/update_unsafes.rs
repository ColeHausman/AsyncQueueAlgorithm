use crate::util::confirmation_list::ConfirmationList;

pub fn update_unsafes(lists: &mut Vec<ConfirmationList>, row_start: usize, index_to_update: usize) {
    for row in row_start + 1..lists.len() {
        if lists[row].response_list[index_to_update] == 2 {
            lists[row].response_list[index_to_update] = 1;
        }
    }
}