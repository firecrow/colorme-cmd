use maplit::hashmap;
use nix::unistd::execvp;
use std::env;
use std::ffi::CString;

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

            let mut arglist = cmd_string.split_whitespace();

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

fn main() {
    let args: Vec<String> = env::args().collect();
    parse_incoming_command_line(&args);

    let cmd = CString::new("ls").unwrap();
    let dir = CString::new(".").unwrap();

    execvp(&cmd, &[&cmd, &dir])
        .map_err(|err| println!("error from execvp {:?}", err))
        .ok();
}
