pub mod file_state_container;
pub mod shared_io_container;

use std::error::Error;
use std::fs::OpenOptions;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{current, ThreadId};

use self::file_state_container::FileStateContainer;
use self::shared_io_container::SharedIOContainer;
use crate::message::Message;

pub struct ThreadState {
	thread_id: ThreadId,
	threads_io: Arc<SharedIOContainer>,
	files: Arc<FileStateContainer>,
	canonical_home: PathBuf,
	pub current_file_loc: Option<PathBuf>,
}

impl ThreadState {
	pub fn new(
		threads_io: Arc<SharedIOContainer>,
		files: Arc<FileStateContainer>,
		canonical_home: PathBuf,
	) -> ThreadState {
		ThreadState {
			thread_id: current().id(),
			threads_io,
			files,
			canonical_home,
			current_file_loc: None,
		}
	}

	pub fn canonical_home(&self) -> &PathBuf { &self.canonical_home }

	pub fn contains_file(&self, path: &PathBuf) -> Result<bool, Box<dyn Error>> {
		self.files.contains(path)
	}

	pub fn insert_thread_io(&mut self, stream: TcpStream) -> Result<(), Box<dyn Error>> {
		self.threads_io.insert(self.thread_id, stream)
	}

	pub fn remove_thread_io(&mut self) -> Result<(), Box<dyn Error>> {
		self.threads_io.remove(self.thread_id)
	}

	pub fn file_create(&self, path: &str) -> Result<(), Box<dyn Error>> {
		OpenOptions::new().write(true).create_new(true).open(path)?;
		Ok(())
	}

	pub fn file_open(&mut self, path: &str) -> Result<PathBuf, Box<dyn Error>> {
		// (currently) clients can only have one file open
		self.file_close()?;

		let canonical_path = Path::new(path).canonicalize()?;

		// Check that path is valid given client home
		if !canonical_path.starts_with(self.canonical_home()) {
			return Err("Invalid file path".into());
		}

		self.files.open(canonical_path.clone(), &self.thread_id)?;

		self.current_file_loc = Some(canonical_path.clone());

		Ok(canonical_path)
	}

	pub fn file_close(&mut self) -> Result<(), Box<dyn Error>> {
		// Check whether a file is currently open
		if let Some(path) = &self.current_file_loc {
			self.files.close(&path, &self.thread_id)?;
			self.current_file_loc = None;
		}
		Ok(())
	}

	pub fn socket_read(&self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error>> {
		self.threads_io.socket_read(&self.thread_id, buffer)
	}

	pub fn socket_write(&self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
		self.threads_io.socket_write(&self.thread_id, buffer)
	}

	pub fn file_read(&self, from: usize, to: usize) -> Result<Vec<u8>, Box<dyn Error>> {
		self.files.read(self.file_loc()?, from, to)
	}

	pub fn file_write(&self, offset: usize, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.files.write(self.file_loc()?, offset, data)?;
		// Sync neigbours with the data just written
		self.broadcast_neighbours(Message::make_add_broadcast(offset, data))?;
		Ok(())
	}

	pub fn file_delete(&self, offset: usize, len: usize) -> Result<(), Box<dyn Error>> {
		self.files.delete(self.file_loc()?, offset, len)?;
		// Sync neighbours with deletion
		self.broadcast_neighbours(Message::make_del_broadcast(offset, len))?;
		Ok(())
	}

	pub fn file_save(&self) -> Result<(), Box<dyn Error>> { self.files.flush(self.file_loc()?) }

	// Returns client's current file path
	fn file_loc(&self) -> Result<&PathBuf, &str> {
		self.current_file_loc.as_ref().ok_or("No file opened")
	}

	// Broadcasts a message to other clients in the same file as self
	fn broadcast_neighbours(&self, msg: Message) -> Result<(), Box<dyn Error>> {
		let data = msg.to_vec()?;
		self.files.for_each_client(self.file_loc()?, |client| {
			if client != &self.thread_id {
				self.threads_io.socket_write(client, &data)?;
			}
			Ok(())
		})?;
		Ok(())
	}
}
