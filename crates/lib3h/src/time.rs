use std::time::{SystemTime, UNIX_EPOCH};

pub fn since_epoch_ms() -> u64 {
    let since_the_epoch;
    //unsafe {
    let start = SystemTime::now();
    since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    //}
    let in_ms =
        since_the_epoch.as_secs() * 1000 + u64::from(since_the_epoch.subsec_nanos()) / 1_000_000;
    in_ms
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn test_since_epoch_ms() {
        let first = since_epoch_ms();
        println!("first: {}", first);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let second = since_epoch_ms();
        println!("second: {}", second);
        assert!(second > first);
        assert!(second >= 8);
    }
}
