use std::cmp::Ordering;
use crate::util::constants::NUM_PROCS;

#[derive(Debug, PartialEq)]
pub enum ComparisonResult {
    Less,
    StrictlyLess,
    Greater,
}

/// Here is the actual good comparison function, it can distinguish between strictly less and less
/// when comparing two vector timestamps
///
/// # Arguments
///
/// * `vec_i` - A reference to the first timestamp vector.
/// * `vec_j` - A reference to the second timestamp vector.
///
/// # Returns
///
/// * `ComparisonResult` - An enum representing the comparison result:
pub fn compare_ts(vec_i: &[i32; NUM_PROCS], vec_j: &[i32; NUM_PROCS]) -> ComparisonResult {
    let mut strictly_less = true;
    let mut less = false;

    for (ts_i, ts_j) in vec_i.iter().zip(vec_j.iter()) {
        if ts_i >= ts_j {
            strictly_less = false;
            if ts_i > ts_j {
                less = true;
            }
        }
    }

    if strictly_less {
        ComparisonResult::StrictlyLess
    } else if less {
        ComparisonResult::Less
    } else {
        ComparisonResult::Greater
    }
}



/// This function is just here because Its easier to do sorting with standard Ordering
/// and idc about strict or regular less than in ts sorting, its one or the other
/// and im too lazy to rewrite the sorter for now
///
/// # Arguments
///
/// * `vec_i` - A reference to the first timestamp vector.
/// * `vec_j` - A reference to the second timestamp vector.
///
/// # Returns
///
/// * `Ordering` - A very standard ordering idk, no shot I write these comments for the whole project
pub fn compare_ts_ord(vec_i: &[i32; NUM_PROCS], vec_j: &[i32; NUM_PROCS]) -> Ordering {
    for (ts_i, ts_j) in vec_i.iter().zip(vec_j.iter()) {
        match ts_i.cmp(ts_j) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return Ordering::Greater,
            Ordering::Equal => {} // Continue comparing the next elements
        }
    }

    Ordering::Equal // All elements are equal
}

pub fn contains_all_zeros(vec_i: &[i32; NUM_PROCS]) -> bool {
    for ts_i in vec_i.iter() {
        if *ts_i != 0 {
            return false;
        }
    }

    true
}