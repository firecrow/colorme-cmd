use libc::fcntl;
use maplit::hashmap;
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
    // the path to the executable to run
    pub bin: String,
    // the command line arguments sent to the executable
    pub args: Vec<String>,
    // the color of the log output to the console
    out_color: String,
    // weather to only show lines containing a specific regex
    filter: Option<String>,
}

pub struct CommandCtx<'a> {
    // command struct of the definition of how/display/filter to run the commands output
    command: &'a Command,
    // file handle for the childs output
    child_out_file: File,
    // not yet shown content, may be a partial line
    shelf: String,
    // the regex object used to decide weather to output the line or not
    regex: Option<Regex>,
}

/**
 * Launch the command, by forking and then replacing the process image with
 * exec, this is done manually so that we can listen with non-blocking io
 * cooperatively to multiple processes
 */
pub fn launch_command<'a>(cmd: &'a Command) -> Option<Box<CommandCtx>> {
    let cstring_filename = CString::new(cmd.bin.clone()).unwrap();
    let cmdname = cstring_filename.as_c_str();

    let mut argsvec = Vec::<CString>::new();

    argsvec.push(cstring_filename.clone());
    let argiter = cmd.args.clone().into_iter();
    argiter.for_each(|s| argsvec.push(CString::new(s).unwrap()));

    let (from_child_fd, to_parent) = pipe().unwrap();

    match unsafe { fork() } {
        Ok(ForkResult::Parent { .. }) => unsafe {
            fcntl(from_child_fd, libc::F_SETFL, libc::O_NONBLOCK);

            let from_child = File::from_raw_fd(from_child_fd);

            let mut re: Option<Regex> = None;
            if let Some(regex_str) = &cmd.filter {
                re = Some(Regex::new(regex_str.as_str()).unwrap());
            }

            // Create an object that has all the information we will need about
            // listening to the process later
            let ctx = Box::new(CommandCtx {
                command: &cmd,
                child_out_file: from_child,
                shelf: "".to_string(),
                regex: re,
            });
            return Some(ctx);
        },
        Ok(ForkResult::Child) => {
            // redirect stdout of this process into a pipe so that the parent
            // process can read from it and, filter/color it
            dup2(to_parent, 1).unwrap();

            // span the process, replacing this process's image
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

/**
 * This function cycles through the running processes, and reads for output that
 * is sent to the file descriptor associated with the process(s) we spawned
 *
 * once read, the oupt is shown if it matches a filter regex, and colored
 * according to the configuration specified in the config
 */
pub fn listen_to_commands(mut commands: Vec<Box<CommandCtx>>) {
    loop {
        for ctx in &mut commands {
            // read up to 1024 bytes from the pipe, this will return 0 if there
            // is nothing read because of the fcntl flag set earlier,
            let mut buf = [0; 1024];
            let len = ctx.child_out_file.read(&mut buf).unwrap_or_default();

            if len > 0 {
                let content = str::from_utf8(&buf).unwrap();

                // add the content to what we have already found
                ctx.shelf += content;
                // if it has at least one line
                if ctx.shelf.contains("\n") {
                    // print out the lines if they match the regex filter, or no filter is set
                    for line in ctx.shelf.split("\n") {
                        if ctx.regex.is_none() || ctx.regex.as_ref().unwrap().is_match(line) {
                            print!(
                                "\x1b[{}m{}\x1b[0m\n",
                                get_color(&ctx.command.out_color),
                                line
                            );
                        }
                    }
                    // reset the content once it has been displayed
                    ctx.shelf = "".to_string();
                }
            }

            // throttle so that it does not consume more resources than would be visible
            sleep(time::Duration::from_millis(100));
        }
    }
}
