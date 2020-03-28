use std::io;
use std::fs;

extern crate timewarrior_timesheet_report;

use timewarrior_timesheet_report as report;

type ExpectedValue = Result<Vec<u8>, timewarrior_timesheet_report::Error>;

struct TestError(String);

fn check_input_buf(input: &mut dyn io::BufRead, expected: ExpectedValue) -> Result<(), TestError> {
    Ok(())
}

fn check_input_file(path: &std::path::Path) -> Result<(), TestError> {
    let empty_config = report::Config{};
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => return Err(TestError(error.to_string())),
    };
    let mut input = io::BufReader::new(file);
    let mut output: Vec<u8> = Vec::new();
    match report::run(empty_config, &mut input, &mut output) {
        Ok(()) => {
            write!(io::stdout().lock(), "{}", std::str::from_utf8(&output).unwrap())?;
        },
        Err(e) => { 
            return Err(e.to_string());
        },
    }
    Ok(())
}

#[test]
fn check_input_files() {
    let mut test_pass: bool = true;
    for entry in fs::read_dir(".").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let extension = path.extension();
        let ext: std::ffi::OsString;
        match extension {
            None => continue,
            Some(extension) => ext = std::ffi::OsString::from(extension),
        }
        if ext.to_str() != Some("input") {
            continue;
        };

        match check_input_file(&path) {
            Err(error) => { test_pass=false; eprintln!("{:?}", error); },
            _ => (),
        }
    }
    assert!(test_pass, "Failed all input file checks.");
}
