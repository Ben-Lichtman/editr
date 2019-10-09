use std::slice::Iter;

#[derive(Debug)]
pub struct Rope {
	root: Node,
}

#[derive(Debug)]
enum Node {
	Leaf(LeafData),
	Internal(InternalData),
	Virtual(VirtualData),
}

#[derive(Debug)]
struct LeafData {
	data: Vec<u8>,
}

#[derive(Debug)]
struct InternalData {
	index: usize,
	size: usize,
	children: Box<(Node, Node)>,
}

#[derive(Debug)]
struct VirtualData {
	size: usize,
	backing_offset: usize,
}

#[derive(Debug)]
struct NodeIter<'a> {
	stack: Vec<&'a Node>,
	cur_iter: Option<Iter<'a, u8>>,
}

impl<'a> NodeIter<'a> {
	fn new(n: &'a Node) -> NodeIter {
		NodeIter {
			stack: vec![n],
			cur_iter: None,
		}
	}

	fn new_leaf(&mut self) {
		self.cur_iter = loop {
			let popped = self.stack.pop();
			match popped {
				None => break None,
				Some(Node::Leaf(ld)) => break Some(ld.data.iter()),
				Some(Node::Internal(id)) => {
					self.stack.push(&id.children.1);
					self.stack.push(&id.children.0);
				},
				Some(Node::Virtual(vd)) => break {
					// TODO create iterator from file
				},
			}
		}
	}
}

impl<'a> Iterator for NodeIter<'a> {
	type Item = &'a u8;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if self.cur_iter.is_none() {
				self.new_leaf()
			}

			// No leaf found at this point = end of iteration
			let iter = self.cur_iter.as_mut()?;

			if let found @ Some(_) = iter.next() {
				return found
			}

			self.cur_iter = None;
		}
	}
}

impl Node {
	fn size(&self) -> usize {
		match self {
			Node::Leaf(ld) => ld.data.len(),
			Node::Internal(id) => id.size,
			Node::Virtual(vd) => vd.size,
		}
	}

	fn insert_at(self, mut input: Vec<u8>, index: usize) -> Node {
		match self {
			Node::Leaf(ld) => {
				let mut left_node_data = ld.data;
				let right_node_data = left_node_data.split_off(index);

				// Operate
				left_node_data.append(&mut input);

				let new_index = left_node_data.len();
				let new_size = new_index + right_node_data.len();

				let left_node = Node::Leaf(LeafData {
					data: left_node_data,
				});
				let right_node = Node::Leaf(LeafData {
					data: right_node_data,
				});

				if left_node.size() == 0 {
					return right_node;
				}

				if right_node.size() == 0 {
					return left_node;
				}

				Node::Internal(InternalData {
					index: new_index,
					size: new_size,
					children: Box::new((left_node, right_node)),
				})
			},
			Node::Internal(id) => {
				if index <= id.index {
					let left_child = id.children.0.insert_at(input, index);
					let right_child = id.children.1;

					if left_child.size() == 0 {
						return right_child;
					}

					Node::Internal(InternalData {
						index: left_child.size(),
						size: left_child.size() + right_child.size(),
						children: Box::new((left_child, right_child)),
					})
				}
				else {
					let left_child = id.children.0;
					let right_child = id.children.1.insert_at(input, index - id.index);

					if right_child.size() == 0 {
						return left_child;
					}

					Node::Internal(InternalData {
						index: left_child.size(),
						size: left_child.size() + right_child.size(),
						children: Box::new((left_child, right_child)),
					})
				}
			},
			Node::Virtual(vd) => {
				assert!(index <= vd.size);
				let left_left_child = Node::Virtual(VirtualData {
					size: index,
					backing_offset: vd.backing_offset,
				});
				let left_right_child = Node::Leaf(LeafData {
					data: input,
				});
				let left_child = Node::Internal(InternalData {
					index: index,
					size: left_left_child.size() + left_right_child.size(),
					children: Box::new((left_left_child, left_right_child)),
				});
				let right_child = Node::Virtual(VirtualData {
					size: vd.size - index,
					backing_offset: vd.backing_offset + index,
				});
				Node::Internal(InternalData {
					index: left_child.size(),
					size: left_child.size() + right_child.size(),
					children: Box::new((left_child, right_child)),
				})
			},
		}
	}

