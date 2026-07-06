use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

fn main() {
    println!("=== PTY basic read/write test ===");
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(std::env::current_dir().unwrap());

    let mut child = pair.slave.spawn_command(cmd).expect("spawn");

    let mut writer = pair.master.take_writer().expect("writer");
    writer
        .write_all(b"printf 'marker:%s\\n' 'pty-ok'\n")
        .expect("write");
    writer.flush().expect("flush");

    let mut reader = pair.master.try_clone_reader().expect("reader");
    let mut output = String::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut buf = [0u8; 1024];
    while Instant::now() < deadline && !output.contains("marker:pty-ok") {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) => {
                println!("read error: {}", e);
                break;
            }
        }
    }
    println!("output:\n{}", output);
    assert!(
        output.contains("marker:pty-ok"),
        "expected marker output not found"
    );
    println!("basic PTY test passed");

    println!("\n=== PTY resize test ===");
    pair.master
        .resize(PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("resize");
    writer
        .write_all(b"stty size && exit\n")
        .expect("write resize probe");
    writer.flush().expect("flush");

    let mut resize_output = String::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && !resize_output.contains("40 120") {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => resize_output.push_str(&String::from_utf8_lossy(&buf[..n])),
            Err(e) => {
                println!("read error: {}", e);
                break;
            }
        }
    }
    println!("resize output:\n{}", resize_output);
    assert!(
        resize_output.contains("40 120"),
        "expected resize dimensions not found"
    );
    println!("resize PTY test passed");

    let status = child.wait().expect("wait");
    println!("\nexit code: {:?}", status.exit_code());
    println!("All PTY tests passed on {}", std::env::consts::OS);
}
