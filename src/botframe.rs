use std::io::prelude::*;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::net::TcpStream;

pub fn run_bot<F>(nick: String, server: String, chan: String, mut handler: F) -> crate::Result<()>
where
    F: FnMut(&mut FmtWrite, &str, &str) -> crate::Result<()> + 'static,
{
    let mut in_buf: [u8; 512] = [0; 512];
    let mut out_buf = String::new();
    let mut conn = TcpStream::connect(&server)?;

    write!(out_buf, "NICK {}\r\n", nick)?;
    write!(out_buf, "USER {} 0 * :CounterBot\r\n", nick)?;
    write!(out_buf, "JOIN {}\r\n", chan)?;
    
    conn.write(out_buf.as_bytes())?;
    conn.flush()?;
    out_buf.clear();

    let mut end = 0;
    loop {
        let count = conn.read(&mut in_buf[end..])?;
        end += count;

        if end > 0 {
            match in_buf.windows(2).position(|w| w == &[13, 10]) {
                Some(line_end) => {
                    match std::str::from_utf8(&in_buf[..line_end]) {
                        Ok(line) => println!("IRCCMD {}", line),
                        Err(e) => eprintln!("not utf8! {}", e),
                    }

                    let real_line_end = line_end + 2;
                    let rest = end - real_line_end;
                    for i in 0..std::cmp::min(rest,real_line_end) {
                        in_buf[i] = in_buf[i+real_line_end];
                    }
                    end -= real_line_end;
                }
                None => assert!(end != 512),
            }

        }
    }
}