	fn remove_range(self, from: usize, to: usize) -> Node {
		match self {
			Node::Leaf(ld) => {
				assert!(from <= to && to <= ld.data.len());
				let mut left_node_data = ld.data;
				let right_node_data = left_node_data.split_off(to);

				// Operate
				left_node_data.truncate(from);

				let new_index = left_node_data.len();
				let new_size = new_index + right_node_data.len();

				let left_node = Node::Leaf(LeafData {
					data: left_node_data,
				});
				let right_node = Node::Leaf(LeafData {
					data: right_node_data,
				});

				if left_node.size() == 0 {
					return right_node;
				}

				if right_node.size() == 0 {
					return left_node;
				}

				Node::Internal(InternalData {
					index: new_index,
					size: new_size,
					children: Box::new((left_node, right_node)),
				})
			},
			Node::Internal(id) => {
				assert!(from <= to);
				let l_from = id.index.min(from);
				let l_to = id.index.min(to);
				let r_from = id.index.max(from) - id.index;
				let r_to = id.index.max(to) - id.index;
				let left_child = id.children.0.remove_range(l_from, l_to);
				let right_child = id.children.1.remove_range(r_from, r_to);

				if left_child.size() == 0 {
					return right_child;
				}

				if right_child.size() == 0 {
					return left_child;
				}

				Node::Internal(InternalData {
					index: left_child.size(),
					size: left_child.size() + right_child.size(),
					children: Box::new((left_child, right_child)),
				})
			},
			Node::Virtual(vd) => {
				assert!(from <= to && to <= vd.size);

				let new_index = from;
				let new_size = new_index + vd.size - to;

				let left_node = Node::Virtual(VirtualData {
					size: from,
					backing_offset: vd.backing_offset,
				});
				let right_node = Node::Virtual(VirtualData {
					size: vd.size - to,
					backing_offset: vd.backing_offset + to,
				});

				if left_node.size() == 0 {
					return right_node;
				}

				if right_node.size() == 0 {
					return left_node;
				}

				Node::Internal(InternalData {
					index: new_index,
					size: new_size,
					children: Box::new((left_node, right_node)),
				})
			},
		}
	}

	fn flatten(self) -> Node {
		match self {
			leaf @ Node::Leaf(_) => leaf,
			Node::Internal(id) => {
				let mut new_data = Vec::new();
				let left = id.children.0.flatten();
				if let Node::Leaf(mut ld_l) = left {
					new_data.append(&mut ld_l.data);
				}
				let right = id.children.1.flatten();
				if let Node::Leaf(mut ld_r) = right {
					new_data.append(&mut ld_r.data);
				}
				Node::Leaf(LeafData {
					data: new_data
				})
			},
		}
	}

	fn merge(self, other: Node) -> Node {
		Node::Internal(InternalData {
			index: self.size(),
			size: self.size() + other.size(),
			children: Box::new((self, other)),
		})
	}
}

impl Rope {
	pub fn new() -> Rope {
		Rope {
			root: Node::Leaf(LeafData {
				data: Vec::new(),
			}),
		}
	}

	pub fn size(&self) -> usize {
		self.root.size()
	}

	pub fn insert_at(self, input: Vec<u8>, index: usize) -> Rope {
		Rope {
			root: self.root.insert_at(input, index),
		}
	}

	pub fn remove_range(self, from: usize, to: usize) -> Rope {
		Rope {
			root: self.root.remove_range(from, to),
		}
	}

	pub fn flatten(self) -> Rope {
		Rope {
			root: self.root.flatten(),
		}
	}

	pub fn merge(self, other: Rope) -> Rope {
		Rope {
			root: self.root.merge(other.root),
		}
	}

	pub fn print(&self) {
		println!("{:#?}", self.root);

		let i = NodeIter::new(&self.root);
		let s: Vec<u8> = i.copied().collect();
		let s = s.as_slice();
		println!("{:?}", std::str::from_utf8(s).unwrap());
	}
}
