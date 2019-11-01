use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct FileStateContainer {
	container: RwLock<HashMap<PathBuf, FileState>>
}

impl FileStateContainer {
	pub fn new() -> FileStateContainer {
		FileStateContainer {
			container: RwLock::new(HashMap::new()),
		}
	}
	
	// Inserts a FileState at path
	pub fn insert(&self, path: &PathBuf, val: FileState) -> Result<(), Box<dyn Error>>{
		self.write_op(|mut container| container.insert(path, val))
		Ok(())
	}
	
	// Removes FileState at path
	pub fn remove(&self, path: &PathBuf) -> Result<(), Box<dyn Error>>{
		self.write_op(|mut container| container.remove(path))
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
}
