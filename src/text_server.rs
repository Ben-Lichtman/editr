use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{current, spawn, ThreadId};

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

struct FileState {
	rope: Rope,
	clients: HashSet<ThreadId>,
}

struct ClientState {
	reader: BufReader<TcpStream>,
	writer: BufWriter<TcpStream>,
	canonical_home: PathBuf,
	current_file_loc: Option<PathBuf>,
	current_file: Option<Rope>,
}

// Takes a message and the current client's state, processes it, and returns a message to reply with
fn process_message(
	thread_data: &Arc<Mutex<HashMap<ThreadId, ClientState>>>,
	msg: Message,
) -> (Message, bool) {
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
fn client_thread(
	thread_id: ThreadId,
	thread_data: Arc<Mutex<HashMap<ThreadId, ClientState>>>,
	files: Arc<RwLock<HashMap<PathBuf, FileState>>>,
) -> Result<(), Box<dyn Error>> {
	let mut buffer = [0u8; MAX_MESSAGE];
	loop {
		let num_read = thread_data
			.lock()
			.map_err(|e| "Unable to lock thread data")?
			.get_mut(&thread_id)
			.ok_or("Thread local storage does not exist")?
			.reader
			.read(&mut buffer)?;

		// Check for a EOF
		if num_read == 0 {
			break;
		}

		let msg: Message = serde_json::from_slice(&buffer[..num_read])?;
		let (response, exit) = process_message(&thread_data, msg);
		let response_raw = serde_json::to_vec(&response)?;
		let num_written = thread_data
			.lock()
			.map_err(|e| "Unable to lock thread data")?
			.get_mut(&thread_id)
			.ok_or("Thread local storage does not exist")?
			.writer
			.write(&response_raw)?;

		// Check for a EOF
		if num_written == 0 {
			break;
		}
		thread_data
			.lock()
			.map_err(|e| "Unable to lock thread data")?
			.get_mut(&thread_id)
			.ok_or("Thread local storage does not exist")?
			.writer
			.flush()?;
		if exit {
			// Client has finished connection
			break;
		}
	}
	Ok(())
}

pub fn start<A: ToSocketAddrs>(path: &Path, address: A) -> Result<(), Box<dyn Error>> {
	let canonical_home = path.canonicalize()?;

	let listener = TcpListener::bind(address)?;

	let files: Arc<RwLock<HashMap<PathBuf, FileState>>> = Arc::new(RwLock::new(HashMap::new()));

	let thread_data: Arc<Mutex<HashMap<ThreadId, ClientState>>> =
		Arc::new(Mutex::new(HashMap::new()));

	for stream_result in listener.incoming() {
		let canonical_home = canonical_home.clone();
		let files = files.clone();
		let thread_data = thread_data.clone();

		spawn(move || {
			let stream = stream_result.unwrap();

			let thread_id = current().id();

			thread_data.lock().unwrap().insert(
				thread_id,
				ClientState {
					reader: BufReader::new(stream.try_clone().unwrap()),
					writer: BufWriter::new(stream.try_clone().unwrap()),
					canonical_home,
					current_file_loc: None,
					current_file: None,
				},
			);

			client_thread(thread_id, thread_data.clone(), files).unwrap();

			thread_data.lock().unwrap().remove(&thread_id);
		});
	}

	Ok(())
}
