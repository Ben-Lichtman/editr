mod rope;

fn main() {
	let r = rope::Rope::new();
	println!("{:#?}", r);

	r.insert_at(0, "A".as_bytes()).unwrap();
	println!("{:#?}", r);

	r.insert_at(0, "B".as_bytes()).unwrap();
	println!("{:#?}", r);

	r.insert_at(0, "C".as_bytes()).unwrap();
	println!("{:#?}", r);

	r.remove_range(2, 3).unwrap();
	println!("{:#?}", r);

	r.insert_at(2, "HELLO WORLD".as_bytes()).unwrap();
	println!("{:#?}", r);

	r.remove_range(0, 2).unwrap();
	println!("{:#?}", r);

	r.flatten().unwrap();
	println!("{:#?}", r);
}
