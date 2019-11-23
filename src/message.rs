use std::error::Error;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use serde_json;

use crate::state::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum CreateResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DeleteResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RenameReqData {
	from: String,
	to: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RenameResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OpenResult {
	Ok(PathBuf),
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CloseResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WriteReqData {
	offset: usize,
	data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WriteResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateAdd {
	offset: usize,
	data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateRemove {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UpdateData {
	Add(UpdateAdd),
	Remove(UpdateRemove),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReadReqData {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ReadResult {
	Ok(Vec<u8>),
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveReqData {
	offset: usize,
	len: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RemoveResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SaveResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FilesListResult {
	Ok(Vec<String>),
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MoveCursorResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WriteAtCursorReqData {
	data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WriteAtCursorResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveAtCursorReqData {
	len: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RemoveAtCursorResult {
	Ok,
	Err(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GetCursorsResult {
	Ok((usize, Vec<usize>)),
	Err(String),
}
#[derive(Serialize, Deserialize, Debug)]
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
	CloseReq,
	CloseResp(CloseResult),
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
	MoveCursor(isize),
	MoveCursorResp(MoveCursorResult),
	WriteAtCursorReq(WriteAtCursorReqData),
	WriteAtCursorResp(WriteAtCursorResult),
	RemoveAtCursorReq(RemoveAtCursorReqData),
	RemoveAtCursorResp(RemoveAtCursorResult),
	GetCursorsReq,
	GetCursorsResp(GetCursorsResult),
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
			Message::CloseReq => match thread_local.file_close() {
				Ok(_) => (Message::CloseResp(CloseResult::Ok), false),
				Err(e) => (Message::CloseResp(CloseResult::Err(e.to_string())), false),
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
			Message::MoveCursor(inner) => match thread_local.move_cursor(inner) {
				Ok(_) => (Message::MoveCursorResp(MoveCursorResult::Ok), false),
				Err(e) => (
					Message::MoveCursorResp(MoveCursorResult::Err(e.to_string())),
					false,
				),
			},
			Message::WriteAtCursorReq(inner) => match thread_local.file_write_cursor(inner.data) {
				Ok(_) => (Message::WriteAtCursorResp(WriteAtCursorResult::Ok), false),
				Err(e) => (
					Message::WriteAtCursorResp(WriteAtCursorResult::Err(e.to_string())),
					false,
				),
			},
			Message::RemoveAtCursorReq(inner) => match thread_local.file_remove_cursor(inner.len) {
				Ok(_) => (Message::RemoveAtCursorResp(RemoveAtCursorResult::Ok), false),
				Err(e) => (
					Message::RemoveAtCursorResp(RemoveAtCursorResult::Err(e.to_string())),
					false,
				),
			},
			Message::GetCursorsReq => match thread_local.get_cursors() {
				Ok(cursors) => (
					Message::GetCursorsResp(GetCursorsResult::Ok(cursors)),
					false,
				),
				Err(e) => (
					Message::GetCursorsResp(GetCursorsResult::Err(e.to_string())),
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
