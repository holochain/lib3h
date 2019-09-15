use std::io::Write;

pub fn try_fmt(input: String) -> String {
    let mut child = match std::process::Command::new("rustfmt")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        Err(e) => {
            eprintln!("try_fmt failed to spawn rustfmt {:?}", e);
            return input;
        }
        Ok(c) => c,
    };

    {
        let stdin = match child.stdin.as_mut() {
            None => {
                eprintln!("try_fmt failed to get stdin handle to rustfmt");
                return input;
            }
            Some(h) => h,
        };
        match stdin.write_all(input.as_bytes()) {
            Err(e) => {
                eprintln!("try_fmt failed to write to rustfmt stdin: {:?}", e);
                return input;
            }
            Ok(_) => (),
        }
    }

    let output = match child.wait_with_output() {
        Err(e) => {
            eprintln!("try_fmt failed to get output from rustfmt: {:?}", e);
            return input;
        }
        Ok(o) => o,
    };

    String::from_utf8_lossy(&output.stdout).to_string()
}
