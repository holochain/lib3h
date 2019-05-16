#[derive(Debug)]
pub enum SodiumError {
    Generic(String),
    OutputLength(String),
    OutOfMemory,
}

impl SodiumError {
    pub fn new(msg: &str) -> SodiumError {
        SodiumError::Generic(msg.to_string())
    }
}
