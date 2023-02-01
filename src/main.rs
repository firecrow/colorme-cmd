use nix::unistd::execvp;
use std::env;
use std::ffi::CString;

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(args);

    let cmd = CString::new("ls").unwrap();
    let dir = CString::new(".").unwrap();

    execvp(&cmd, &[&cmd, &dir])
        .map_err(|err| println!("error from execvp {:?}", err))
        .ok();
}
