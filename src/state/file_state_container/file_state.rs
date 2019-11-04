use std::collections::HashSet;
use std::error::Error;
use std::ops::Deref;
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
		self.clients_op(|mut clients| Ok(clients.insert(id.clone())))?;
		Ok(())
	}

	// Removes a client by their ThreadId
	pub fn remove_client(&self, id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.clients_op(|mut clients| Ok(clients.remove(id)))?;
		Ok(())
	}

	// Returns true if self doesn't have any clients
	pub fn no_clients(&self) -> Result<bool, Box<dyn Error>> {
		Ok(self.clients_op(|clients| Ok(clients.is_empty()))?)
	}

	// Calls a closure f on each client
	pub fn for_each_client<F: Fn(&ThreadId) -> Result<(), Box<dyn Error>>>(
		&self,
		f: F,
	) -> Result<(), Box<dyn Error>> {
		self.clients_op(|clients| {
			for c in clients.iter() {
				f(c)?;
			}
			Ok(())
		})
	}

	// Locks clients and applies op
	fn clients_op<T, F: FnOnce(MutexGuard<HashSet<ThreadId>>) -> Result<T, Box<dyn Error>>>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.clients.lock().map_err(|e| e.to_string())?)
	}
}
