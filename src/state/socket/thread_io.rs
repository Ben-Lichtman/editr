use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;

use crate::error::EditrResult;

pub(super) struct ThreadIn {
	reader: BufReader<TcpStream>,
}

impl ThreadIn {
	pub fn new(stream: TcpStream) -> EditrResult<ThreadIn> {
		let reader_copy = stream.try_clone()?;
		Ok(ThreadIn {
			reader: BufReader::new(reader_copy),
		})
	}

	// Reads from reader into buffer
	pub fn read(&mut self, buf: &mut [u8]) -> EditrResult<usize> { Ok(self.reader.read(buf)?) }
}

pub(super) struct ThreadOut {
	writer: Mutex<BufWriter<TcpStream>>,
}

impl ThreadOut {
	pub fn new(stream: TcpStream) -> EditrResult<ThreadOut> {
		let writer_copy = stream.try_clone()?;
		Ok(ThreadOut {
			writer: Mutex::new(BufWriter::with_capacity(0, writer_copy)),
		})
	}

	// Writes from buffer into writer
	pub fn write(&self, buf: &[u8]) -> EditrResult<usize> {
		Ok(self.writer.lock().map_err(|e| e.to_string())?.write(buf)?)
	}
}
