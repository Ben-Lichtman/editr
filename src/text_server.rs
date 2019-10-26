use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{current, spawn, ThreadId};

use serde_json;

use crate::message::{process_message, Message};
use crate::rope::Rope;
use crate::state::{FileState, ThreadShared, ThreadState};

const MAX_MESSAGE: usize = 1024;

// The main function run by the client thread
fn client_thread(thread_local: &mut ThreadState) -> Result<(), Box<dyn Error>> {
	let mut buffer = [0u8; MAX_MESSAGE];
	loop {
		let num_read = thread_local
			.thread_shared
			.read()
			.or(Err("Could not get read lock"))?
			.get(&thread_local.thread_id)
			.ok_or("Thread local storage does not exist")?
			.lock()
			.or(Err("Unable to lock thread shared data"))?
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
			.read()
			.or(Err("Could not get read lock"))?
			.get(&thread_local.thread_id)
			.ok_or("Thread local storage does not exist")?
			.lock()
			.or(Err("Unable to lock thread shared data"))?
			.writer
			.write(&response_raw)?;

		// Check for a EOF
		if num_written == 0 {
			break;
		}

		// thread_local
		// 	.thread_shared
		// 	.get(&thread_local.thread_id)
		// 	.ok_or("Thread local storage does not exist")?
		// 	.lock()
		// 	.or(Err("Unable to lock thread shared data"))?
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

	let thread_shared: Arc<RwLock<HashMap<ThreadId, Mutex<ThreadShared>>>> =
		Arc::new(RwLock::new(HashMap::new()));

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

			thread_local.thread_shared.write().unwrap().insert(
				thread_local.thread_id,
				Mutex::new(ThreadShared {
					reader: BufReader::new(stream.try_clone().unwrap()),
					// writer: BufWriter::new(stream.try_clone().unwrap()),
					writer: BufWriter::with_capacity(0, stream.try_clone().unwrap()),
				}),
			);

			client_thread(&mut thread_local).unwrap();

			thread_local
				.thread_shared
				.write()
				.unwrap()
				.remove(&thread_local.thread_id);
		});
	}

	Ok(())
}
