use std::error::Error;
use std::mem::replace;
use std::sync::{Arc, RwLock};

type RopeResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Rope {
	root: Arc<RwLock<Node>>,
}

#[derive(Debug)]
enum Node {
	Leaf(LeafData),
	Internal(InternalData),
}

struct LeafData {
	data: Vec<u8>,
}

// Make it more friendly to print leaves as debug - turn it to readable characters
impl std::fmt::Debug for LeafData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", std::str::from_utf8(&self.data).unwrap())
	}
}

#[derive(Debug)]
struct InternalData {
	index: usize,
	size: usize,
	children: Box<(Node, Node)>,
}

// This will yield a leaf Vec for each call of next()
// Maintains an internal stack for a depth first search
struct LeafIter<'a> {
	stack: Vec<&'a Node>,
}

impl<'a> Iterator for LeafIter<'a> {
	type Item = &'a Node;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.stack.pop() {
				Some(Node::Internal(inner)) => {
					self.stack.push(&inner.children.1);
					self.stack.push(&inner.children.0);
				}
				Some(leaf) => break Some(leaf),
				None => break None,
			}
		}
	}
}

impl Default for Rope {
	fn default() -> Self { Self::new() }
}

impl Node {
	fn size(&self) -> usize {
		match self {
			Node::Leaf(inner) => inner.data.len(),
			Node::Internal(inner) => inner.size,
		}
	}

	fn insert_at(&mut self, index: usize, input: &[u8]) {
		match self {
			Node::Leaf(inner) => {
				// Move Vec out of the node
				let mut left_node_data = replace(&mut inner.data, Vec::new());

				// Split into 2 - clone is performed here
				let right_node_data = left_node_data.split_off(index);

				// Clone our slice to the end of the left node data
				left_node_data.extend_from_slice(&input);

				// Create the new node structures and move our new Vecs inside
				let left_node = Node::Leaf(LeafData {
					data: left_node_data,
				});

				let right_node = Node::Leaf(LeafData {
					data: right_node_data,
				});

				// If a node is empty, use only the other one
				if left_node.size() == 0 {
					replace(self, right_node);
				}
				else if right_node.size() == 0 {
					replace(self, left_node);
				}
				// If both nodes have data use an Internal parent node
				else {
					replace(
						self,
						Node::Internal(InternalData {
							index: left_node.size(),
							size: left_node.size() + right_node.size(),
							children: Box::new((left_node, right_node)),
						}),
					);
				}
			}
			// Recurse deeper
			Node::Internal(inner) => {
				if index <= inner.index {
					inner.children.0.insert_at(index, input);
				}
				else {
					inner.children.1.insert_at(index - inner.index, input);
				}
				// Update node sizes
				inner.index = inner.children.0.size();
				inner.size = inner.children.0.size() + inner.children.1.size();
			}
		}
	}

	fn remove_range(&mut self, from: usize, to: usize) {
		match self {
			Node::Leaf(inner) => {
				// Move Vec out of the node
				let mut left_node_data = replace(&mut inner.data, Vec::new());
				// Split into 2 - clone is performed here
				let right_node_data = left_node_data.split_off(to);

				// Truncate left node data
				left_node_data.truncate(from);

				// Create new node structures and move our new Vecs inside
				let left_node = Node::Leaf(LeafData {
					data: left_node_data,
				});

				let right_node = Node::Leaf(LeafData {
					data: right_node_data,
				});

				// If a node is empty, use only the other one
				if left_node.size() == 0 {
					replace(self, right_node);
				}
				else if right_node.size() == 0 {
					replace(self, left_node);
				}
				// If both nodes have data use an Internal parent node
				else {
					replace(
						self,
						Node::Internal(InternalData {
							index: left_node.size(),
							size: left_node.size() + right_node.size(),
							children: Box::new((left_node, right_node)),
						}),
					);
				}
			}
			Node::Internal(inner) => {
				// Calculate parameters for children
				let l_from = inner.index.min(from);
				let l_to = inner.index.min(to);
				let r_from = inner.index.max(from) - inner.index;
				let r_to = inner.index.max(to) - inner.index;

				let left_node = &mut inner.children.0;
				let right_node = &mut inner.children.1;

				// Recurse deeper
				left_node.remove_range(l_from, l_to);
				right_node.remove_range(r_from, r_to);

				// Check for empty children and replace self with nonempty child
				if left_node.size() == 0 {
					match right_node {
						Node::Leaf(child_inner) => {
							let saved_data = replace(&mut child_inner.data, Vec::new());
							replace(self, Node::Leaf(LeafData { data: saved_data }));
						}
						Node::Internal(child_inner) => {
							let saved_box = replace(
								&mut child_inner.children,
								Box::new((
									Node::Leaf(LeafData { data: Vec::new() }),
									Node::Leaf(LeafData { data: Vec::new() }),
								)),
							);
							replace(
								self,
								Node::Internal(InternalData {
									index: saved_box.0.size(),
									size: saved_box.0.size() + saved_box.1.size(),
									children: saved_box,
								}),
							);
						}
					}
				}
				else if right_node.size() == 0 {
					match left_node {
						Node::Leaf(child_inner) => {
							let saved_data = replace(&mut child_inner.data, Vec::new());
							replace(self, Node::Leaf(LeafData { data: saved_data }));
						}
						Node::Internal(child_inner) => {
							let saved_box = replace(
								&mut child_inner.children,
								Box::new((
									Node::Leaf(LeafData { data: Vec::new() }),
									Node::Leaf(LeafData { data: Vec::new() }),
								)),
							);
							replace(
								self,
								Node::Internal(InternalData {
									index: saved_box.0.size(),
									size: saved_box.0.size() + saved_box.1.size(),
									children: saved_box,
								}),
							);
						}
					}
				}
				// Otherwise update sizes
				else {
					inner.index = inner.children.0.size();
					inner.size = inner.children.0.size() + inner.children.1.size();
				}
			}
		}
	}

