use crate::util::process::Process;
use std::fmt::Write;

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