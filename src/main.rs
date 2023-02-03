#![allow(dead_code)]

use arrayvec::ArrayVec;
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

fn get_color(color: &str) -> i32 {
    let color_map = hashmap! {
        "default".to_string() => DEFAULT_COLOR,
        "red".to_string() => RED,
        "green".to_string() => GREEN,
        "yellow".to_string() => YELLOW,
    };
    return color_map[color];
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

    let reader = File::open(config_fname)?;
    let config: Config =
        serde_yaml::from_reader(reader).expect("Config file of the expected commands format");

    return Ok(config);
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let opts = parse_incoming_command_line(&args);
    dbg!(opts.config_filename);

    let config = parse_config(&opts.config_filename);
    dbg!(config.unwrap());
    /*

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
                fcntl(from_child_fd, libc::F_SETFL, libc::O_NONBLOCK);

                let mut from_child = File::from_raw_fd(from_child_fd);

                let mut buf = [0; 1024];
                let mut len = Read::read(&mut from_child, &mut buf).unwrap_or_default();

                loop {
                    if len > 0 {
                        print!(
                            "\x1b[{}m{}\x1b[0m",
                            cmd.out_color,
                            str::from_utf8(&buf).unwrap()
                        );
                    } else {
                        println!("\x1b[36m0\x1b[0m");
                    }
                    len = Read::read(&mut from_child, &mut buf).unwrap_or_default();

                    let status = wait::waitpid(child, Some(WaitPidFlag::WNOHANG)).unwrap();

                    match status {
                        WaitStatus::Exited(_child, _) => break,
                        WaitStatus::Signaled(_child, _, _) => break,
                        WaitStatus::Stopped(_child, _) => break,
                        _ => println!("still rockin"),
                    }

                    sleep(time::Duration::from_millis(100));
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
    */
    return Ok(());
}
