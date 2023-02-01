use ferris_says::say;
use std::io::{stdout, BufWriter};

fn main() {
    println!("yay I am rustiful too!");
    let stdout = stdout();
    let msg = String::from("hello objects");
    let len = msg.chars().count();

    let mut writer = BufWriter::new(stdout.lock());
    say(msg.as_bytes(), len, &mut writer).unwrap();
}
