// vim: set sw=4 ts=4 et :

use std::io;
use timewarrior_timesheet_report::{run, Config};

fn main() -> Result<(), io::Error> {
    match run(Config{}, &mut io::stdin().lock(), &mut io::stdout().lock()) {
        Ok(()) => Ok(()),
        Err(_) => panic!("Error in run"),
    }
}
