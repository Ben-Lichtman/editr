use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::{current, ThreadId};

use crate::rope::Rope;

pub type ThreadSharedContainer = Arc<RwLock<HashMap<ThreadId, Mutex<ThreadShared>>>>;

pub type FileStateContainer = Arc<RwLock<HashMap<PathBuf, FileState>>>;

pub struct FileState {
	rope: Rope,
	clients: HashSet<ThreadId>,
}

pub struct ThreadShared {
	reader: BufReader<TcpStream>,
	writer: BufWriter<TcpStream>,
}

pub struct ThreadState {
	thread_id: ThreadId,
	thread_shared: ThreadSharedContainer,
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

impl ThreadShared {
	pub fn new(stream: TcpStream) -> ThreadShared {
		ThreadShared {
			reader: BufReader::new(stream.try_clone().unwrap()),
			writer: BufWriter::with_capacity(0, stream.try_clone().unwrap()),
		}
	}
}

impl ThreadState {
	pub fn new(
		thread_shared: ThreadSharedContainer,
		files: FileStateContainer,
		canonical_home: PathBuf,
	) -> ThreadState {
		ThreadState {
			thread_id: current().id(),
			thread_shared,
			files,
			canonical_home,
			current_file_loc: None,
		}
	}

	fn thread_hashmap_read_op<
		T,
		F: FnOnce(
			RwLockReadGuard<HashMap<ThreadId, Mutex<ThreadShared>>>,
		) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		f: F,
	) -> Result<T, Box<dyn Error>> {
		f(self.thread_shared.read().map_err(|e| e.to_string())?)
	}

	fn thread_hashmap_write_op<
		T,
		F: FnOnce(
			RwLockWriteGuard<HashMap<ThreadId, Mutex<ThreadShared>>>,
		) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		f: F,
	) -> Result<T, Box<dyn Error>> {
		f(self.thread_shared.write().map_err(|e| e.to_string())?)
	}

	fn thread_shared_op<T, F: FnOnce(MutexGuard<ThreadShared>) -> Result<T, Box<dyn Error>>>(
		&self,
		id: &ThreadId,
		f: F,
	) -> Result<T, Box<dyn Error>> {
		self.thread_hashmap_read_op(|m| {
			f(m.get(id)
				.ok_or("Thread local storage does not exist")?
				.lock()
				.map_err(|e| e.to_string())?)
		})
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

	pub fn insert_thread_shared(&mut self, stream: TcpStream) -> Result<(), Box<dyn Error>> {
		self.thread_hashmap_write_op(|mut m| {
			m.insert(self.thread_id, Mutex::new(ThreadShared::new(stream)));
			Ok(())
		})
	}

	pub fn remove_thread_shared(&mut self) -> Result<(), Box<dyn Error>> {
		self.thread_hashmap_write_op(|mut m| {
			m.remove(&self.thread_id);
			Ok(())
		})
	}

	pub fn add_file_bookkeeping(&mut self, key: &PathBuf) -> Result<(), Box<dyn Error>> {
		self.file_state_write_op(key, |m| {
			m.clients.insert(self.thread_id);
			Ok(())
		})
	}

	pub fn file_create(&self, path: &str) -> Result<(), Box<dyn Error>> {
		OpenOptions::new().create(true).open(path)?;
		Ok(())
	}

	pub fn file_open(&mut self, path: &str) -> Result<PathBuf, Box<dyn Error>> {
		// TODO Remove self from bookkeeping of a file already opened
		// TODO possibly close file that was already opened
		let path = Path::new(path);

		let canonical_path = path.canonicalize()?;

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

	pub fn socket_read(&self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error>> {
		self.thread_shared_op(&self.thread_id, |mut m| Ok(m.reader.read(buffer)?))
	}

	pub fn socket_write(&self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
		self.thread_shared_op(&self.thread_id, |mut m| Ok(m.writer.write(buffer)?))
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
	}

	pub fn file_save(&self) -> Result<(), Box<dyn Error>> {
		self.file_state_read_op(
			self.current_file_loc.as_ref().ok_or("No file opened")?,
			|m| m.flatten(),
		)
	}
}
