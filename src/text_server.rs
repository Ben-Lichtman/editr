use std::error::Error;
use std::path::{Path, PathBuf};
use std::net::{ToSocketAddrs, TcpListener, TcpStream};
use std::thread::spawn;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::io::{BufReader, BufWriter, Read, Write};

use serde::{Serialize, Deserialize};
use serde_json;

use crate::rope::Rope;

const MAX_MESSAGE: usize = 1024;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
	s: String,
}

struct ClientState<'a> {
	reader: BufReader<&'a TcpStream>,
	writer: BufWriter<&'a TcpStream>,
	canonical_home: PathBuf,
	files: Arc<RwLock<HashMap<PathBuf, Rope>>>,
}

fn print_rope(r: &Rope, from: usize, to: usize) {
	let c = r.collect(from, to).unwrap();
	println!("{:?}", std::str::from_utf8(&c).unwrap());
}

fn process_message(state: &mut ClientState, msg: Message) -> (Message, bool) {
	(Message {
		s: String::from("hello world"),
	}, false)
}

fn client_thread(mut state: ClientState) -> Result<(), Box<dyn Error>> {
	let mut buffer = [0u8; MAX_MESSAGE];
	loop {
		let num_read = state.reader.read(&mut buffer)?;
		if num_read == 0 { break }
		let msg: Message = serde_json::from_slice(&buffer[..num_read])?;
		let (response, exit) = process_message(&mut state, msg);
		let response_raw = serde_json::to_vec(&response)?;
		let num_written = state.writer.write(&response_raw)?;
		if num_written == 0 { break }
		state.writer.flush()?;
		if exit == true { break }
	}
	Ok(())
}

pub fn start<A: ToSocketAddrs>(path: &Path, address: A) -> Result<(), Box<dyn Error>> {

	let canonical_home = path.canonicalize()?;

	let listener = TcpListener::bind(address)?;

	let files: Arc<RwLock<HashMap<PathBuf, Rope>>> = Arc::new(RwLock::new(HashMap::new()));

	for stream_result in listener.incoming() {
		let canonical_home = canonical_home.clone();
		let files = files.clone();
		spawn(move || {
			let stream = match stream_result {
				Ok(s) => s,
				_ => return,
			};
			let state = ClientState {
				reader: BufReader::new(&stream),
				writer: BufWriter::new(&stream),
				canonical_home: canonical_home,
				files: files,
			};
			client_thread(state).ok();
		});
    }

	Ok(())
}
