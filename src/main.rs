use arrayvec::ArrayVec;
use maplit::hashmap;
use nix::unistd::{dup2, execvp, fork, pipe, ForkResult};
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::str;

const DEFAULT_COLOR: i32 = 0;
const RED: i32 = 31;
const YELLOW: i32 = 33;
const GREEN: i32 = 32;

pub struct Command<'a> {
    out_color: i32,
    err_color: i32,
    filename: Option<&'a str>,
    args: Vec<&'a str>,
}

fn parse_incoming_command_line(args: &[String]) -> Box<Command> {
    dbg!(args);

    let color_map = hashmap! {
        "default".to_string() => DEFAULT_COLOR,
        "red".to_string() => RED,
        "green".to_string() => GREEN,
        "yellow".to_string() => YELLOW,
    };

    let mut cmd = Box::new(Command {
        filename: None,
        args: Vec::<&str>::with_capacity(0),
        out_color: DEFAULT_COLOR,
        err_color: DEFAULT_COLOR,
    });

    for a in args.iter() {
        if a.starts_with("--cmd=") {
            let cmd_string = &a["--cmd=".chars().count()..];
            println!("this is a command {} using {}", a, cmd_string);

            let mut arglist = cmd_string.split(" ");

            cmd.filename = arglist.nth(0);
            cmd.args = arglist.collect();
        } else if a.starts_with("--out=") {
            let out_color_string = &a["--out=".chars().count()..].to_string();
            println!("this is a command {} using {}", a, out_color_string);

            cmd.out_color = color_map[out_color_string];
        } else if a.starts_with("--err=") {
            let err_color_string = &a["--err=".chars().count()..].to_string();
            println!("this is a command {} using {}", a, err_color_string);

            cmd.err_color = color_map[err_color_string];
        } else {
            println!("argument not recognized skipping {}", a);
        }
    }

    return cmd;
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let cmd = parse_incoming_command_line(&args);

    if cmd.filename.is_none() {
        return Ok(());
    }

    let safefilename = cmd.filename.unwrap();
    let cstring_filename = CString::new(safefilename).unwrap();
    let cmdname = cstring_filename.as_c_str();
    let argiter = cmd.args.into_iter();

    let mut argsvec = ArrayVec::<CString, 3>::new();
    argsvec.push(CString::new(safefilename).unwrap());
    argiter.for_each(|s| argsvec.push(CString::new(s).unwrap()));

    let (from_child_fd, to_parent) = pipe().unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("yay we are the parent, process is {}", child);

            unsafe {
                let mut from_child = File::from_raw_fd(from_child_fd);

                let mut buf = [0; 1024];
                Read::read(&mut from_child, &mut buf).unwrap();
                loop {
                    print!("\x1b[33m{}\x1b[0m", str::from_utf8(&buf).unwrap());
                    Read::read(&mut from_child, &mut buf).unwrap();
                }
            };
        }
        Ok(ForkResult::Child) => {
            dup2(to_parent, 1).unwrap();
            execvp(cmdname, &argsvec.into_inner().unwrap())
                .map_err(|err| println!("error from execvp {:?}", err))
                .ok();
        }
        Err(_) => println!("Error fork failed"),
    }
    return Ok(());
}
