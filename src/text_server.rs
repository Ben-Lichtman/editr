use std::collections::HashMap;
use std::error::Error;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread::spawn;

use serde::{Deserialize, Serialize};
use serde_json;

use crate::rope::Rope;

const MAX_MESSAGE: usize = 1024;

#[derive(Serialize, Deserialize)]
struct WriteReqData {
	offset: usize,
	data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct ReadReqData {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize)]
enum Message {
	Invalid,
	Echo(Vec<u8>),
	OpenReq(String),
	OpenResp,
	WriteReq(WriteReqData),
	WriteResp,
	ReadReq(ReadReqData),
	ReadResp(Vec<u8>),
}

struct ClientState<'a> {
	reader: BufReader<&'a TcpStream>,
	writer: BufWriter<&'a TcpStream>,
	canonical_home: PathBuf,
	files: Arc<RwLock<HashMap<PathBuf, Rope>>>,
}

// Takes a message and the current client's state, processes it, and returns a message to reply with
fn process_message(state: &mut ClientState, msg: Message) -> (Message, bool) {
	match msg {
		Message::Echo(inner) => (Message::Echo(inner), false),
		Message::OpenReq(inner) => {
			// TODO Do open
			(Message::OpenResp, false)
		}
		Message::WriteReq(inner) => {
			// TODO Do write
			(Message::WriteResp, false)
		}
		Message::ReadReq(inner) => {
			// TODO Do read
			let resp_data = Vec::new();
			(Message::ReadResp(resp_data), false)
		}
		_ => (Message::Invalid, false),
	}
}

// The main function run by the client thread
fn client_thread(mut state: ClientState) -> Result<(), Box<dyn Error>> {
	let mut buffer = [0u8; MAX_MESSAGE];
	loop {
		let num_read = state.reader.read(&mut buffer)?;

		// Check for a EOF
		if num_read == 0 {
			break;
		}
		let msg: Message = serde_json::from_slice(&buffer[..num_read])?;
		let (response, exit) = process_message(&mut state, msg);
		let response_raw = serde_json::to_vec(&response)?;
		let num_written = state.writer.write(&response_raw)?;

		// Check for a EOF
		if num_written == 0 {
			break;
		}
		state.writer.flush()?;
		// Client has finished connection
		if exit {
			break;
		}
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
				Err(_) => return,
			};
			let state = ClientState {
				reader: BufReader::new(&stream),
				writer: BufWriter::new(&stream),
				canonical_home,
				files,
			};
			client_thread(state).ok();
		});
	}

	Ok(())
}
