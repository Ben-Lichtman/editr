pub mod shared_out;
mod thread_io;

use std::net::TcpStream;
use std::thread::ThreadId;

use crate::error::EditrResult;
use shared_out::SharedOut;
use thread_io::ThreadIn;

pub struct Socket {
	local_in: ThreadIn,
	shared_out: SharedOut,
}

impl Socket {
	pub fn new(thread_id: ThreadId, stream: TcpStream, out: SharedOut) -> EditrResult<Socket> {
		out.insert(thread_id, stream.try_clone()?)?;
		Ok(Socket {
			local_in: ThreadIn::new(stream)?,
			shared_out: out,
		})
	}

	// Reads from reader into buffer
	pub fn read(&mut self, buf: &mut [u8]) -> EditrResult<usize> { self.local_in.read(buf) }

	// Writes from buffer into thread_id's writer
	pub fn write(&self, thread_id: ThreadId, buf: &[u8]) -> EditrResult<usize> {
		self.shared_out.write(thread_id, buf)
	}

	// Closes the socket
	pub fn close(&self, thread_id: ThreadId) -> EditrResult<()> {
		self.shared_out.remove(thread_id)
	}
}
