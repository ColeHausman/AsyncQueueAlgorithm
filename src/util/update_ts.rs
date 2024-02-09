use crate::util::constants::NUM_PROCS;

pub fn update_ts(vec_i: &mut [i32; NUM_PROCS], vec_j: &mut [i32; NUM_PROCS]) {
    for (ts_i, ts_j) in vec_i.iter_mut().zip(vec_j.iter()) {
        *ts_i = (*ts_i).max(*ts_j);
    }
}