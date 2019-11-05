use std::fs::OpenOptions;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::thread::{current, ThreadId};

use crate::error::EditrResult;
use crate::message::Message;
use crate::state::*;

pub struct LocalState {
	thread_id: ThreadId,
	threads_io: SharedIO,
	files: FileStates,
	canonical_home: PathBuf,
	opened_file: Option<PathBuf>,
}

impl LocalState {
	pub fn new(threads_io: SharedIO, files: FileStates, canonical_home: PathBuf) -> LocalState {
		LocalState {
			thread_id: current().id(),
			threads_io,
			files,
			canonical_home,
			opened_file: None,
		}
	}

	pub fn canonical_home(&self) -> &PathBuf { &self.canonical_home }

	pub fn contains_file(&self, path: &PathBuf) -> EditrResult<bool> { self.files.contains(path) }

	pub fn insert_thread_io(&mut self, stream: TcpStream) -> EditrResult<()> {
		self.threads_io.insert(self.thread_id, stream)
	}

	pub fn remove_thread_io(&mut self) -> EditrResult<()> { self.threads_io.remove(self.thread_id) }

	pub fn file_create(&self, path: &str) -> EditrResult<()> {
		OpenOptions::new().write(true).create_new(true).open(path)?;
		Ok(())
	}

	// Returns a list of filenames in canonical_home as Strings.
	pub fn files_list(&self) -> EditrResult<Vec<String>> {
		let mut list = Vec::new();
		for f in self.canonical_home.read_dir()? {
			if let Some(name) = f?.file_name().into_string().ok() {
				list.push(name);
			}
		}
		Ok(list)
	}

	pub fn file_open(&mut self, path: &str) -> EditrResult<PathBuf> {
		// (currently) clients can only have one file open
		self.file_close()?;

		let canonical_path = Path::new(path).canonicalize()?;

		// Check that path is valid given client home
		if !canonical_path.starts_with(self.canonical_home()) {
			return Err("Invalid file path".into());
		}

		self.files.open(canonical_path.clone(), self.thread_id)?;

		self.opened_file = Some(canonical_path.clone());

		Ok(canonical_path)
	}

	pub fn file_close(&mut self) -> EditrResult<()> {
		// Check whether a file is currently open
		if let Some(path) = &self.opened_file {
			self.files.close(&path, self.thread_id)?;
			self.opened_file = None;
		}
		Ok(())
	}

	pub fn socket_read(&self, buffer: &mut [u8]) -> EditrResult<usize> {
		self.threads_io.read(self.thread_id, buffer)
	}

	pub fn socket_write(&self, buffer: &[u8]) -> EditrResult<usize> {
		self.threads_io.write(self.thread_id, buffer)
	}

	pub fn file_read(&self, from: usize, to: usize) -> EditrResult<Vec<u8>> {
		self.files.read(self.get_opened()?, from, to)
	}

	pub fn file_write(&self, offset: usize, data: &[u8]) -> EditrResult<()> {
		self.files.write(self.get_opened()?, offset, data)?;
		// Sync neigbours with the data just written
		self.broadcast_neighbours(Message::make_add_broadcast(offset, data))?;
		Ok(())
	}

	// Removes data from the file, starting from offset
	pub fn file_remove(&self, offset: usize, len: usize) -> EditrResult<()> {
		self.files.remove(self.get_opened()?, offset, len)?;
		// Sync neighbours with deletion
		self.broadcast_neighbours(Message::make_del_broadcast(offset, len))?;
		Ok(())
	}

	// Saves file to disk
	pub fn file_save(&self) -> EditrResult<()> { self.files.flush(self.get_opened()?) }

	fn get_opened(&self) -> EditrResult<&PathBuf> {
		self.opened_file.as_ref().ok_or("File not open".into())
	}

	// Broadcasts a message to other clients in the same file as self
	fn broadcast_neighbours(&self, msg: Message) -> EditrResult<()> {
		let data = msg.to_vec()?;
		self.files.for_each_client(self.get_opened()?, |client| {
			if client != self.thread_id {
				self.threads_io.write(client, &data)?;
			}
			Ok(())
		})?;
		Ok(())
	}
}
