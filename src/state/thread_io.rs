use std::error::Error;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::sync::{Mutex, MutexGuard};

pub struct ThreadIO {
	stream: Mutex<IOBuffers>,
}

impl ThreadIO {
	pub fn new(stream: TcpStream) -> ThreadIO {
		ThreadIO {
			stream: Mutex::new(IOBuffers::new(stream)),
		}
	}

	// Locks stream and applies op
	pub fn apply<T, F: FnOnce(MutexGuard<IOBuffers>) -> Result<T, Box<dyn Error>>>(
		&self,
		op: F,
	) -> Result<T, Box<dyn Error>> {
		op(self.stream.lock().map_err(|e| e.to_string())?)
	}
}

pub struct IOBuffers {
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

	// Reads from reader into buffer
	pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Box<dyn Error>> {
		Ok(self.reader.read(buffer)?)
	}

	// Writes from buffer into writer
	pub fn write(&mut self, buffer: &[u8]) -> Result<usize, Box<dyn Error>> {
		Ok(self.writer.write(buffer)?)
	}
}
