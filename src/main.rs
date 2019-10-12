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
}
