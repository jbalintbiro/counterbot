// this is what makes the bot tick, also here be dragons

use std::io::prelude::*;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::net::TcpStream;

#[derive(Debug, PartialEq)]
pub struct Message<'a> {
    prefix: Option<Prefix<'a>>,
    command: &'a str,
    params: Vec<&'a str>,
    trailing: Option<&'a str>,
}

#[derive(Debug, PartialEq)]
pub enum Prefix<'a> {
    Server(&'a str),
    User{nick: &'a str, user: &'a str, host: &'a str},
}

const INBUFLEN: usize = 8192;

pub fn run_bot<F>(nick: String, server: String, chan: String, mut handler: F) -> crate::Result<()>
where
    F: FnMut(&mut FmtWrite, &str, &str) -> crate::Result<()> + 'static,
{
    let mut in_buf: [u8; INBUFLEN] = [0; INBUFLEN]; // incoming messages buffer
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
            eprint!(">>> {}", out_buf);
            out_buf.clear();
        }

        let count = conn.read(&mut in_buf[ib_end..])?;
        if count == 0 { panic!("connection broke") }; // XXX
        ib_end += count;
        let mut ib_start = 0;
        
        loop {
            let (len, msg) = parse(&in_buf[ib_start..ib_end])?;
            if len == 0 { break; }
            eprintln!("<<< {:?}", msg);
            if let Some(msg) = msg {
                if msg.command == "PING" {
                    write!(out_buf, "PONG :{}\r\n", msg.trailing.unwrap_or(""))?;
                }
                if let Some(Prefix::User{nick, ..}) = msg.prefix {
                    handler(&mut reply_buf, nick, msg.trailing.unwrap_or(""))?;
                    if !reply_buf.is_empty() {
                        write!(out_buf, "PRIVMSG {} :{}\r\n", chan, reply_buf)?;
                        reply_buf.clear();
                    }
                }
                ib_start += len;
            } else {
                unreachable!("wtf, we decoded something");
            }
        }

        ib_end -= ib_start;
        for i in 0..ib_end {
            in_buf[i] = in_buf[ib_start + i];
        }
    }
}

fn parse<'a>(buf: &'a [u8]) -> crate::Result<(usize, Option<Message<'a>>)> {
    match msg(buf) {
        Ok((rem, m)) => Ok((buf.len() - rem.len(), Some(m))),
        Err(nom::Err::Incomplete(_)) => Ok((0, None)),
        Err(e) => {
            eprintln!("{:?}", e);
            let bstr = std::str::from_utf8(buf).expect("well this buffer wasn't utf-8");
            panic!("irc protocol decode error\nbuffer: {}", bstr);
        },
    }
}

named!(userpart, is_not!(" !@\r\n"));
named!(word, is_not!(" \r\n"));
named!(param, recognize!(pair!(is_not!(" \r\n:"), opt!(word))));

named!(prefix<Prefix>,
    do_parse!(
        tag!(":") >>
        prfx: alt!(user|server) >>
        (prfx)
    )
);

named!(user<Prefix>,
    do_parse!(
        nick: userpart >>
        tag!("!") >>
        user: userpart >>
        tag!("@") >>
        host: userpart >>
        (Prefix::User{ nick: s2s(nick), user: s2s(user), host: s2s(host)})
    )
);

named!(server<Prefix>,
    do_parse!(
        s: word >>
        (Prefix::Server(s2s(s)))
    )
);

named!(msg<Message>,
    do_parse!(
        prefix: opt!(prefix) >>
        opt!(tag!(" ")) >>
        command: word >>
        params:  many_m_n!(0, 14,
            do_parse!(
                tag!(" ") >>
                param: param >>
                (s2s(param))
            )
        ) >>
        trailing: opt!(do_parse!(
            tag!(" ") >>
            opt!(tag!(":")) >>
            t: take_until_either!("\n\r") >>
            (t)
        )) >>
        alt!(tag!("\r\n") | tag!("\n")) >>
        (Message { prefix, command: s2s(command), params, trailing: trailing.map(s2s) })
    )
);

fn s2s(s: &[u8]) -> &str {
    std::str::from_utf8(s).unwrap_or("UTF8_ERR_\u{1F344}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_prefix() {
        let p = b":irc.example.net ";
        assert_eq!(prefix(p).unwrap(), (&[0x20][..], Prefix::Server("irc.example.net")));
    }
    
    #[test]
    fn test_user_prefix() {
        let p = b":NICK!~USER@HOST.TLD ";
        assert_eq!(prefix(p).unwrap(), (&[0x20][..], Prefix::User{nick: "NICK", user: "~USER", host: "HOST.TLD"}));
    }

    #[test]
    fn test_welcome() {
        let m = b":irc.example.net 001 test :Welcome to the Internet Relay Network test!~test@localhost\r\n";
        assert_eq!(msg(m).unwrap(), (&[][..],  Message { prefix: Some(Prefix::Server("irc.example.net")), command: "001", params: vec!["test"], trailing: Some("Welcome to the Internet Relay Network test!~test@localhost") }));
    }

    #[test]
    fn test_kick_empty_trailing() {
        let m = b":saati!~bjb@saati.flerp KICK #sirc sIRCbot :\r\n";
        assert_eq!(msg(m).unwrap(), (&[][..],  Message { prefix: Some(Prefix::User{nick: "saati", user: "~bjb", host: "saati.flerp"}), command: "KICK", params: vec!["#sirc", "sIRCbot"], trailing: Some("")}));
    }
}
