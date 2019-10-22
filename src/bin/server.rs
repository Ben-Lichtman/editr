use std::path::Path;

use editr::text_server;

fn main() { text_server::start(Path::new("."), "127.0.0.1:12345").unwrap(); }
