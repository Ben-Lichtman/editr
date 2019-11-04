use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

use crate::rope::Rope;

type FileStateResult<T> = Result<T, Box<dyn Error>>;

struct FileStateInner {
	rope: Rope,
	clients: Mutex<HashSet<ThreadId>>,
}

impl Deref for FileStateInner {
	type Target = Rope;

	fn deref(&self) -> &Self::Target { &self.rope }
}

impl FileStateInner {
	fn new() -> FileStateInner {
		FileStateInner {
			rope: Rope::new(),
			clients: Mutex::new(HashSet::new()),
		}
	}

	fn add_client(&self, id: ThreadId) -> FileStateResult<()> {
		self.clients_op(|mut c| {
			c.insert(id);
			Ok(())
		})
	}

	fn remove_client(&self, id: ThreadId) -> FileStateResult<()> {
		self.clients_op(|mut c| {
			c.remove(&id);
			Ok(())
		})
	}

	fn get_clients(&self) -> FileStateResult<Vec<ThreadId>> {
		self.clients_op(|c| Ok(c.iter().map(|i| *i).collect()))
	}

	fn no_clients(&self) -> FileStateResult<bool> { self.clients_op(|c| Ok(c.len() == 0)) }

	fn clients_op<T, F: FnOnce(MutexGuard<HashSet<ThreadId>>) -> FileStateResult<T>>(
		&self,
		f: F,
	) -> FileStateResult<T> {
		f(self.clients.lock().map_err(|e| e.to_string())?)
	}
}

#[derive(Clone)]
pub struct FileState {
	inner: Arc<RwLock<HashMap<PathBuf, FileStateInner>>>,
}

impl FileState {
	pub fn new() -> FileState {
		FileState {
			inner: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	// Add record that a client is using this file
	pub fn add_bookkeeping(&self, path: &Option<PathBuf>, id: ThreadId) -> FileStateResult<()> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| {
			f.add_client(id)?;
			Ok(())
		})
	}

	// Remove record that a client is using this file
	pub fn remove_bookkeeping(&self, path: &Option<PathBuf>, id: ThreadId) -> FileStateResult<()> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| {
			f.remove_client(id)?;
			Ok(())
		})
	}

	pub fn write(&self, path: &Option<PathBuf>, offset: usize, data: &[u8]) -> FileStateResult<()> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| f.insert_at(offset, data))
	}

	pub fn read(&self, path: &Option<PathBuf>, from: usize, to: usize) -> FileStateResult<Vec<u8>> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| f.collect(from, to))
	}

	pub fn delete(&self, path: &Option<PathBuf>, offset: usize, len: usize) -> FileStateResult<()> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| f.remove(offset, len))
	}

	pub fn len(&self, path: &Option<PathBuf>) -> FileStateResult<usize> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| f.len())
	}

	pub fn flatten(&self, path: &Option<PathBuf>) -> FileStateResult<()> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| f.flatten())
	}

	// Check if the file is currently open
	pub fn contains(&self, path: &PathBuf) -> FileStateResult<bool> {
		self.hashmap_op(|m| Ok(m.contains_key(path)))
	}

	// Add to open files
	pub fn insert_entry(&self, path: &PathBuf) -> FileStateResult<()> {
		self.hashmap_mut_op(|mut m| {
			m.insert(path.to_path_buf(), FileStateInner::new());
			Ok(())
		})
	}

	// Remove from open files
	pub fn remove_entry(&self, path: &PathBuf) -> FileStateResult<()> {
		self.hashmap_mut_op(|mut m| {
			m.remove(path);
			Ok(())
		})
	}

	// Get a vector of clients using the current file
	pub fn get_clients(&self, path: &Option<PathBuf>) -> FileStateResult<Vec<ThreadId>> {
		let path = path.as_ref().ok_or("File not open".to_string())?;
		self.filestate_op(&path, |f| f.get_clients())
	}

	// Check whether there are no clients using the current file
	pub fn should_close(&self, path: &PathBuf) -> FileStateResult<bool> {
		self.filestate_op(&path, |f| f.no_clients())
	}

	fn filestate_op<T, F: FnOnce(&FileStateInner) -> FileStateResult<T>>(
		&self,
		path: &PathBuf,
		f: F,
	) -> FileStateResult<T> {
		self.hashmap_op(|m| {
			let value = m.get(path).ok_or("FileState op failed".to_string())?;
			f(value)
		})
	}

	fn hashmap_op<
		T,
		F: FnOnce(RwLockReadGuard<HashMap<PathBuf, FileStateInner>>) -> FileStateResult<T>,
	>(
		&self,
		f: F,
	) -> FileStateResult<T> {
		f(self.inner.read().map_err(|e| e.to_string())?)
	}

	fn hashmap_mut_op<
		T,
		F: FnOnce(RwLockWriteGuard<HashMap<PathBuf, FileStateInner>>) -> FileStateResult<T>,
	>(
		&self,
		f: F,
	) -> FileStateResult<T> {
		f(self.inner.write().map_err(|e| e.to_string())?)
	}
}
