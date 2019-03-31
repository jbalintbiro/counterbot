use std::io::prelude::*;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::net::TcpStream;

#[derive(Debug)]
pub struct Message<'a> {
    prefix: Option<Prefix<'a>>,
    command: &'a str,
    params: &'a [&'a str],
}

#[derive(Debug)]
pub enum Prefix<'a> {
    Server(&'a str),
    User{nick: &'a str, user: &'a str, host: &'a str},
}

pub fn run_bot<F>(nick: String, server: String, chan: String, mut handler: F) -> crate::Result<()>
where
    F: FnMut(&mut FmtWrite, &str, &str) -> crate::Result<()> + 'static,
{
    let mut in_buf: [u8; 13000] = [0; 13000]; // incoming messages buffer
    let mut ib_end = 0;

    let mut reply_buf = String::new(); // reply buffer
    let mut out_buf = String::new(); // outgoing messages buffer

    let mut conn = TcpStream::connect(&server)?;
    conn.set_nodelay(true)?;

    write!(out_buf, "NICK {}\r\n", nick)?;
    write!(out_buf, "USER {} 0 * :CounterBot\r\n", nick)?;
    write!(out_buf, "JOIN {}\r\n", chan)?;
    

    loop {
        if !out_buf.is_empty() {
            conn.write(out_buf.as_bytes())?;
            conn.flush()?;
            out_buf.clear();
        }

        let count = conn.read(&mut in_buf[ib_end..])?;
        ib_end += count;
        

    }
}

