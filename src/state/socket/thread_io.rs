use std::io::{BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::sync::Mutex;

use crate::error::EditrResult;
use crate::message::Message;

use serde_json::de::IoRead;
use serde_json::{Deserializer, StreamDeserializer};

pub(super) struct ThreadIn {
	reader: StreamDeserializer<'static, IoRead<BufReader<TcpStream>>, Message>,
}

impl ThreadIn {
	pub fn new(stream: TcpStream) -> EditrResult<ThreadIn> {
		let reader_copy = stream.try_clone()?;
		Ok(ThreadIn {
			reader: Deserializer::from_reader(BufReader::new(reader_copy)).into_iter(),
		})
	}

	pub fn get_message(&mut self) -> EditrResult<Message> {
		Ok(self
			.reader
			.next()
			.ok_or("Could not get message")
			.map_err(|e| e.to_string())?
			.map_err(|e| e.to_string())?)
	}
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
