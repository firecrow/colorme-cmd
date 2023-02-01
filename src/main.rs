use nix::unistd::execvp;
use std::env;
use std::ffi::CString;

/*
const RED: i32 = 31;
const YELLOW: i32 = 33;
const GREEN: i32 = 32;

pub struct command {
    out_color: i32,
    filename: CString,
    args: Vec<CString>,
}
*/

fn parse_incoming_command_line(args: &[String]) {
    dbg!(args);
    for a in args.iter() {
        if a.starts_with("--cmd") {
            let cmd_string = &a["--cmd=".chars().count()..].to_string();
            println!("this is a command {} using {}", a, cmd_string);
        } else if a.starts_with("--out") {
            println!("this specifies a stdout color {}", a);
        } else if a.starts_with("--err") {
            println!("this specifies a stderr color {}", a);
        } else {
            println!("unknown arg found {}", a);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    parse_incoming_command_line(&args);

    let cmd = CString::new("ls").unwrap();
    let dir = CString::new(".").unwrap();

    execvp(&cmd, &[&cmd, &dir])
        .map_err(|err| println!("error from execvp {:?}", err))
        .ok();
}
