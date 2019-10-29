use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::rope::Rope;
use crate::state::{FileState, ThreadState};

#[derive(Serialize, Deserialize)]
pub struct WriteReqData {
	offset: usize,
	data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct ReadReqData {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
	Invalid,
	Echo(Vec<u8>),
	OpenReq(String),
	OpenResp(bool),
	WriteReq(WriteReqData),
	WriteResp,
	ReadReq(ReadReqData),
	ReadResp(Vec<u8>),
	SaveReq,
	SaveResp,
}

fn open_file(thread_local: &mut ThreadState, path: &str) -> Result<PathBuf, Box<dyn Error>> {
	// TODO Remove self from bookkeeping of a file already opened
	// TODO possibly close file that was already opened
	let path = Path::new(path);

	let canonical_path = path.canonicalize()?;

	// Check that path is valid given client home
	if !canonical_path.starts_with(thread_local.canonical_home()) {
		return Err("Invalid file path".into());
	}

	// Make sure the files hashmap contains this file
	if !thread_local.contains_file(&canonical_path)? {
		// Read file
		let mut buffer = Vec::new();
		let mut file = File::open(&canonical_path)?;
		file.read_to_end(&mut buffer)?;

		// Add to rope
		let rope = Rope::new();
		rope.insert_at(0, &buffer)?;

		thread_local.insert_files(canonical_path.clone(), FileState::new(rope))?;
	}

	// Add bookkeeping
	thread_local.add_file_bookkeeping(&canonical_path)?;

	thread_local.current_file_loc = Some(canonical_path.clone());

	Ok(canonical_path)
}

// Function to handle the save request made by a thread
// Current flow: Receive the message
//				 Acquire lock for the filestate
// 				 Flatten the rope
//  		 	 Release the lock for the filestate
fn handle_save(thread_local: &mut ThreadState, _msg: Message) -> Result<(), Box<dyn Error>> {
	// thread_local
	// 	.files
	// 	.read()
	// 	.
	Ok(())
}

// Takes a message and the current client's state, processes it, and returns a message to reply with
pub fn process_message(thread_local: &mut ThreadState, msg: Message) -> (Message, bool) {
	match msg {
		Message::Echo(inner) => (Message::Echo(inner), false),
		Message::OpenReq(inner) => match open_file(thread_local, &inner) {
			// TODO Multithreading: Add new clients here, update the ThreadState's files
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
		Message::SaveReq => {
			// Flatten the rope & save the file
			// Current assumption is that client will have the most up-to-date version and is happy with
			// having that version being written to file
			// TODO Multithreading: Consider the client not having the most updated file, thus saving the altered view from another client.
			(Message::SaveResp, false)
		}
		_ => (Message::Invalid, false),
	}
}
