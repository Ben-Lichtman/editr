use std::fs::{self, OpenOptions};
use std::net::TcpStream;

use std::path::PathBuf;
use std::thread::{current, ThreadId};

use crate::error::EditrResult;
use crate::message::Message;
use crate::state::*;

pub struct LocalState {
	thread_id: ThreadId,
	socket: Socket,
	files: FileStates,
	canonical_home: PathBuf,
	opened_file: Option<PathBuf>,
}

impl LocalState {
	pub fn new(
		threads_out: shared_out::SharedOut,
		files: FileStates,
		canonical_home: PathBuf,
		stream: TcpStream,
	) -> EditrResult<LocalState> {
		Ok(LocalState {
			thread_id: current().id(),
			socket: Socket::new(current().id(), stream, threads_out)?,
			files,
			canonical_home,
			opened_file: None,
		})
	}

	pub fn canonical_home(&self) -> &PathBuf { &self.canonical_home }

	pub fn contains_file(&self, path: &PathBuf) -> EditrResult<bool> { self.files.contains(path) }

	pub fn remove_thread_io(&mut self) -> EditrResult<()> { self.socket.close(self.thread_id) }

	// Creates a new file at path
	pub fn file_create(&self, path: &str) -> EditrResult<()> {
		OpenOptions::new()
			.write(true)
			.create_new(true)
			.open(self.prepend_home(path))?;
		Ok(())
	}

	// Deletes the file at path
	pub fn file_delete(&self, path: &str) -> EditrResult<()> {
		let path = self.prepend_home(path).canonicalize()?;
		// File must not be open by anyone
		if self.contains_file(&path)? {
			Err("File is busy".into())
		}
		else {
			fs::remove_file(path)?;
			Ok(())
		}
	}

	// Renames the file at 'from' into 'to'
	pub fn file_rename(&self, from: &str, to: &str) -> EditrResult<()> {
		let from = self.prepend_home(from).canonicalize()?;
		let to = self.prepend_home(to);

		if to.exists() {
			Err("File already exists".into())
		}
		else {
			// File must not be open by anyone
			if self.contains_file(&from)? {
				Err("File is busy".into())
			}
			else {
				fs::rename(from, to)?;
				Ok(())
			}
		}
	}

	// Returns a list of filenames in canonical_home as Strings.
	pub fn files_list(&self) -> EditrResult<Vec<String>> {
		let mut list = Vec::new();
		for f in self.canonical_home.read_dir()? {
			if let Ok(name) = f?.file_name().into_string() {
				list.push(name)
			}
		}
		Ok(list)
	}

	pub fn file_open(&mut self, path: &str) -> EditrResult<PathBuf> {
		// (currently) clients can only have one file open
		self.file_close()?;

		let canonical_path = self.prepend_home(path).canonicalize()?;

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

	pub fn socket_read(&mut self, buffer: &mut [u8]) -> EditrResult<usize> {
		self.socket.read(buffer)
	}

	pub fn socket_write(&self, buffer: &[u8]) -> EditrResult<usize> {
		self.socket.write(self.thread_id, buffer)
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

	pub fn move_cursor(&self, offset: isize) -> EditrResult<()> {
		self.files
			.move_cursor(self.get_opened()?, self.thread_id, offset)
	}

	pub fn file_write_cursor(&self, data: Vec<u8>) -> EditrResult<()> {
		self.files
			.file_write_cursor(self.get_opened()?, self.thread_id, &data)
	}

	pub fn file_remove_cursor(&self, len: usize) -> EditrResult<()> {
		self.files
			.file_remove_cursor(self.get_opened()?, self.thread_id, len)
	}

	pub fn get_cursors(&self) -> EditrResult<(usize, Vec<usize>)> {
		self.files.get_cursors(self.get_opened()?, self.thread_id)
	}

	fn get_opened(&self) -> EditrResult<&PathBuf> {
		self.opened_file
			.as_ref()
			.ok_or_else(|| "File not open".into())
	}

	// Broadcasts a message to other clients in the same file as self
	fn broadcast_neighbours(&self, msg: Message) -> EditrResult<()> {
		let data = msg.to_vec()?;
		self.files.for_each_client(self.get_opened()?, |client| {
			if client != self.thread_id {
				self.socket.write(client, &data)?;
			}
			Ok(())
		})?;
		Ok(())
	}

	// Prepends user input paths with canonical home
	fn prepend_home(&self, path: &str) -> PathBuf {
		let mut new_path = self.canonical_home().clone();
		new_path.push(path);
		new_path
	}
}
