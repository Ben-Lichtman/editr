use std::collections::HashMap;
use std::error::Error;
use std::net::{TcpListener, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread::spawn;

use crate::message::Message;
use crate::state::{shared_io_container::SharedIOContainer, FileState, ThreadState};

const MAX_MESSAGE: usize = 1024;

// The main function run by the client thread
fn client_thread(thread_local: &mut ThreadState) -> Result<(), Box<dyn Error>> {
	let mut buffer = [0u8; MAX_MESSAGE];
	loop {
		let num_read = thread_local.socket_read(&mut buffer)?;

		// Check for a EOF
		if num_read == 0 {
			break;
		}

		let msg = Message::from_slice(&buffer[..num_read])?;

		let (response, exit) = msg.process(thread_local);

		let response_raw = response.to_vec()?;

		let num_written = thread_local.socket_write(&response_raw)?;

		// Check for a EOF
		if num_written == 0 {
			break;
		}

		// thread_local
		//	.thread_io
		//	.get(&thread_local.thread_id)
		//	.ok_or("Thread local storage does not exist")?
		//	.lock()
		//	.or(Err("Unable to lock thread shared data"))?
		//	.writer
		//	.flush()?;

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

	let threads_io: Arc<SharedIOContainer> = Arc::new(SharedIOContainer::new());

	for stream_result in listener.incoming() {
		let canonical_home = canonical_home.clone();
		let files = files.clone();
		let threads_io = Arc::clone(&threads_io);

		spawn(move || {
			let stream = stream_result.unwrap();

			let mut thread_local = ThreadState::new(threads_io, files, canonical_home);
			thread_local.insert_thread_io(stream).unwrap();

			client_thread(&mut thread_local).unwrap();

			// Close file
			thread_local.file_close().unwrap();

			// Remove io
			thread_local.remove_thread_io().unwrap();
		});
	}

	Ok(())
}
