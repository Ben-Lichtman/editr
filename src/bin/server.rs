use std::path::Path;

use editr::text_server;

fn main() { text_server::start(Path::new("files"), "0.0.0.0:12345").unwrap(); }
