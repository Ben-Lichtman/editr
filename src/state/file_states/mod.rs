mod file_state;

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

use self::file_state::FileState;
use crate::rope::Rope;

pub struct FileStateContainer {
	container: RwLock<HashMap<PathBuf, FileState>>,
}

impl FileStateContainer {
	pub fn new() -> FileStateContainer {
		FileStateContainer {
			container: RwLock::new(HashMap::new()),
		}
	}

	// True if container contains file at path
	pub fn contains(&self, path: &PathBuf) -> Result<bool, Box<dyn Error>> {
		self.read_op(|container| Ok(container.contains_key(path)))
	}

	// Opens the file at path for the client.
	// If the file isn't in container, it will be read in.
	// TODO: Minimise write lock while avoiding race on insertion
	pub fn open(&self, path: PathBuf, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.write_op(|mut container| {
			match container.get(&path) {
				Some(file) => file.add_client(id)?,
				// Read into container if not present
				None => {
					let rope = read_to_rope(&path)?;
					container.insert(path.clone(), FileState::new(rope));
				}
			}
			Ok(())
		})
	}

	// Closes the file at path for client.
	pub fn close(&self, path: &PathBuf, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.file_op(path, |file| file.remove_client(id))?;
		// Remove file from container if there are no clients remaining
		self.write_op(|mut container| {
			if let Some(state) = container.get(path) {
				if state.no_clients()? {
					container.remove(path);
				}
			}
			Ok(())
		})
	}

	// Reads from the file at path starting from 'from' and ending at 'to'
	pub fn read(&self, path: &PathBuf, from: usize, to: usize) -> Result<Vec<u8>, Box<dyn Error>> {
		self.file_op(path, |file| file.collect(from, to))
	}

	// Writes to file at path at offset
	pub fn write(&self, path: &PathBuf, offset: usize, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.file_op(path, |file| file.insert_at(offset, data))
	}

	// Deletes from the file at path, starting from offset
	pub fn delete(&self, path: &PathBuf, offset: usize, len: usize) -> Result<(), Box<dyn Error>> {
		self.file_op(path, |file| file.remove_range(offset, offset + len))
	}

	// Flushes file to disk
	pub fn flush(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
		let rope = self.file_op(path, |file| {
			file.flatten()?;
			file.collect(0, file.len()?)
		})?;
		File::create(&path)?.write_all(&rope)?;
		Ok(())
	}

	// Calls a closure f on each client in the file at path
	pub fn for_each_client<F: Fn(&ThreadId) -> Result<(), Box<dyn Error>>>(
		&self,
		path: &PathBuf,
		f: F,
	) -> Result<(), Box<dyn Error>> {
		self.file_op(path, |file| file.for_each_client(|id| f(id)))
	}

	// Applies an op that requires a read lock on the underlying container
	fn read_op<
		T,
		F: FnOnce(RwLockReadGuard<HashMap<PathBuf, FileState>>) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.container.read().map_err(|e| e.to_string())?)
	}

	// Applies an op that requires a write lock on the underlying container
	fn write_op<
		T,
		F: FnOnce(RwLockWriteGuard<HashMap<PathBuf, FileState>>) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.container.write().map_err(|e| e.to_string())?)
	}

	// Applies an op on path's FileState
	fn file_op<T, F: FnOnce(&FileState) -> Result<T, Box<dyn Error>>>(
		&self,
		path: &PathBuf,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		self.read_op(|container| {
			op(container
				.get(path)
				.ok_or("Thread local storage does not exist")?)
		})
	}
}

// Loads contents of file at path into a Rope
fn read_to_rope(path: &PathBuf) -> Result<Rope, Box<dyn Error>> {
	let mut buffer = Vec::new();
	let mut file = File::open(&path)?;
	file.read_to_end(&mut buffer)?;

	let rope = Rope::new();
	rope.insert_at(0, &buffer)?;
	Ok(rope)
}
