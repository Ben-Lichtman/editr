use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
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

	pub fn get_rope(&self) -> &Rope { &self.rope }
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

	pub fn canonical_home(&self) -> &PathBuf { &self.canonical_home }

	pub fn contains_file(&self, key: &PathBuf) -> Result<bool, Box<dyn Error>> {
		Ok(self
			.files
			.read()
			.map_err(|e| e.to_string())?
			.contains_key(key))
	}

	pub fn insert_files(&mut self, key: PathBuf, val: FileState) -> Result<(), Box<dyn Error>> {
		self.files
			.write()
			.map_err(|e| e.to_string())?
			.insert(key, val);
		Ok(())
	}

	pub fn remove_files(&mut self, key: &PathBuf) -> Result<(), Box<dyn Error>> {
		self.files.write().map_err(|e| e.to_string())?.remove(key);
		Ok(())
	}

	pub fn insert_thread_shared(&mut self, stream: TcpStream) -> Result<(), Box<dyn Error>> {
		self.thread_shared
			.write()
			.map_err(|e| e.to_string())?
			.insert(self.thread_id, Mutex::new(ThreadShared::new(stream)));
		Ok(())
	}

	pub fn remove_thread_shared(&mut self) -> Result<(), Box<dyn Error>> {
		self.thread_shared
			.write()
			.map_err(|e| e.to_string())?
			.remove(&self.thread_id);
		Ok(())
	}

	pub fn add_file_bookkeeping(&mut self, key: &PathBuf) -> Result<(), Box<dyn Error>> {
		self.files
			.write()
			.map_err(|e| e.to_string())?
			.get_mut(key)
			.ok_or("Thread local storage does not exist")?
			.clients
			.insert(self.thread_id);
		Ok(())
	}

	pub fn read(&self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error>> {
		Ok(self
			.thread_shared
			.read()
			.map_err(|e| e.to_string())?
			.get(&self.thread_id)
			.ok_or("Thread local storage does not exist")?
			.lock()
			.map_err(|e| e.to_string())?
			.reader
			.read(buffer)?)
	}

	pub fn write(&self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
		Ok(self
			.thread_shared
			.read()
			.map_err(|e| e.to_string())?
			.get(&self.thread_id)
			.ok_or("Thread local storage does not exist")?
			.lock()
			.map_err(|e| e.to_string())?
			.writer
			.write(buffer)?)
	}
}
