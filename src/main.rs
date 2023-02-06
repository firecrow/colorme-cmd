use serde_derive::{self, Deserialize, Serialize};
use serde_yaml;
use std::env;
use std::error::Error;
use std::fs::File;

#[path = "command.rs"]
mod command;

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
    commands: Vec<command::Command>,
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

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let opts = parse_incoming_command_line(&args);
    dbg!(opts.config_filename);

    match parse_config(&opts.config_filename) {
        Ok(config) => {
            let mut running_commands =
                Vec::<Box<command::CommandCtx>>::with_capacity(config.commands.len());
            for cmd in &config.commands {
                if let Some(ctx) = command::launch_command(&cmd) {
                    println!("Found a command: {}", cmd.bin);
                    running_commands.push(ctx)
                }
            }
            command::listen_to_commands(running_commands);
        }
        Err(_) => panic!("Error parsing command file"),
    }

    return Ok(());
}
