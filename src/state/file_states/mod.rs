mod file_state;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::ThreadId;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use self::file_state::FileState;
use crate::error::EditrResult;
use crate::rope::Rope;

#[derive(Clone, Default)]
pub struct FileStates {
	container: Arc<RwLock<HashMap<PathBuf, FileState>>>,
}

impl FileStates {
	pub fn new() -> FileStates {
		FileStates {
			container: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	// True if container contains file at path
	pub fn contains(&self, path: &PathBuf) -> EditrResult<bool> {
		self.op(|container| Ok(container.contains_key(path)))
	}

	// Opens the file at path for the client.
	// If the file isn't in container, it will be read in.
	// TODO: Minimise write lock while avoiding race on insertion
	pub fn open(&self, path: PathBuf, id: ThreadId, name: Option<String>) -> EditrResult<()> {
		self.mut_op(|mut container| {
			match container.get(&path) {
				Some(file) => file.add_client(id, name)?,
				// Read into container if not present
				None => {
					let file = FileState::new(read_to_rope(&path)?);
					file.add_client(id, name)?;
					container.insert(path.clone(), file);
				}
			}
			Ok(())
		})
	}

	// Closes the file at path for client.
	pub fn close(&self, path: &PathBuf, id: ThreadId) -> EditrResult<()> {
		self.file_op(path, |file| file.remove_client(id))?;
		// Remove file from container if there are no clients remaining
		self.mut_op(|mut container| {
			if let Some(state) = container.get(path) {
				if state.no_clients()? {
					container.remove(path);
				}
			}
			Ok(())
		})
	}

	// Reads from the file at path starting from 'from' and ending at 'to'
	pub fn read(&self, path: &PathBuf, from: usize, to: usize) -> EditrResult<Vec<u8>> {
		self.file_op(path, |file| file.collect(from, to))
	}

	// Writes to file at path at offset
	pub fn write(&self, path: &PathBuf, offset: usize, data: &[u8]) -> EditrResult<()> {
		self.file_op(path, |file| file.insert_at(offset, data))
	}

	// Removes from the file at path, starting from offset
	pub fn remove(&self, path: &PathBuf, offset: usize, len: usize) -> EditrResult<()> {
		self.file_op(path, |file| file.remove_range(offset, offset + len))
	}

	// Flushes file to disk
	pub fn flush(&self, path: &PathBuf) -> EditrResult<()> {
		let rope = self.file_op(path, |file| {
			file.flatten()?;
			file.collect(0, file.len()?)
		})?;
		File::create(&path)?.write_all(&rope)?;
		Ok(())
	}

	// Calls a closure f on each client in the file at path
	pub fn for_each_client<F: Fn(ThreadId) -> EditrResult<()>>(
		&self,
		path: &PathBuf,
		f: F,
	) -> EditrResult<()> {
		self.file_op(path, |file| file.for_each_client(|id| f(id)))
	}

	pub fn move_cursor(&self, path: &PathBuf, id: ThreadId, offset: isize) -> EditrResult<()> {
		self.file_op(path, |file| file.move_cursor(id, offset))
	}

	pub fn file_write_cursor(
		&self,
		path: &PathBuf,
		id: ThreadId,
		data: &[u8],
	) -> EditrResult<usize> {
		self.file_op(path, |file| file.write_at_cursor(id, data))
	}

	pub fn file_remove_cursor(
		&self,
		path: &PathBuf,
		id: ThreadId,
		len: usize,
	) -> EditrResult<usize> {
		self.file_op(path, |file| file.remove_at_cursor(id, len))
	}

	pub fn get_cursors(
		&self,
		path: &PathBuf,
		id: ThreadId,
	) -> EditrResult<(usize, Vec<(usize, Option<String>)>)> {
		self.file_op(path, |file| file.get_cursors(id))
	}

	// Applies an op that requires a read lock on the underlying container
	fn op<T, F: FnOnce(RwLockReadGuard<HashMap<PathBuf, FileState>>) -> EditrResult<T>>(
		&self,
		op: F,
	) -> EditrResult<T> {
		op(self.container.read())
	}

	// Applies an op that requires a write lock on the underlying container
	fn mut_op<T, F: FnOnce(RwLockWriteGuard<HashMap<PathBuf, FileState>>) -> EditrResult<T>>(
		&self,
		op: F,
	) -> EditrResult<T> {
		op(self.container.write())
	}

	// Applies an op on path's FileState
	fn file_op<T, F: FnOnce(&FileState) -> EditrResult<T>>(
		&self,
		path: &PathBuf,
		op: F,
	) -> EditrResult<T> {
		self.op(|container| {
			op(container
				.get(path)
				.ok_or("Thread local storage does not exist")?)
		})
	}
}

// Loads contents of file at path into a Rope
fn read_to_rope(path: &PathBuf) -> EditrResult<Rope> {
	let mut buffer = Vec::new();
	let mut file = File::open(&path)?;
	file.read_to_end(&mut buffer)?;

	let rope = Rope::new();
	rope.insert_at(0, &buffer)?;
	Ok(rope)
}
