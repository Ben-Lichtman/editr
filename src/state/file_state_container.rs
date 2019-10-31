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
}
