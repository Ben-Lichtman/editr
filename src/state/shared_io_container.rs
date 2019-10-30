use std::collections::HashMap;
use std::error::Error;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

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
	pub fn insert(&self, thread_id: &ThreadId, stream: TcpStream) -> Result<(), Box<dyn Error>> {
		self.write_op(|mut container| {
			container.insert(thread_id.clone(), ThreadIO::new(stream));
			Ok(())
		})
	}

	// Removes thread_id's stream
	pub fn remove(&self, thread_id: &ThreadId) -> Result<(), Box<dyn Error>> {
		self.write_op(|mut container| {
			container.remove(thread_id);
			Ok(())
		})
	}

	// Given a valid thread_id, reads from its stream and
	// places read data into buffer
	pub fn socket_read(
		&self,
		thread_id: &ThreadId,
		buffer: &mut [u8],
	) -> Result<usize, Box<dyn Error>> {
		//Ok(self.get_thread_io(thread_id)?
		//		.read(buffer).map_err(|e| e.to_string())?)
		//self.read_lock()?.get(thread_id).ok_or("no")?.apply(|m| m.read(buffer))
		self.thread_io_op(thread_id, |io| io.apply(|mut stream| stream.read(buffer)))
	}

	// Given a valid thread_id, reads from its stream and
	// places read data into buffer
	pub fn socket_write(
		&self,
		thread_id: &ThreadId,
		buffer: &[u8],
	) -> Result<usize, Box<dyn Error>> {
		//Ok(self.get_thread_io(thread_id)?
		//		.write(buffer).map_err(|e| e.to_string())?)
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
		id: &ThreadId,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		self.read_op(|container| {
			op(container
				.get(&id)
				.ok_or("Thread local storage does not exist")?)
		})
	}

	//// Acquires a read lock on underlying container
	//fn read_lock(&self) -> Result<RwLockReadGuard<IOContainer>, String> {
	//	self.shared_io.read().map_err(|e| e.to_string())
	//}

	//// Acquires a write lock on underlying container
	//fn write_lock(&self) -> Result<RwLockWriteGuard<IOContainer>, String> {
	//	self.shared_io.write().map_err(|e| e.to_string())
	//}

	// Acquires a mutex on the ThreadIO matching thread_id
	//fn get_thread_io(&, thread_id: &ThreadId) -> Result<ThreadIO>, String> {
	//self.read_lock()?
	//	.get(thread_id)
	//	.ok_or("Thread local storage does not exist")?
	//	.lock()
	//	.map_err(|e| e.to_string())
	//}
}

struct ThreadIO {
	stream: Mutex<IOBuffers>,
}

impl ThreadIO {
	pub fn new(stream: TcpStream) -> ThreadIO {
		ThreadIO {
			stream: Mutex::new(IOBuffers::new(stream)),
		}
	}

	pub fn apply<T, F: FnOnce(MutexGuard<IOBuffers>) -> Result<T, Box<dyn Error>>>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.stream.lock().map_err(|e| e.to_string())?)
	}
}

struct IOBuffers {
	reader: BufReader<TcpStream>,
	writer: BufWriter<TcpStream>,
}

impl IOBuffers {
	pub fn new(stream: TcpStream) -> IOBuffers {
		IOBuffers {
			reader: BufReader::new(stream.try_clone().unwrap()),
			writer: BufWriter::with_capacity(0, stream.try_clone().unwrap()),
		}
	}

	pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error>> {
		Ok(self.reader.read(buffer)?)
	}

	pub fn write(&mut self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
		Ok(self.writer.write(buffer)?)
	}
}
