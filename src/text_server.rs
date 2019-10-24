use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
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
	OpenResp(bool),
	WriteReq(WriteReqData),
	WriteResp,
	ReadReq(ReadReqData),
	ReadResp(Vec<u8>),
}

struct FileState {
	rope: Rope,
	clients: HashSet<ThreadId>,
}

struct ThreadShared {
	reader: BufReader<TcpStream>,
	writer: BufWriter<TcpStream>,
}

struct ThreadState {
	thread_id: ThreadId,
	thread_shared: Arc<Mutex<HashMap<ThreadId, ThreadShared>>>,
	files: Arc<RwLock<HashMap<PathBuf, FileState>>>,
	canonical_home: PathBuf,
	current_file_loc: Option<PathBuf>,
}

fn open_file(thread_local: &mut ThreadState, path: &str) -> Result<PathBuf, Box<dyn Error>> {
	let path = Path::new(path);
	let canonical_path = path.canonicalize()?;

	// Check that path is valid given client home
	if !canonical_path.starts_with(&thread_local.canonical_home) {
		Err("Invalid file path")?
	}

	// Make sure the files hashmap contains this file
	if !thread_local
		.files
		.read()
		.or(Err("Could not read lock file map"))?
		.contains_key(&canonical_path)
	{
		// Read file
		let mut buffer = Vec::new();
		let mut file = File::open(&canonical_path)?;
		file.read_to_end(&mut buffer)?;

		// Add to rope
		let rope = Rope::new();
		rope.insert_at(0, &buffer);

		thread_local
			.files
			.write()
			.or(Err("Could not write lock file map"))?
			.insert(
				canonical_path.clone(),
				FileState {
					rope,
					clients: HashSet::new(),
				},
			);
	}

	// Add bookkeeping
	thread_local
		.files
		.write()
		.or(Err("Could not write lock file map"))?
		.get_mut(&canonical_path)
		.ok_or("Thread local storage does not exist")?
		.clients
		.insert(thread_local.thread_id);

	thread_local.current_file_loc = Some(canonical_path.clone());

	Ok(canonical_path)
}

// Takes a message and the current client's state, processes it, and returns a message to reply with
fn process_message(thread_local: &mut ThreadState, msg: Message) -> (Message, bool) {
	match msg {
		Message::Echo(inner) => (Message::Echo(inner), false),
		Message::OpenReq(inner) => match open_file(thread_local, &inner) {
			Ok(_) => (Message::OpenResp(true), false),
			Err(_) => (Message::OpenResp(false), false),
		},
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
fn client_thread(thread_local: &mut ThreadState) -> Result<(), Box<dyn Error>> {
	let mut buffer = [0u8; MAX_MESSAGE];
	loop {
		let num_read = thread_local
			.thread_shared
			.lock()
			.or(Err("Unable to lock thread shared data"))?
			.get_mut(&thread_local.thread_id)
			.ok_or("Thread local storage does not exist")?
			.reader
			.read(&mut buffer)?;

		// Check for a EOF
		if num_read == 0 {
			break;
		}

		let msg: Message = serde_json::from_slice(&buffer[..num_read])?;
		let (response, exit) = process_message(thread_local, msg);
		let response_raw = serde_json::to_vec(&response)?;
		let num_written = thread_local
			.thread_shared
			.lock()
			.or(Err("Unable to lock thread shared data"))?
			.get_mut(&thread_local.thread_id)
			.ok_or("Thread local storage does not exist")?
			.writer
			.write(&response_raw)?;

		// Check for a EOF
		if num_written == 0 {
			break;
		}
		// thread_local
		// 	.thread_shared
		// 	.lock()
		// 	.or(Err("Unable to lock thread shared data"))?
		// 	.get_mut(&thread_local.thread_id)
		// 	.ok_or("Thread local storage does not exist")?
		// 	.writer
		// 	.flush()?;
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

	let thread_shared: Arc<Mutex<HashMap<ThreadId, ThreadShared>>> =
		Arc::new(Mutex::new(HashMap::new()));

	for stream_result in listener.incoming() {
		let canonical_home = canonical_home.clone();
		let files = files.clone();
		let thread_shared = thread_shared.clone();

		spawn(move || {
			let stream = stream_result.unwrap();

			let mut thread_local = ThreadState {
				thread_id: current().id(),
				thread_shared,
				files,
				canonical_home,
				current_file_loc: None,
			};

			thread_local.thread_shared.lock().unwrap().insert(
				thread_local.thread_id,
				ThreadShared {
					reader: BufReader::new(stream.try_clone().unwrap()),
					// writer: BufWriter::new(stream.try_clone().unwrap()),
					writer: BufWriter::with_capacity(0, stream.try_clone().unwrap()),
				},
			);

			client_thread(&mut thread_local).unwrap();

			thread_local
				.thread_shared
				.lock()
				.unwrap()
				.remove(&thread_local.thread_id);
		});
	}

	Ok(())
}
