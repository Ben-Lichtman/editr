pub mod shared_io_container;
mod thread_io;

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::net::TcpStream;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::{current, ThreadId};

use crate::rope::Rope;
use crate::state::shared_io_container::SharedIOContainer;

pub type FileStateContainer = Arc<RwLock<HashMap<PathBuf, FileState>>>;

pub struct FileState {
	rope: Rope,
	clients: HashSet<ThreadId>,
}

pub struct ThreadState {
	thread_id: ThreadId,
	threads_io: Arc<SharedIOContainer>,
	files: FileStateContainer,
	canonical_home: PathBuf,
	pub current_file_loc: Option<PathBuf>,
}

impl Deref for FileState {
	type Target = Rope;
	fn deref(&self) -> &Self::Target { &self.rope }
}

impl FileState {
	pub fn new(rope: Rope) -> FileState {
		FileState {
			rope,
			clients: HashSet::new(),
		}
	}
}

impl ThreadState {
	pub fn new(
		threads_io: Arc<SharedIOContainer>,
		files: FileStateContainer,
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

	fn file_hashmap_read_op<
		T,
		F: FnOnce(RwLockReadGuard<HashMap<PathBuf, FileState>>) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		f: F,
	) -> Result<T, Box<dyn Error>> {
		f(self.files.read().map_err(|e| e.to_string())?)
	}

	fn file_hashmap_write_op<
		T,
		F: FnOnce(RwLockWriteGuard<HashMap<PathBuf, FileState>>) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		f: F,
	) -> Result<T, Box<dyn Error>> {
		f(self.files.write().map_err(|e| e.to_string())?)
	}

	fn file_state_read_op<T, F: FnOnce(&FileState) -> Result<T, Box<dyn Error>>>(
		&self,
		key: &PathBuf,
		f: F,
	) -> Result<T, Box<dyn Error>> {
		self.file_hashmap_write_op(|m| {
			f(m.get(key).ok_or("Thread local storage does not exist")?)
		})
	}

	fn file_state_write_op<T, F: FnOnce(&mut FileState) -> Result<T, Box<dyn Error>>>(
		&self,
		key: &PathBuf,
		f: F,
	) -> Result<T, Box<dyn Error>> {
		self.file_hashmap_write_op(|mut m| {
			f(m.get_mut(key)
				.ok_or("Thread local storage does not exist")?)
		})
	}

	fn add_file_bookkeeping(&mut self, key: &PathBuf) -> Result<(), Box<dyn Error>> {
		self.file_state_write_op(key, |m| {
			m.clients.insert(self.thread_id);
			Ok(())
		})
	}

	fn remove_file_bookkeeping(&mut self, key: &PathBuf) -> Result<(), Box<dyn Error>> {
		self.file_state_write_op(key, |m| {
			m.clients.remove(&self.thread_id);
			Ok(())
		})?;

		// Check if the hashset of clients for this file is empty
		if self.file_state_read_op(key, |s| Ok(s.len()? == 0))? {
			// Remove file from hashmap
			self.file_hashmap_write_op(|mut m| {
				m.remove(key);
				Ok(())
			})?;
		}

		Ok(())
	}

	pub fn canonical_home(&self) -> &PathBuf { &self.canonical_home }

	pub fn contains_file(&self, key: &PathBuf) -> Result<bool, Box<dyn Error>> {
		self.file_hashmap_read_op(|m| Ok(m.contains_key(key)))
	}

	pub fn insert_files(&mut self, key: PathBuf, val: FileState) -> Result<(), Box<dyn Error>> {
		self.file_hashmap_write_op(|mut m| {
			m.insert(key, val);
			Ok(())
		})
	}

	pub fn remove_files(&mut self, key: &PathBuf) -> Result<(), Box<dyn Error>> {
		self.file_hashmap_write_op(|mut m| {
			m.remove(key);
			Ok(())
		})
	}

	pub fn insert_thread_io(&mut self, stream: TcpStream) -> Result<(), Box<dyn Error>> {
		self.threads_io.insert(self.thread_id, stream)
	}

	pub fn remove_thread_io(&mut self) -> Result<(), Box<dyn Error>> {
		self.threads_io.remove(self.thread_id)
	}

	pub fn file_create(&self, path: &str) -> Result<(), Box<dyn Error>> {
		OpenOptions::new().create(true).open(path)?;
		Ok(())
	}

	pub fn file_open(&mut self, path: &str) -> Result<PathBuf, Box<dyn Error>> {
		self.file_close()?;

		let canonical_path = Path::new(path).canonicalize()?;

		// Check that path is valid given client home
		if !canonical_path.starts_with(self.canonical_home()) {
			return Err("Invalid file path".into());
		}

		// Make sure the files hashmap contains this file
		if !self.contains_file(&canonical_path)? {
			// Read file
			let mut buffer = Vec::new();
			let mut file = File::open(&canonical_path)?;
			file.read_to_end(&mut buffer)?;

			// Add to rope
			let rope = Rope::new();
			rope.insert_at(0, &buffer)?;

			self.insert_files(canonical_path.clone(), FileState::new(rope))?;
		}

		// Add bookkeeping
		self.add_file_bookkeeping(&canonical_path)?;

		self.current_file_loc = Some(canonical_path.clone());

		Ok(canonical_path)
	}

	pub fn file_close(&mut self) -> Result<(), Box<dyn Error>> {
		// Check whether a file is currently open
		if let Some(pathbuf) = self.current_file_loc.clone() {
			// File already open, remove bookkeeping
			self.remove_file_bookkeeping(&pathbuf)?;
			self.current_file_loc = None;
		}
		Ok(())
	}

	pub fn socket_read(&self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error>> {
		self.threads_io.socket_read(self.thread_id, buffer)
	}

	pub fn socket_write(&self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
		self.threads_io.socket_write(self.thread_id, buffer)
	}

	pub fn file_read(&self, from: usize, to: usize) -> Result<Vec<u8>, Box<dyn Error>> {
		self.file_state_read_op(
			self.current_file_loc.as_ref().ok_or("No file opened")?,
			|m| m.collect(from, to),
		)
	}

	pub fn file_write(&self, offset: usize, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.file_state_read_op(
			self.current_file_loc.as_ref().ok_or("No file opened")?,
			|m| m.insert_at(offset, data),
		)

		// Iterate through clients editing the file
		// If the client isn't self, send an update packet to them through their socket
	}

	pub fn file_save(&self) -> Result<(), Box<dyn Error>> {
		self.file_state_read_op(
			self.current_file_loc.as_ref().ok_or("No file opened")?,
			|m| m.flatten(),
		)
	}
}
