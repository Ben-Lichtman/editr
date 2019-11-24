use std::collections::HashMap;
use std::error::Error;
use std::ops::Deref;
use std::sync::{Mutex, MutexGuard};
use std::thread::ThreadId;

use crate::error::EditrResult;
use crate::rope::Rope;

pub(super) struct FileState {
	rope: Rope,
	clients: Mutex<HashMap<ThreadId, (usize, Option<String>)>>,
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
	pub fn add_client(&self, id: ThreadId, name: Option<String>) -> EditrResult<()> {
		self.clients_op(|mut clients| Ok(clients.insert(id, (0, name))))?;
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
			for (key, _) in clients.iter() {
				f(*key)?;
			}
			Ok(())
		})
	}

	pub fn move_cursor(&self, id: ThreadId, offset: isize) -> EditrResult<()> {
		Ok(self.clients_op(|mut clients| {
			if let Some((found_offset, name)) = clients.get(&id) {
				let name_clone = name.clone();
				let new_offset_signed = *found_offset as isize + offset;
				let new_offset_unsigned = new_offset_signed as usize;
				clients.insert(id, (new_offset_unsigned, name_clone));
			}
			Ok(())
		})?)
	}

	pub fn write_at_cursor(&self, id: ThreadId, data: &[u8]) -> EditrResult<usize> {
		self.clients_op(|mut clients| {
			let found_value = match clients.get(&id) {
				Some((found_offset, _)) => *found_offset,
				None => return Err("ID not found in clients".into()),
			};

			self.insert_at(found_value, data)?;

			for (_, (found_offset, _)) in clients.iter_mut() {
				if *found_offset >= found_value {
					let new_offset_signed = *found_offset as isize + data.len() as isize;
					*found_offset = new_offset_signed as usize;
				}
			}
			Ok(found_value)
		})
	}

	pub fn remove_at_cursor(&self, id: ThreadId, len: usize) -> EditrResult<usize> {
		Ok(self.clients_op(|mut clients| {
			let found_value = match clients.get(&id) {
				Some((found_offset, _)) => *found_offset,
				None => return Err("ID not found in clients".into()),
			};

			self.remove_range(found_value, found_value + len)?;

			for (_, (found_offset, _)) in clients.iter_mut() {
				if *found_offset >= found_value {
					let new_offset_signed = *found_offset as isize - len as isize;
					let new_offset_signed = if new_offset_signed < found_value as isize {
						found_value
					}
					else {
						new_offset_signed as usize
					};
					*found_offset = new_offset_signed as usize;
				}
			}
			Ok(found_value)
		})?)
	}

	pub fn get_cursors(&self, id: ThreadId) -> EditrResult<(usize, Vec<(usize, Option<String>)>)> {
		Ok(self.clients_op(|clients| {
			let found_value = match clients.get(&id) {
				Some((found_offset, _)) => *found_offset,
				None => return Err("ID not found in clients".into()),
			};

			let others = clients
				.iter()
				.map(|(_, (found_offset, name))| (*found_offset, name.clone()))
				.collect();

			Ok((found_value, others))
		})?)
	}

	// Locks clients and applies op
	fn clients_op<
		T,
		F: FnOnce(MutexGuard<HashMap<ThreadId, (usize, Option<String>)>>) -> EditrResult<T>,
	>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.clients.lock().map_err(|e| e.to_string())?)
	}
}
