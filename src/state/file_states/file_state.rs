use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::sync::{Mutex, MutexGuard};
use std::thread::ThreadId;

use crate::error::EditrResult;
use crate::rope::Rope;

pub(super) struct FileState {
	rope: Rope,
	clients: Mutex<HashMap<ThreadId, usize>>,
}

impl Deref for FileState {
	type Target = Rope;
	fn deref(&self) -> &Self::Target { &self.rope }
}

impl FileState {
	pub fn new(rope: Rope) -> FileState {
		FileState {
			rope,
			clients: Mutex::new(HashMap::new()),
		}
	}

	// Inserts a new client by their ThreadId
	pub fn add_client(&self, id: ThreadId) -> EditrResult<()> {
		self.clients_op(|mut clients| Ok(clients.insert(id.clone(), 0)))?;
		Ok(())
	}

	// Removes a client by their ThreadId
	pub fn remove_client(&self, id: ThreadId) -> EditrResult<()> {
		self.clients_op(|mut clients| Ok(clients.remove(&id)))?;
		Ok(())
	}

	// Returns true if self doesn't have any clients
	pub fn no_clients(&self) -> EditrResult<bool> {
		Ok(self.clients_op(|clients| Ok(clients.is_empty()))?)
	}

	// Calls a closure f on each client
	pub fn for_each_client<F: Fn(ThreadId) -> EditrResult<()>>(
		&self,
		f: F,
	) -> Result<(), Box<dyn Error>> {
		self.clients_op(|clients| {
			for (key, _val) in clients.iter() {
				f(*key)?;
			}
			Ok(())
		})
	}

	pub fn move_cursor(&self, id: ThreadId, offset: isize) -> EditrResult<()> {
		Ok(self.clients_op(|mut clients| {
			if let Some(value) = clients.get(&id) {
				let new_offset_signed = *value as isize + offset;
				let new_offset_unsigned = new_offset_signed as usize;
				clients.insert(id, new_offset_unsigned);
			}
			Ok(())
		})?)
	}

	pub fn write_at_cursor(&self, id: ThreadId, data: &[u8]) -> EditrResult<()> {
		self.clients_op(|mut clients| {
			let found_value = match clients.get(&id) {
				Some(value) => *value,
				None => return Err("ID not found in clients".into()),
			};

			self.insert_at(found_value, data)?;

			for (_, value) in clients.iter_mut() {
				if *value >= found_value {
					let new_offset_signed = *value as isize + data.len() as isize;
					*value = new_offset_signed as usize;
				}
			}
			Ok(())
		})?;
		Ok(())
	}

	pub fn remove_at_cursor(&self, id: ThreadId, len: usize) -> EditrResult<()> {
		Ok(self.clients_op(|mut clients| {
			let found_value = match clients.get(&id) {
				Some(value) => *value,
				None => return Err("ID not found in clients".into()),
			};

			self.remove_range(found_value, found_value + len)?;

			for (_, value) in clients.iter_mut() {
				if *value >= found_value {
					let new_offset_signed = *value as isize - len as isize;
					*value = new_offset_signed as usize;
				}
			}
			Ok(())
		})?)
	}

	pub fn get_cursors(&self, id: ThreadId) -> EditrResult<(usize, Vec<usize>)> {
		Ok(self.clients_op(|clients| {
			let found_value = match clients.get(&id) {
				Some(value) => *value,
				None => return Err("ID not found in clients".into()),
			};

			let others = clients.iter().map(|(_, value)| *value).collect();

			Ok((found_value, others))
		})?)
	}

	// Locks clients and applies op
	fn clients_op<T, F: FnOnce(MutexGuard<HashMap<ThreadId, usize>>) -> EditrResult<T>>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.clients.lock().map_err(|e| e.to_string())?)
	}
}
