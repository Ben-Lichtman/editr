use std::collections::HashSet;

use crate::rope::Rope;

pub struct FileState {
	rope: Rope,
	clients: HashSet<ThreadId>,
}

impl Deref for FileState {
	type Target = Rope;
	fn deref(&self) -> &Self::Target { &self.rope }
}

impl FileState {
	pub fn new(rope: Rope) -> FileState {
		FileState {
			rope,
			clients: HashSet::new(),
		}
	}
}
