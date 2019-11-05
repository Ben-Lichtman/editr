use std::error::Error;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use serde_json;

use crate::state::*;

#[derive(Serialize, Deserialize)]
pub enum CreateResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum DeleteResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub struct RenameReqData {
	from: String,
	to: String,
}

#[derive(Serialize, Deserialize)]
pub enum RenameResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum OpenResult {
	Ok(PathBuf),
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
pub struct RemoveReqData {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize)]
pub enum RemoveResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum SaveResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum FilesListResult {
	Ok(Vec<String>),
	Err(String),
}

#[derive(Serialize, Deserialize)]
pub enum Message {
	Invalid,
	Echo(Vec<u8>),
	CreateReq(String),
	CreateResp(CreateResult),
	DeleteReq(String),
	DeleteResp(DeleteResult),
	RenameReq(RenameReqData),
	RenameResp(RenameResult),
	OpenReq(String),
	OpenResp(OpenResult),
	WriteReq(WriteReqData),
	WriteResp(WriteResult),
	UpdateMessage(UpdateData),
	ReadReq(ReadReqData),
	ReadResp(ReadResult),
	RemoveReq(RemoveReqData),
	RemoveResp(RemoveResult),
	SaveReq,
	SaveResp(SaveResult),
	FilesListReq,
	FilesListResp(FilesListResult),
}

impl Message {
	pub fn from_slice(slice: &[u8]) -> Result<Message, Box<dyn Error>> {
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
			Message::CreateReq(inner) => match thread_local.file_create(&inner) {
				Ok(_) => (Message::CreateResp(CreateResult::Ok), false),
				Err(e) => (Message::CreateResp(CreateResult::Err(e.to_string())), false),
			},
			Message::DeleteReq(inner) => match thread_local.file_delete(&inner) {
				Ok(_) => (Message::DeleteResp(DeleteResult::Ok), false),
				Err(e) => (Message::DeleteResp(DeleteResult::Err(e.to_string())), false),
			},
			Message::RenameReq(inner) => match thread_local.file_rename(&inner.from, &inner.to) {
				Ok(_) => (Message::RenameResp(RenameResult::Ok), false),
				Err(e) => (Message::RenameResp(RenameResult::Err(e.to_string())), false),
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
			Message::RemoveReq(inner) => match thread_local.file_remove(inner.offset, inner.len) {
				Ok(_) => (Message::RemoveResp(RemoveResult::Ok), false),
				Err(e) => (Message::RemoveResp(RemoveResult::Err(e.to_string())), false),
			},
			Message::SaveReq => match thread_local.file_save() {
				Ok(_) => (Message::SaveResp(SaveResult::Ok), false),
				Err(e) => (Message::SaveResp(SaveResult::Err(e.to_string())), false),
			},
			Message::FilesListReq => match thread_local.files_list() {
				Ok(list) => (Message::FilesListResp(FilesListResult::Ok(list)), false),
				Err(e) => (
					Message::FilesListResp(FilesListResult::Err(e.to_string())),
					false,
				),
			},
			_ => (Message::Invalid, true),
		}
	}

	pub fn to_vec(&self) -> Result<Vec<u8>, Box<dyn Error>> {
		Ok(serde_json::to_vec(self).map_err(|e| e.to_string())?)
	}
}
