use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::fs::File;
use std::io::Read;

use crate::rope::Rope;

pub struct FileStateContainer {
	container: RwLock<HashMap<PathBuf, FileState>>
}

impl FileStateContainer {
	pub fn new() -> FileStateContainer {
		FileStateContainer {
			container: RwLock::new(HashMap::new()),
		}
	}

	// True if container contains file at path
	pub fn contains(&self, path: &PathBuf) ->Result<bool, Box<dyn Error>> {
		self.read_op(|container| Ok(container.contains_key(path)))
	}

	// Opens a file at path for the client.
	// If the file isn't in container, it will be read in.
	// TODO: Get rid of race on self.contains(path)
	// e.g. thread A, B may overwrite each other if they both get past it
	pub fn file_open(&self, path: &PathBuf, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		// Read into container if not present
		if !self.contains(path)? {
			let rope = read_to_rope(path)?;
			let state = FileState::new(rope).add_client(id)?;
			self.write_op(|container| {
				container.insert(path, state)?;
			})
		}
		else {
			self.state_op(path, |state| {
				state.add_client(id)?;
			})
		}
		Ok(())
	}

	//pub fn file_close(&self, path: &PathBuf, id: &ThreadId) -> Result<(), Box<dyn Error>> {
	//	
	//}

	// Removes FileState at path
	pub fn remove(&self, path: &PathBuf) -> Result<(), Box<dyn Error>>{
		self.write_op(|container| container.remove(path));
		Ok(())
	}

	// Adds a new client to the FileState at path
	pub fn add_client(&self, path: &PathBuf, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.state_op(path, |state| state.add_client(id));
		Ok(())
	}

	// Removes client from FileState at path
	pub fn add_client(&self, path: &PathBuf, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.state_op(path, |state| state.remove_client(id));
		Ok(())
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
	fn state_op<
		T,
		F: FnOnce(&FileState) -> Result<T, Box<dyn Error>>,
	>(
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
	let mut file = File::open(&canonical_path)?;
	file.read_to_end(&mut buffer)?;

	let rope = Rope::new();
	rope.insert_at(0, &buffer)?;
	rope
}
