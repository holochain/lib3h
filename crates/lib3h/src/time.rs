
use std::time::{SystemTime, UNIX_EPOCH};

pub fn since_epoch_ms() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)
                               .expect("Time went backwards");
    let in_ms = since_the_epoch.as_secs() * 1000
        + since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    in_ms
}

#[cfg(test)]
pub mod tests {
    use super::since_epoch_ms;

    #[test]
    pub fn test_since_epoch_ms() {
        let first = since_epoch_ms();
        println!("first: {}", first);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = since_epoch_ms();
        assert!(second > first);
    }
}