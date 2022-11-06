use std::path::PathBuf;

use clap::{Arg, Command, value_parser};

use home_space::{hello, commands::import::import_command};

fn main() {
    let matches = Command::new(env!("CARGO_CRATE_NAME"))
    .version(env!("CARGO_PKG_VERSION"))
    .about("Home space CLI")
    .subcommand_required(true)
    .subcommand(
        Command::new("import")
        .about("Import external location.")
        .arg(Arg::new("source")
            .short('s')
            .long("source")
            .required(true)
            .value_parser(value_parser!(PathBuf))
            .help("Required. Location to import."))
        .arg(Arg::new("user")
            .short('u')
            .long("user")
            .required(true)
            .help("Required. User name"))
        .arg(Arg::new("destination")
            .short('d')
            .long("destination")
            .required(false)
            .default_value("/")
            .help("Parent location, if not provided import into root."))
    ).get_matches();

    match matches.subcommand() {
        Some((_, import_matches)) => {
            let source = import_matches.get_one::<PathBuf>("source")
                .expect("Source is required");
            let user = import_matches.get_one::<String>("user")
                .expect("User is required");
            let destination = import_matches.get_one::<String>("destination")
                .expect("Destination is required");
            import_command(source, user, destination);
        },
        None => {
            println!("No command provided.");
            std::process::exit(0);
        },
    }

    hello();
}
