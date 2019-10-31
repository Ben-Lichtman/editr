use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};
use std::thread::ThreadId;

use crate::rope::Rope;

pub(super) struct FileState {
	rope: Rope,
	clients: Mutex<HashSet<ThreadId>>,
}

impl Deref for FileState {
	type Target = Rope;
	fn deref(&self) -> &Self::Target { &self.rope }
}

impl FileState {
	pub fn new(rope: Rope) -> FileState {
		FileState {
			rope,
			clients: Mutex::new(HashSet::new()),
		}
	}

	// Inserts a new client by their ThreadId
	pub fn add_client(&self, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.clients_op(|clients| clients.insert(id))?;
		Ok(())
	}

	// Removes a client by their ThreadId
	pub fn remove_client(&self, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.clients_op(|clients| clients.remove(id))?;
		Ok(())
	}

	// Locks clients and applies op
	fn clients_op<
		T,
		F: FnOnce(MutexGuard<HashSet<ThreadId>) -> Result<T, Box<dyn Error>>>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.clients.lock().map_err(|e| e.to_string())?)
	}
}
