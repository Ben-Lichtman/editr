use editr::rope::Rope;

fn print_rope(r: &Rope, from: usize, to: usize) {
	let c = r.collect(from, to).unwrap();
	println!("{:?}", std::str::from_utf8(&c).unwrap());
}

fn main() {
	let r = Rope::new();
	// println!("{:#?}", r);

	r.insert_at(0, "ABCDEFGH".as_bytes()).unwrap();
	// print_rope(&r, 0, 9000);

	r.insert_at(0, "1234567".as_bytes()).unwrap();
	// print_rope(&r, 0, 9000);

	// r.remove_range(2, 3).unwrap();
	// println!("{:#?}", r);

	// for _ in 0..10 {
	// 	r.insert_at(3, &[0x33]).unwrap();
	// }

	// r.remove_range(0, 2).unwrap();
	// println!("{:#?}", r);

	print_rope(&r, 0, r.len().unwrap());
}
