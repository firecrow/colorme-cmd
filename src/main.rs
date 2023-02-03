#![allow(dead_code)]

use libc::fcntl;
use maplit::hashmap;
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::{dup2, execvp, fork, pipe, ForkResult};
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

fn get_color(color: &String) -> i32 {
    let color_map = hashmap! {
        "default" => DEFAULT_COLOR,
        "red" => RED,
        "green" => GREEN,
        "yellow" => YELLOW,
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
    err_color: String,
    bin: String,
    args: Vec<String>,
    follow: bool,
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

struct RunningProcess {
    command: Command,
    out: File,
    status: Option<i32>,
}

fn launch_command(command: Command) {
    let cstring_filename = CString::new(command.bin).unwrap();
    let cmdname = cstring_filename.as_c_str();

    let mut argsvec = Vec::<CString>::new();

    argsvec.push(cstring_filename.clone());
    let argiter = command.args.into_iter();
    argiter.for_each(|s| argsvec.push(CString::new(s).unwrap()));

    let (from_child_fd, to_parent) = pipe().unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("yay we are the parent, process is {}", child);

            unsafe {
                fcntl(from_child_fd, libc::F_SETFL, libc::O_NONBLOCK);

                let mut from_child = File::from_raw_fd(from_child_fd);

                let mut buf = [0; 1024];
                let mut len = Read::read(&mut from_child, &mut buf).unwrap_or_default();

                loop {
                    if len > 0 {
                        print!(
                            "\x1b[{}m{}\x1b[0m",
                            get_color(&command.out_color),
                            str::from_utf8(&buf).unwrap()
                        );
                    }
                    len = Read::read(&mut from_child, &mut buf).unwrap_or_default();

                    let status = wait::waitpid(child, Some(WaitPidFlag::WNOHANG)).unwrap();

                    match status {
                        WaitStatus::Exited(_child, _) => break,
                        WaitStatus::Signaled(_child, _, _) => break,
                        WaitStatus::Stopped(_child, _) => break,
                        _ => (),
                    }

                    sleep(time::Duration::from_millis(100));
                }
            };
        }
        Ok(ForkResult::Child) => {
            dup2(to_parent, 1).unwrap();
            execvp(cmdname, &argsvec[..])
                .map_err(|err| println!("error from execvp {:?}", err))
                .ok();
        }
        Err(_) => println!("Error fork failed"),
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let opts = parse_incoming_command_line(&args);
    dbg!(opts.config_filename);

    match parse_config(&opts.config_filename) {
        Ok(config) => {
            for cmd in config.commands {
                println!("Found a command: {}", cmd.bin);
                launch_command(cmd);
            }
        }
        Err(_) => panic!("Error parsing command file"),
    }

    return Ok(());
}
