#![allow(dead_code)]

use libc::fcntl;
use maplit::hashmap;
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{dup2, execvp, fork, pipe, ForkResult, Pid};
use serde_derive::{self, Deserialize, Serialize};
use serde_yaml;
use std::error::Error;
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::str;
use std::thread::sleep;
use std::{env, time};

const DEFAULT_COLOR: i32 = 0;
const RED: i32 = 31;
const YELLOW: i32 = 33;
const GREEN: i32 = 32;
const BLUE: i32 = 34;
const PURPLE: i32 = 35;

fn get_color(color: &String) -> i32 {
    let color_map = hashmap! {
        "default" => DEFAULT_COLOR,
        "red" => RED,
        "green" => GREEN,
        "yellow" => YELLOW,
        "blue" => BLUE,
        "purple" => PURPLE,
    };
    return color_map[&color[..]];
}

pub struct OptConfiguration<'a> {
    config_filename: &'a str,
}

const DEFAULT_CONFIG_FILENAME: &str = "cmd.yml";

fn parse_incoming_command_line(args: &[String]) -> Box<OptConfiguration> {
    dbg!(args);

    let mut config = Box::new(OptConfiguration {
        config_filename: &DEFAULT_CONFIG_FILENAME,
    });

    for a in args.iter() {
        let config_pref = "--config=";
        if a.starts_with(config_pref) {
            let config_file_string = &a[config_pref.chars().count()..];
            println!("parsing the config file from {}", config_file_string);

            config.config_filename = &config_file_string;
        } else {
            println!("argument not recognized skipping {}", a);
        }
    }

    return config;
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    commands: Vec<Command>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Command {
    out_color: String,
    bin: String,
    args: Vec<String>,
    follow: bool,
}

struct CommandCtx<'a> {
    command: &'a Command,
    child_out_file: File,
    pid: Pid,
}

fn parse_config(config_fname: &str) -> Result<Config, Box<dyn Error>> {
    println!("opening config file {}", config_fname);

    let reader = File::open(config_fname).expect("Unable to parse config file");
    let config_result: Result<Config, serde_yaml::Error> = serde_yaml::from_reader(reader);

    match config_result {
        Ok(config) => Ok(config),
        Err(_) => panic!("Error parsing config file"),
    }
}

fn listen_to_commands(mut commands: Vec<Box<CommandCtx>>) {
    loop {
        for ctx in &mut commands {
            let mut buf = [0; 1024];
            let len = ctx.child_out_file.read(&mut buf).unwrap_or_default();

            println!("reading {} : {:?}", len, ctx.pid);

            if len > 0 {
                println!("using color {}", &ctx.command.out_color);
                print!(
                    "\x1b[{}m{}\x1b[0m",
                    get_color(&ctx.command.out_color),
                    str::from_utf8(&buf).unwrap()
                );
            }

            if ctx.command.follow {
                let status = wait::waitpid(ctx.pid, Some(WaitPidFlag::WNOHANG)).unwrap();
                match status {
                    WaitStatus::Exited(_child, _) => break,
                    WaitStatus::Signaled(_child, _, _) => break,
                    WaitStatus::Stopped(_child, _) => break,
                    _ => (),
                }
            }
            sleep(time::Duration::from_millis(1000));
        }
    }
}

fn launch_command<'a>(command: &'a Command) -> Option<Box<CommandCtx>> {
    let cstring_filename = CString::new(command.bin.clone()).unwrap();
    let cmdname = cstring_filename.as_c_str();

    let mut argsvec = Vec::<CString>::new();

    argsvec.push(cstring_filename.clone());
    let argiter = command.args.clone().into_iter();
    argiter.for_each(|s| argsvec.push(CString::new(s).unwrap()));

    let (from_child_fd, to_parent) = pipe().unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("yay we are the parent, process is {}", child);

            unsafe {
                fcntl(from_child_fd, libc::F_SETFL, libc::O_NONBLOCK);

                let from_child = File::from_raw_fd(from_child_fd);

                let ctx = Box::new(CommandCtx {
                    command: &command,
                    pid: child,
                    child_out_file: from_child,
                });
                return Some(ctx);
            }
        }
        Ok(ForkResult::Child) => {
            println!("copyout out things");
            dup2(to_parent, 1).unwrap();
            execvp(cmdname, &argsvec[..])
                .map_err(|err| println!("error from execvp {:?}", err))
                .ok();
            None
        }
        Err(_) => {
            println!("Error fork failed");
            None
        }
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let opts = parse_incoming_command_line(&args);
    dbg!(opts.config_filename);

    match parse_config(&opts.config_filename) {
        Ok(config) => {
            let mut running_commands = Vec::<Box<CommandCtx>>::with_capacity(config.commands.len());
            for cmd in &config.commands {
                if let Some(ctx) = launch_command(&cmd) {
                    println!("Found a command: {}", cmd.bin);
                    running_commands.push(ctx)
                }
            }
            listen_to_commands(running_commands);
        }
        Err(_) => panic!("Error parsing command file"),
    }

    return Ok(());
}
