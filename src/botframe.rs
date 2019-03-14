use std::io::prelude::*;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::net::TcpStream;

pub fn run_bot<F>(nick: String, server: String, chan: String, mut handler: F) -> crate::Result<()>
where
    F: FnMut(&mut FmtWrite, &str, &str) -> crate::Result<()> + 'static,
{
    let mut in_buf: [u8; 4096] = [0; 4096];
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
            let mut start = 0;

        }
        eprintln!("{}", end);
    }
}

