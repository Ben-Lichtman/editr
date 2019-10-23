use std::collections::HashMap;
use std::error;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread;

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
	OpenResp(String),
	WriteReq(WriteReqData),
	WriteResp,
	ReadReq(ReadReqData),
	ReadResp(Vec<u8>),
}

struct FileState {
	rope: Rope,
}

struct ClientState<'a> {
	reader: BufReader<&'a TcpStream>,
	writer: BufWriter<&'a TcpStream>,
	canonical_home: PathBuf,
	path: Option<PathBuf>,
	files: Arc<RwLock<HashMap<PathBuf, FileState>>>,
}

// Takes a message and the current client's state, processes it, and returns a message to reply with
fn process_message(state: &mut ClientState, msg: Message) -> (Message, bool) {
	match msg {
		Message::Echo(inner) => (Message::Echo(inner), false),
		Message::OpenReq(inner) => {
			let result = open_file(state, &inner);
			let mut response_body = String::new();
			if let Err(e) = result {
				response_body.push_str(&e.to_string());
			}
			(Message::OpenResp(response_body), false)
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

fn open_file(state: &mut ClientState, path: &str) -> Result<(), Box<dyn error::Error>> {
	let path = Path::new(&path);
	if !is_valid_path(path) {
		return Err(Box::new(io::Error::new(
			ErrorKind::PermissionDenied,
			"Path is out of bounds",
		)));
	}
	else {
		// Acquire write lock on state.files
		let mut ropes = state.files.write().unwrap();

		let path = path.canonicalize()?;
		if !ropes.contains_key(&path) {
			let mut file = File::open(&path)?;
			let mut buffer = Vec::new();
			file.read_to_end(&mut buffer)?;

			let new_rope = Rope::new();
			new_rope.insert_at(0, buffer.as_slice())?;
			ropes.insert(path.clone(), FileState { rope: new_rope });
			state.path = Some(path);
		}
	}
	Ok(())
}

// Returns true if path is within the bounds of editr's root
// i.e. no access with respect to filesystem root or parent
fn is_valid_path(path: &Path) -> bool {
	let mut components = path.components();
	match components.next() {
		Some(Component::RootDir) => false,
		Some(Component::ParentDir) => false,
		_ => true,
	}
}

// The main function run by the client thread
fn client_thread(mut state: ClientState) -> Result<(), Box<dyn error::Error>> {
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
		if exit {
			// Client has finished connection
			break;
		}
	}
	Ok(())
}

pub fn start<A: ToSocketAddrs>(path: &Path, address: A) -> Result<(), Box<dyn error::Error>> {
	let canonical_home = path.canonicalize()?;

	let listener = TcpListener::bind(address)?;

	let files: Arc<RwLock<HashMap<PathBuf, FileState>>> = Arc::new(RwLock::new(HashMap::new()));

	for stream_result in listener.incoming() {
		let canonical_home = canonical_home.clone();
		let files = Arc::clone(&files);
		thread::spawn(move || {
			let stream = match stream_result {
				Ok(s) => s,
				Err(_) => return,
			};
			let state = ClientState {
				reader: BufReader::new(&stream),
				writer: BufWriter::new(&stream),
				canonical_home,
				files,
				path: None,
			};
			client_thread(state).ok();
		});
	}

	Ok(())
}
