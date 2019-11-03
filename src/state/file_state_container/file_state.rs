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
		self.clients_op(|mut clients| clients.insert(id))?;
		Ok(())
	}

	// Removes a client by their ThreadId
	pub fn remove_client(&self, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.clients_op(|mut clients| clients.remove(id))?;
		Ok(())
	}

	// Returns true if self doesn't have any clients
	pub fn no_clients(&self) -> Result<bool, Box<dyn Error>>> {
		self.clients_op(|clients| clients.is_empty())?
	}

	// Returns a vector of read bytes starting from 'from' and ending at 'to'
	pub fn read(&self, from: usize, to: usize) -> Result<Vec<u8>, Box<dyn Error>> {
		self.rope.collect(from, to)
	}

	// Writes bytes in data into the file at offset
	pub fn write(&self, offset: usize, data: &[u8]) -> Result<(), Box<dyn Error>> {
		self.rope.insert_at(offset, data)
	}

	// Deletes from the file, starting from offset
	pub fn delete(&self, offset: usize, len: usize) Result<(), Box<dyn Error>> {
		self.rope.remove_range(offset, offset + len)
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
