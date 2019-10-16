use std::io::Write;

pub fn try_fmt(input: String) -> Result<String, String> {
    let mut child = match std::process::Command::new("rustfmt")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Err(e) => {
            return Err(format!("failed to spawn rustfmt {:?}", e));
        }
        Ok(c) => c,
    };

    {
        let stdin = match child.stdin.as_mut() {
            None => {
                return Err("failed to get stdin handle to rustfmt".to_string());
            }
            Some(h) => h,
        };
        match stdin.write_all(input.as_bytes()) {
            Err(e) => {
                return Err(format!("failed to write to rustfmt stdin: {:?}", e));
            }
            Ok(_) => (),
        }
    }

    let output = match child.wait_with_output() {
        Err(e) => {
            return Err(format!("failed to get output from rustfmt: {:?}", e));
        }
        Ok(o) => o,
    };

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !stderr.is_empty() && stdout.is_empty() {
        return Err(stderr);
    }

    Ok(stdout)
}
