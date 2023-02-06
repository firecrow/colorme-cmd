use libc::fcntl;
use maplit::hashmap;
use nix::sys::wait::{self, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use nix::unistd::{dup2, execvp, fork, pipe, ForkResult};
use serde_derive::{self, Deserialize, Serialize};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::str;
use std::thread::sleep;
use std::time;

use regex::Regex;

#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    pub bin: String,
    pub args: Vec<String>,
    out_color: String,
    follow: bool,
    filter: Option<String>,
}

pub struct CommandCtx<'a> {
    command: &'a Command,
    child_out_file: File,
    pid: Pid,
    shelf: String,
    regex: Option<Regex>,
}

pub fn launch_command<'a>(cmd: &'a Command) -> Option<Box<CommandCtx>> {
    let cstring_filename = CString::new(cmd.bin.clone()).unwrap();
    let cmdname = cstring_filename.as_c_str();

    let mut argsvec = Vec::<CString>::new();

    argsvec.push(cstring_filename.clone());
    let argiter = cmd.args.clone().into_iter();
    argiter.for_each(|s| argsvec.push(CString::new(s).unwrap()));

    let (from_child_fd, to_parent) = pipe().unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("yay we are the parent, process is {}", child);

            unsafe {
                fcntl(from_child_fd, libc::F_SETFL, libc::O_NONBLOCK);

                let from_child = File::from_raw_fd(from_child_fd);

                let mut re: Option<Regex> = None;
                if let Some(regex_str) = &cmd.filter {
                    re = Some(Regex::new(regex_str.as_str()).unwrap());
                }

                let ctx = Box::new(CommandCtx {
                    command: &cmd,
                    pid: child,
                    child_out_file: from_child,
                    shelf: "".to_string(),
                    regex: re,
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

pub fn listen_to_commands(mut commands: Vec<Box<CommandCtx>>) {
    loop {
        for ctx in &mut commands {
            let mut buf = [0; 1024];
            let len = ctx.child_out_file.read(&mut buf).unwrap_or_default();

            if len > 0 {
                let content = str::from_utf8(&buf).unwrap();

                ctx.shelf += content;
                if ctx.shelf.contains("\n") {
                    for line in ctx.shelf.split("\n") {
                        if ctx.regex.is_none() || ctx.regex.as_ref().unwrap().is_match(line) {
                            print!(
                                "\x1b[{}m{}\x1b[0m\n",
                                get_color(&ctx.command.out_color),
                                line
                            );
                        }
                    }
                    ctx.shelf = "".to_string();
                }
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
            // throttle so that it does not consume more resources than would be visible
            sleep(time::Duration::from_millis(100));
        }
    }
}
