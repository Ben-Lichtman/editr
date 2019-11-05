mod thread_io;

use std::collections::HashMap;
use std::error::Error;
use std::net::TcpStream;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

use thread_io::ThreadIO;

#[derive(Default, Clone)]
pub struct SharedIO {
	shared_io: Arc<RwLock<HashMap<ThreadId, ThreadIO>>>,
}

impl SharedIO {
	// Constructs empty SharedIOContainer
	pub fn new() -> SharedIO {
		SharedIO {
			shared_io: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	// Inserts a new stream
	pub fn insert(&self, thread_id: ThreadId, stream: TcpStream) -> Result<(), Box<dyn Error>> {
		self.hashmap_mut_op(|mut hashmap| {
			hashmap.insert(thread_id, ThreadIO::new(stream)?);
			Ok(())
		})
	}

	// Removes thread_id's stream
	pub fn remove(&self, thread_id: ThreadId) -> Result<(), Box<dyn Error>> {
		self.hashmap_mut_op(|mut hashmap| {
			hashmap.remove(&thread_id);
			Ok(())
		})
	}

	// Given a valid thread_id, reads from its stream and
	// places read data into buffer
	pub fn socket_read(
		&self,
		thread_id: ThreadId,
		buffer: &mut [u8],
	) -> Result<usize, Box<dyn Error>> {
		self.thread_io_op(thread_id, |io| io.read(buffer))
	}

	// Given a valid thread_id, reads from its stream and
	// places read data into buffer
	pub fn socket_write(
		&self,
		thread_id: ThreadId,
		buffer: &[u8],
	) -> Result<usize, Box<dyn Error>> {
		self.thread_io_op(thread_id, |io| io.write(buffer))
	}

	// Performs an operation on ThreadIO object belonging to id
	fn thread_io_op<T, F: FnOnce(&ThreadIO) -> Result<T, Box<dyn Error>>>(
		&self,
		id: ThreadId,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		self.hashmap_op(|hashmap| {
			op(hashmap
				.get(&id)
				.ok_or("Thread local storage does not exist")?)
		})
	}

	// Performs an operation that requires read access to the
	// underlying container
	fn hashmap_op<
		T,
		F: FnOnce(RwLockReadGuard<HashMap<ThreadId, ThreadIO>>) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.shared_io.read().map_err(|e| e.to_string())?)
	}

	// Performs an operation that requires write access to the
	// underlying container
	fn hashmap_mut_op<
		T,
		F: FnOnce(RwLockWriteGuard<HashMap<ThreadId, ThreadIO>>) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.shared_io.write().map_err(|e| e.to_string())?)
	}
}