	fn flatten(&mut self) {
		if let Node::Internal(inner) = self {
			// Recurse deeper
			inner.children.0.flatten();
			inner.children.1.flatten();

			// Replace self with leaf node containing both child leaf nodes concatenated
			match (&mut inner.children.0, &mut inner.children.1) {
				(Node::Leaf(left), Node::Leaf(right)) => {
					let mut saved_data_left = replace(&mut left.data, Vec::new());
					let mut saved_data_right = replace(&mut right.data, Vec::new());
					saved_data_left.append(&mut saved_data_right);
					replace(
						self,
						Node::Leaf(LeafData {
							data: saved_data_left,
						}),
					);
				}
				_ => panic!("Flatten Failed"),
			}
		}
	}

	fn iterate_leaves(&self) -> LeafIter { LeafIter { stack: vec![self] } }
}

impl Rope {
	pub fn new() -> Rope {
		Rope {
			root: Arc::new(RwLock::new(Node::Leaf(LeafData { data: Vec::new() }))),
		}
	}

	pub fn insert_at(&self, index: usize, input: &[u8]) -> RopeResult<()> {
		self.root
			.write()
			.map_err(|e| e.to_string())?
			.insert_at(index, input);
		Ok(())
	}

	pub fn remove(&self, from: usize, size: usize) -> RopeResult<()> {
		self.root
			.write()
			.map_err(|e| e.to_string())?
			.remove_range(from, from + size);
		Ok(())
	}

	pub fn len(&self) -> RopeResult<usize> {
		Ok(self.root.read().map_err(|e| e.to_string())?.size())
	}

	pub fn is_empty(&self) -> RopeResult<bool> { Ok(self.len()? == 0) }

	pub fn flatten(&self) -> RopeResult<()> {
		self.root.write().map_err(|e| e.to_string())?.flatten();
		Ok(())
	}

	pub fn collect(&self, from: usize, to: usize) -> RopeResult<Vec<u8>> {
		let mut collection = Vec::new();
		let mut counter = 0usize;

		for node in self
			.root
			.read()
			.map_err(|e| e.to_string())?
			.iterate_leaves()
		{
			if let Node::Leaf(inner) = node {
				let len = inner.data.len();
				let array_start = counter;
				let array_end = counter + len;

				if to <= array_start || array_end <= from {
					// Requested bytes not in current array
					counter += len;
					continue;
				}

				// Requested bytes are in current array

				// Set bounds to slice current array
				let slice_from = if array_start < from {
					from - array_start
				}
				else {
					0
				};
				let slice_to = if to < array_end {
					to - array_start
				}
				else {
					len
				};

				// Append slice to collected bytes
				collection.extend_from_slice(&inner.data[slice_from..slice_to]);

				counter += len;
			}
		}
		Ok(collection)
	}

	pub fn search(&self, needle: u8) -> RopeResult<Vec<usize>> {
		let mut matches = Vec::new();
		let mut counter = 0usize;
		for node in self
			.root
			.read()
			.map_err(|e| e.to_string())?
			.iterate_leaves()
		{
			if let Node::Leaf(inner) = node {
				for byte in inner.data.iter() {
					if *byte == needle {
						matches.push(counter);
					}
					counter += 1;
				}
			}
		}
		Ok(matches)
	}
}
