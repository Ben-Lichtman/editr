use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::Arc;
use std::thread::ThreadId;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::thread_io::ThreadOut;
use crate::error::EditrResult;

#[derive(Default, Clone)]
pub struct SharedOut {
	shared_out: Arc<RwLock<HashMap<ThreadId, ThreadOut>>>,
}

impl SharedOut {
	// Constructs empty SharedOutContainer
	pub fn new() -> SharedOut {
		SharedOut {
			shared_out: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	// Inserts a new stream
	pub fn insert(&self, thread_id: ThreadId, stream: TcpStream) -> EditrResult<()> {
		self.hashmap_mut_op(|mut hashmap| {
			hashmap.insert(thread_id, ThreadOut::new(stream)?);
			Ok(())
		})
	}

	// Removes thread_id's stream
	pub fn remove(&self, thread_id: ThreadId) -> EditrResult<()> {
		self.hashmap_mut_op(|mut hashmap| {
			hashmap.remove(&thread_id);
			Ok(())
		})
	}

	// Given a valid thread_id, reads from its stream and
	// places read data into buffer
	pub fn write(&self, thread_id: ThreadId, buffer: &[u8]) -> EditrResult<usize> {
		self.thread_out_op(thread_id, |io| io.write(buffer))
	}

	// Performs an operation on ThreadOut object belonging to id
	fn thread_out_op<T, F: FnOnce(&ThreadOut) -> EditrResult<T>>(
		&self,
		id: ThreadId,
		op: F,
	) -> EditrResult<T> {
		self.hashmap_op(|hashmap| {
			op(hashmap
				.get(&id)
				.ok_or("Thread local storage does not exist")?)
		})
	}

	// Performs an operation that requires read access to the
	// underlying container
	fn hashmap_op<T, F: FnOnce(RwLockReadGuard<HashMap<ThreadId, ThreadOut>>) -> EditrResult<T>>(
		&self,
		op: F,
	) -> EditrResult<T> {
		op(self.shared_out.read())
	}

	// Performs an operation that requires write access to the
	// underlying container
	fn hashmap_mut_op<
		T,
		F: FnOnce(RwLockWriteGuard<HashMap<ThreadId, ThreadOut>>) -> EditrResult<T>,
	>(
		&self,
		op: F,
	) -> EditrResult<T> {
		op(self.shared_out.write())
	}
}
