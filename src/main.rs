mod util;

use mpi::traits::*;
use std::error::Error;
use crate::util::process::Process;
use crate::util::compare_ts::{compare_ts, ComparisonResult};

const COUNT: usize = 10;
const ENQ_REQ: u16 = 0;
const DEQ_REQ: u16 = 1;

fn main() {
    //let mut vec_i: Vec<u8> = vec![0, 1, 0, 0, 2];
    //let mut vec_j: Vec<u8> = vec![2, 2, 1, 1, 1];
    //let mut vec_k: Vec<u8> = vec![3,0,0,0,0];
    //let mut vec_p: Vec<u8> = vec![0,9,9,9,9];
    //let mut inst1 = ConfirmationList::new(5, vec_j,vec![0,0,0,1]);
    //let mut inst2 = ConfirmationList::new(5, vec_i, vec![0,0,0,1]);
    //let mut inst3 = ConfirmationList::new(5, vec_k, vec![0,0,0,1]);
    //let mut inst4 = ConfirmationList::new(5, vec_p, vec![0,0,0,1]);
    //let mut inst3 = ConfirmationList::new(5, &vec_j, 1);
    //update_ts(&mut inst1.response_list, &mut vec_i);
    //update_ts(&mut inst2.response_list, &mut vec_j);
    //let mut lists: Vec<ConfirmationList> = vec![&inst1, &inst2, &inst3];
    //propagate_earlier_responses(&mut lists);
    //println!("{:?}", lists[1].response_list);
    //println!("{:?}", lists[0].response_list);

    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    //let size = world.size();
    let rank = world.rank();

    let mut p_i = Process::initialize(&world);

    /*

    p_i.add_confirmation_list(inst1);
    p_i.add_confirmation_list(inst2);
    p_i.add_confirmation_list(inst4);
    p_i.add_confirmation_list(inst3);
    p_i.lists[0].response_list[0] = 2;
    p_i.lists[1].response_list[0] = 2;
    p_i.lists[2].response_list[0] = 2;
    p_i.lists[3].response_list[0] = 2;
    print_confirmation_lists(&p_i);

    update_unsafes(&mut p_i.lists, 0, 0);
    print_confirmation_lists(&p_i);

     */

    let mut result: Vec<i32> = vec![0; COUNT];
    if rank == 0 {
        result[0] = 99;
    }
    if rank == 1 {
        result[1] = 89;
    }

    p_i.Enqueue(&universe, 1, 12);

    p_i.Enqueue(&universe, 2, 13);

    p_i.dequeue(&universe, 0);
}