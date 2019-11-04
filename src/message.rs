use std::error::Error;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use serde_json;

use crate::state::*;

type MessageResult<T> = Result<T, Box<dyn Error>>;

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
pub enum CloseResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub struct WriteReqData {
	offset: usize,
	data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub enum WriteResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub struct UpdateAdd {
	offset: usize,
	data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateRemove {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize)]
pub enum UpdateData {
	Add(UpdateAdd),
	Remove(UpdateRemove),
}

#[derive(Serialize, Deserialize)]
pub struct ReadReqData {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize)]
pub enum ReadResult {
	Ok(Vec<u8>),
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub struct DeleteReqData {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize)]
pub enum DeleteResult {
	Ok,
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
	CloseReq,
	CloseResp(CloseResult),
	WriteReq(WriteReqData),
	WriteResp(WriteResult),
	UpdateMessage(UpdateData),
	ReadReq(ReadReqData),
	ReadResp(ReadResult),
	DeleteReq(DeleteReqData),
	DeleteResp(DeleteResult),
	SaveReq,
	SaveResp(SaveResult),
}

impl Message {
	pub fn from_slice(slice: &[u8]) -> MessageResult<Message> {
		Ok(serde_json::from_slice(slice).map_err(|e| e.to_string())?)
	}

	pub fn make_add_broadcast(offset: usize, data: &[u8]) -> Message {
		Message::UpdateMessage(UpdateData::Add(UpdateAdd {
			offset,
			data: Vec::from(data),
		}))
	}

	pub fn make_del_broadcast(offset: usize, len: usize) -> Message {
		Message::UpdateMessage(UpdateData::Remove(UpdateRemove { offset, len }))
	}

	pub fn process(self, thread_local: &mut LocalState) -> (Message, bool) {
		match self {
			Message::Echo(inner) => (Message::Echo(inner), false),
			Message::CreateReq(inner) => match thread_local.create(&inner) {
				Ok(_) => (Message::CreateResp(CreateResult::Ok), false),
				Err(e) => (Message::CreateResp(CreateResult::Err(e.to_string())), false),
			},
			Message::OpenReq(inner) => match thread_local.open(&inner) {
				Ok(p) => (Message::OpenResp(OpenResult::Ok(p)), false),
				Err(e) => (Message::OpenResp(OpenResult::Err(e.to_string())), false),
			},
			Message::CloseReq => match thread_local.close() {
				Ok(_) => (Message::CloseResp(CloseResult::Ok), false),
				Err(e) => (Message::CloseResp(CloseResult::Err(e.to_string())), false),
			},
			Message::WriteReq(inner) => match thread_local.write(inner.offset, &inner.data) {
				Ok(_) => (Message::WriteResp(WriteResult::Ok), false),
				Err(e) => (Message::WriteResp(WriteResult::Err(e.to_string())), false),
			},
			Message::ReadReq(inner) => {
				let read_from = inner.offset;
				let read_to = inner.offset + inner.len;
				match thread_local.file_state.read(
					&thread_local.current_file_loc,
					read_from,
					read_to,
				) {
					Ok(data) => (Message::ReadResp(ReadResult::Ok(data)), false),
					Err(e) => (Message::ReadResp(ReadResult::Err(e.to_string())), false),
				}
			}
			Message::DeleteReq(inner) => match thread_local.delete(inner.offset, inner.len) {
				Ok(_) => (Message::DeleteResp(DeleteResult::Ok), false),
				Err(e) => (Message::DeleteResp(DeleteResult::Err(e.to_string())), false),
			},
			Message::SaveReq => match thread_local.flush() {
				Ok(_) => (Message::SaveResp(SaveResult::Ok), false),
				Err(e) => (Message::SaveResp(SaveResult::Err(e.to_string())), false),
			},
			_ => (Message::Invalid, true),
		}
	}

	pub fn to_vec(&self) -> MessageResult<Vec<u8>> {
		Ok(serde_json::to_vec(self).map_err(|e| e.to_string())?)
	}
}
