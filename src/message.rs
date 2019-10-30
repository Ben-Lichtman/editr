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
	Ok(PathBuf),
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

// Takes a message and the current client's state, processes it, and returns a message to reply with
pub fn process_message(thread_local: &mut ThreadState, msg: Message) -> (Message, bool) {
	match msg {
		Message::Echo(inner) => (Message::Echo(inner), false),
		Message::CreateReq(inner) => match thread_local.file_create(&inner) {
			Ok(_) => (Message::CreateResp(CreateResult::Ok), false),
			Err(e) => (Message::CreateResp(CreateResult::Err(e.to_string())), false),
		},
		Message::OpenReq(inner) => match thread_local.file_open(&inner) {
			Ok(p) => (Message::OpenResp(OpenResult::Ok(p)), false),
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
		Message::SaveReq => match thread_local.file_save() {
			Ok(_) => (Message::SaveResp(SaveResult::Ok), false),
			Err(e) => (Message::SaveResp(SaveResult::Err(e.to_string())), false),
		},
		_ => (Message::Invalid, false),
	}
}
