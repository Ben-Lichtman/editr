use std::error::Error;
use std::fs::{File, OpenOptions};
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
pub enum CreateResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum OpenResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum WriteResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum ReadResult {
	Ok(Vec<u8>),
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum SaveResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum Message {
	Invalid,
	Echo(Vec<u8>),
	CreateReq(String),
	CreateResp(CreateResult),
	OpenReq(String),
	OpenResp(OpenResult),
	WriteReq(WriteReqData),
	WriteResp(WriteResult),
	ReadReq(ReadReqData),
	ReadResp(ReadResult),
	SaveReq,
	SaveResp(SaveResult),
}

fn create_file(thread_local: &mut ThreadState, path: &str) -> Result<(), Box<dyn Error>> {
	OpenOptions::new().create(true).open(path)?;

	Ok(())
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
fn save_file(thread_local: &mut ThreadState) -> Result<(), Box<dyn Error>> { Ok(()) }

// Takes a message and the current client's state, processes it, and returns a message to reply with
pub fn process_message(thread_local: &mut ThreadState, msg: Message) -> (Message, bool) {
	match msg {
		Message::Echo(inner) => (Message::Echo(inner), false),
		Message::CreateReq(inner) => match create_file(thread_local, &inner) {
			Ok(_) => (Message::CreateResp(CreateResult::Ok), false),
			Err(e) => (Message::CreateResp(CreateResult::Err(e.to_string())), false),
		},
		Message::OpenReq(inner) => match open_file(thread_local, &inner) {
			// TODO Multithreading: Add new clients here, update the ThreadState's files
			Ok(_) => (Message::OpenResp(OpenResult::Ok), false),
			Err(e) => (Message::OpenResp(OpenResult::Err(e.to_string())), false),
		},
		Message::WriteReq(inner) => match thread_local.file_write(inner.offset, &inner.data) {
			Ok(_) => (Message::WriteResp(WriteResult::Ok), false),
			Err(e) => (Message::WriteResp(WriteResult::Err(e.to_string())), false),
		},
		Message::ReadReq(inner) => {
			let read_from = inner.offset;
			let read_to = inner.offset + inner.len;
			match thread_local.file_read(read_from, read_to) {
				Ok(data) => (Message::ReadResp(ReadResult::Ok(data)), false),
				Err(e) => (Message::ReadResp(ReadResult::Err(e.to_string())), false),
			}
		}
		Message::SaveReq => match save_file(thread_local) {
			Ok(_) => (Message::SaveResp(SaveResult::Ok), false),
			Err(e) => (Message::SaveResp(SaveResult::Err(e.to_string())), false),
		},
		_ => (Message::Invalid, false),
	}
}
