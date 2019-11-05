use std::error::Error;

pub type EditrResult<T> = Result<T, Box<dyn Error>>;
