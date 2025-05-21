use clap::{command, Arg, ArgMatches, Command};
use owo_colors::OwoColorize;

mod service;

fn args() -> ArgMatches{
    let cmd = command!()
    .subcommand(Command::new("throw").about("Throw a shuriken (start a service)")
    .arg(Arg::new("shuriken").required(true)))
    .subcommand(Command::new("recall").about("Recall a shuriken (stop a service)")
    .arg(Arg::new("shuriken").required(true)))
    .subcommand(Command::new("trace").about("Trace a shuriken (check if a service is running)")
    .arg(Arg::new("shuriken").required(true)));

    cmd.get_matches()
}

fn main() {
    let args = args();

    match args.subcommand() {
        Some(("throw", shuriken)) => println!("{}", format!("Throwing shuriken {}...", shuriken.get_one::<String>("shuriken").expect("idk").green()).bold()),
        Some(("recall", shuriken)) => println!("{}", format!("Recalling shuriken {}...", shuriken.get_one::<String>("shuriken").expect("idk").green()).bold()),
        Some(("trace", shuriken)) => println!("{}", format!("Shuriken {}{}{}{}", shuriken.get_one::<String>("shuriken").expect("idk").yellow(), " is running with access to port: ".green(), "5173".magenta(), ".".green()).green().bold()),
        _ => println!("{}", "Invalid action".red()),
    }
}