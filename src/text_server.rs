use std::error::Error;
use std::net::{TcpListener, ToSocketAddrs};
use std::path::Path;
use std::thread::spawn;

use crate::message::Message;
use crate::state::*;

// The main function run by the client thread
fn client_thread(mut thread_local: &mut LocalState) -> Result<(), Box<dyn Error>> {
	loop {
		let msg = Message::from_reader(&mut thread_local)?;

		println!("<=: {:?}", msg);

		let (response, exit) = msg.process(thread_local);

		println!("=>: {:?}", response);

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

pub fn start<A: ToSocketAddrs>(path: &str, address: A) -> Result<(), Box<dyn Error>> {
	let canonical_home = Path::new(path).canonicalize()?;

	let listener = TcpListener::bind(address)?;

	let files: FileStates = FileStates::new();

	let shared_out: shared_out::SharedOut = shared_out::SharedOut::new();

	for stream_result in listener.incoming() {
		let canonical_home = canonical_home.clone();
		let files = files.clone();
		let shared_out = shared_out.clone();

		spawn(move || {
			let stream = stream_result.unwrap();

			let mut thread_local =
				LocalState::new(shared_out, files, canonical_home, stream).unwrap();

			// Handle errors safely without breaking the server state
			client_thread(&mut thread_local)
				.map_err(|e| {
					println!("Thread exited with error: {}", e);
				})
				.ok();

			// Close file
			thread_local.file_close().unwrap();

			// Remove io
			thread_local.remove_thread_io().unwrap();
		});
	}

	Ok(())
}
