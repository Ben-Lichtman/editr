use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread::{current, ThreadId};

use crate::message::Message;
use crate::state::*;

type LocalStateResult<T> = Result<T, Box<dyn Error>>;

pub struct LocalState {
	pub thread_id: ThreadId,
	pub shared_io: SharedIO,
	pub file_state: FileState,
	pub current_file_loc: Option<PathBuf>,
	canonical_home: PathBuf,
}

impl LocalState {
	pub fn new(shared_io: SharedIO, file_state: FileState, canonical_home: PathBuf) -> LocalState {
		LocalState {
			thread_id: current().id(),
			shared_io,
			file_state,
			current_file_loc: None,
			canonical_home,
		}
	}

	pub fn create(&mut self, path: &str) -> LocalStateResult<()> {
		OpenOptions::new().write(true).create_new(true).open(path)?;
		Ok(())
	}

	// Open file at give path - file must already exist
	pub fn open(&mut self, path: &str) -> LocalStateResult<PathBuf> {
		// If a file is currently open, close it
		self.close().ok();

		let canonical_path = Path::new(path).canonicalize()?;

		// Check that path is valid given client home
		if !canonical_path.starts_with(&self.canonical_home) {
			return Err("Invalid file path".into());
		}

		self.current_file_loc = Some(canonical_path.clone());

		// Make sure the files hashmap contains this file
		if !self.file_state.contains(&canonical_path)? {
			// Read file
			let mut buffer = Vec::new();
			let mut file = File::open(&canonical_path)?;
			file.read_to_end(&mut buffer)?;

			self.file_state.insert_entry(&canonical_path)?;
			self.file_state.write(&self.current_file_loc, 0, &buffer)?;
		}

		// Add bookkeeping
		self.file_state
			.add_bookkeeping(&self.current_file_loc, self.thread_id)?;

		Ok(canonical_path)
	}

	pub fn close(&mut self) -> LocalStateResult<()> {
		// Check whether a file is currently open
		if let Some(p) = self.current_file_loc.as_ref() {
			// File already open, remove bookkeeping
			self.file_state
				.remove_bookkeeping(&self.current_file_loc, self.thread_id)?;

			// If the file is not being used by any clients, remove its records
			if self.file_state.should_close(p)? {
				self.file_state.remove_entry(p)?;
			}

			self.current_file_loc = None;
		}
		Ok(())
	}

	pub fn write(&self, offset: usize, data: &[u8]) -> LocalStateResult<()> {
		self.file_state
			.write(&self.current_file_loc, offset, data)?;

		// Iterate all clients of the current file
		for id in self.file_state.get_clients(&self.current_file_loc)? {
			// Do not send data to self
			if id == self.thread_id {
				continue;
			}
			// Send update to client
			self.shared_io
				.write(id, &Message::make_add_broadcast(offset, data).to_vec()?)?;
		}
		Ok(())
	}

	pub fn delete(&self, offset: usize, len: usize) -> LocalStateResult<()> {
		self.file_state
			.delete(&self.current_file_loc, offset, len)?;

		// Iterate all clients of the current file
		for id in self.file_state.get_clients(&self.current_file_loc)? {
			// Do not send data to self
			if id == self.thread_id {
				continue;
			}
			// Send update to client
			self.shared_io
				.write(id, &Message::make_del_broadcast(offset, len).to_vec()?)?;
		}
		Ok(())
	}

	pub fn flush(&mut self) -> LocalStateResult<()> {
		// Flatten the rope
		self.file_state.flatten(&self.current_file_loc)?;

		let path = self
			.current_file_loc
			.as_ref()
			.ok_or("File not open".to_string())?;

		// Create new file - truncating if existing
		let mut file = File::create(&path)?;

		// Read rope into vec
		let complete = self.file_state.read(
			&self.current_file_loc,
			0,
			self.file_state.len(&self.current_file_loc)?,
		)?;

		// Write rope to file
		file.write_all(&complete)?;
		Ok(())
	}
}
