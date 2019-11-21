use editr::text_server;

fn main() { text_server::start("files", "0.0.0.0:12345").unwrap(); }
