use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;

use crate::error::EditrResult;

pub struct ThreadIO {
	reader: Mutex<BufReader<TcpStream>>,
	writer: Mutex<BufWriter<TcpStream>>,
}

impl ThreadIO {
	pub fn new(stream: TcpStream) -> EditrResult<ThreadIO> {
		let reader_copy = stream.try_clone()?;
		let writer_copy = stream.try_clone()?;

		Ok(ThreadIO {
			reader: Mutex::new(BufReader::new(reader_copy)),
			writer: Mutex::new(BufWriter::with_capacity(0, writer_copy)),
		})
	}

	// Reads from reader into buffer
	pub fn read(&self, buf: &mut [u8]) -> EditrResult<usize> {
		Ok(self.reader.lock().map_err(|e| e.to_string())?.read(buf)?)
	}

	// Writes from buffer into writer
	pub fn write(&self, buf: &[u8]) -> EditrResult<usize> {
		Ok(self.writer.lock().map_err(|e| e.to_string())?.write(buf)?)
	}
}
