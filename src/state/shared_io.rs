use std::collections::HashMap;
use std::error::Error;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::ThreadId;

type SharedIOResult<T> = Result<T, Box<dyn Error>>;

struct SharedIOInner {
	reader: Mutex<BufReader<TcpStream>>,
	writer: Mutex<BufWriter<TcpStream>>,
}

impl SharedIOInner {
	fn new(stream: TcpStream) -> SharedIOResult<SharedIOInner> {
		let reader_copy = stream.try_clone()?;
		let writer_copy = stream.try_clone()?;

		Ok(SharedIOInner {
			reader: Mutex::new(BufReader::new(reader_copy)),
			writer: Mutex::new(BufWriter::with_capacity(0, writer_copy)),
		})
	}

	fn read(&self, buf: &mut [u8]) -> SharedIOResult<usize> {
		Ok(self.reader.lock().map_err(|e| e.to_string())?.read(buf)?)
	}

	fn write(&self, buf: &[u8]) -> SharedIOResult<usize> {
		Ok(self.writer.lock().map_err(|e| e.to_string())?.write(buf)?)
	}
}

#[derive(Clone)]
pub struct SharedIO {
	inner: Arc<RwLock<HashMap<ThreadId, SharedIOInner>>>,
}

impl SharedIO {
	pub fn new() -> SharedIO {
		SharedIO {
			inner: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	pub fn add(&self, id: ThreadId, stream: TcpStream) -> SharedIOResult<()> {
		let new = SharedIOInner::new(stream)?;
		self.hashmap_mut_op(|mut m| {
			m.insert(id, new);
			Ok(())
		})
	}

	pub fn remove(&self, id: ThreadId) -> SharedIOResult<()> {
		self.hashmap_mut_op(|mut m| {
			m.remove(&id);
			Ok(())
		})
	}

	pub fn read(&self, id: ThreadId, buf: &mut [u8]) -> SharedIOResult<usize> {
		self.shared_io_op(id, |s| s.read(buf))
	}

	pub fn write(&self, id: ThreadId, buf: &[u8]) -> SharedIOResult<usize> {
		self.shared_io_op(id, |s| s.write(buf))
	}

	fn shared_io_op<T, F: FnOnce(&SharedIOInner) -> SharedIOResult<T>>(
		&self,
		id: ThreadId,
		f: F,
	) -> SharedIOResult<T> {
		self.hashmap_op(|m| {
			let value = m.get(&id).ok_or("Shared IO op failed".to_string())?;
			f(value)
		})
	}

	fn hashmap_op<
		T,
		F: FnOnce(RwLockReadGuard<HashMap<ThreadId, SharedIOInner>>) -> SharedIOResult<T>,
	>(
		&self,
		f: F,
	) -> SharedIOResult<T> {
		f(self.inner.read().map_err(|e| e.to_string())?)
	}

	fn hashmap_mut_op<
		T,
		F: FnOnce(RwLockWriteGuard<HashMap<ThreadId, SharedIOInner>>) -> SharedIOResult<T>,
	>(
		&self,
		f: F,
	) -> SharedIOResult<T> {
		f(self.inner.write().map_err(|e| e.to_string())?)
	}
}
