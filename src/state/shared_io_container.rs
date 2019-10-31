use std::collections::HashMap;
use std::error::Error;
use std::net::TcpStream;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

use super::thread_io::ThreadIO;

#[derive(Default)]
pub struct SharedIOContainer {
	shared_io: RwLock<HashMap<ThreadId, ThreadIO>>,
}

impl SharedIOContainer {
	// Constructs empty SharedIOContainer
	pub fn new() -> SharedIOContainer {
		SharedIOContainer {
			shared_io: RwLock::new(HashMap::new()),
		}
	}

	// Inserts a new stream
	pub fn insert(&self, thread_id: ThreadId, stream: TcpStream) -> Result<(), Box<dyn Error>> {
		self.write_op(|mut container| {
			container.insert(thread_id, ThreadIO::new(stream));
			Ok(())
		})
	}

	// Removes thread_id's stream
	pub fn remove(&self, thread_id: ThreadId) -> Result<(), Box<dyn Error>> {
		self.write_op(|mut container| {
			container.remove(&thread_id);
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
		self.thread_io_op(thread_id, |io| io.apply(|mut stream| stream.read(buffer)))
	}

	// Given a valid thread_id, reads from its stream and
	// places read data into buffer
	pub fn socket_write(
		&self,
		thread_id: ThreadId,
		buffer: &[u8],
	) -> Result<usize, Box<dyn Error>> {
		self.thread_io_op(thread_id, |io| io.apply(|mut stream| stream.write(buffer)))
	}

	// Performs an operation that requires read access to the
	// underlying container
	fn read_op<
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
	fn write_op<
		T,
		F: FnOnce(RwLockWriteGuard<HashMap<ThreadId, ThreadIO>>) -> Result<T, Box<dyn Error>>,
	>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.shared_io.write().map_err(|e| e.to_string())?)
	}

	// Performs an operation on ThreadIO object belonging to id
	fn thread_io_op<T, F: FnOnce(&ThreadIO) -> Result<T, Box<dyn Error>>>(
		&self,
		id: ThreadId,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		self.read_op(|container| {
			op(container
				.get(&id)
				.ok_or("Thread local storage does not exist")?)
		})
	}
}
