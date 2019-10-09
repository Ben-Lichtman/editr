extern crate memmap;

mod rope;

use std::env;
use std::fs::{File, OpenOptions};
use memmap::{Mmap, MmapOptions};
use rope::Rope;
use std::collections::BTreeMap;
use std::slice::Iter;

struct Change {
	position: usize,
	rope: Rope,
	deletions: usize,
}

struct State {
	file: File,
	changes: BTreeMap<u64, Change>,
}

fn open_file(path: &str) -> File {
	OpenOptions::new()
	.read(true)
	.write(true)
	.create(true)
	.open(path)
	.unwrap()
}

fn get_mapping(file: &File, offset: u64, mapping_size: u64) -> Mmap {
	let meta = file.metadata().unwrap();

	let length = match meta.len().checked_sub(offset) {
		Some(x) if x < mapping_size => x,
		Some(_) => mapping_size,
		None => 0,
	};

	unsafe {
		MmapOptions::new()
		.offset(offset)
		.len(length as usize)
		.map(&file)
		.unwrap()
	}
}

impl State {
	fn iter_over(&self, from: u64, to: u64) {
		assert!(from <= to);
		let mmap = get_mapping(&self.file, from, to - from);

		let s: Vec<u8> = mmap.iter().copied().collect();
		let s = s.as_slice();
		let string = std::str::from_utf8(s).unwrap();
		println!("{:?}", string);
	}

	fn insert(&mut self, at: u64, data: Vec<u8>) {

	}

	fn remove(&mut self, at: u64, count: u64) {

	}
}

fn main() {
	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		println!("Please provide a file");
		return;
	}
	let path = &args[1];
	let offset: u64 = if args.len() < 3 {
		0
	}
	else {
		args[2].parse().unwrap()
	};

	let open_file = open_file(path);

	let state = State {
		file: open_file,
		changes: BTreeMap::new(),
	};

	loop {
		let curpos: u64 = 0;
		let num_bytes = 10000;

		let mut ret = String::new();
		std::io::stdin().read_line(&mut ret);
	}
}
