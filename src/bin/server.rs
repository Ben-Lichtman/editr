use std::env;
use std::path::PathBuf;
use std::net::SocketAddr;

use editr::text_server;

fn main() {
	let args: Vec<String> = env::args().collect();
	match Config::new(args) {
		Ok(config) => {
			text_server::start(&config.home, config.address).unwrap();
		}
		Err(e) => {
			println!("Error parsing arguments...");
			println!("\t{}", e.to_string());
			print_help();
		}
	}
}

fn print_help() {
	println!("usage: server <home> <address>")
}

struct Config {
	home: PathBuf,
	address: SocketAddr,
}

impl Config {
	fn new(args: Vec<String>) -> Result<Config, &'static str>{
		const NUM_ARGS: usize = 2;
		if args.len() == NUM_ARGS + 1 {
			let home = PathBuf::from(&args[1]);
			if !home.exists() {
				return Err("Path does not exist")
			}
			else if !home.is_dir() {
				return Err("Path is not a directory")
			}

			let address = args[2].parse::<SocketAddr>()
							.map_err(|_|
								"Address is invalid"
							)?;

			Ok(Config {home, address})
		}
		else {
			Err("Wrong number of arguments given")
		}
	}
}
