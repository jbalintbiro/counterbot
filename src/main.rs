#[macro_use]
extern crate serde_derive;
use irc::client::prelude::*;

type Counter = std::collections::HashMap<String, u64>;

#[derive(Debug, Deserialize)]
struct Settings {
    nick: String,
    server: String,
    channel: String,
    dbfile: String,
    count_words: Vec<String>,
    count_color: Option<u8>,
    count_unit: String,
    replacements: std::collections::HashMap<String, String>,
}

fn absorb_message<C: Client>(client: &C, state: &mut Counter, settings: &Settings, chan: &str, nick: &str, text: &str) {
    let realnick: &str = settings.replacements.get(nick).map(|s| &**s).unwrap_or(&nick);
    for cw in settings.count_words.iter() {
        if text.starts_with(cw) {
            *state.entry(realnick.to_owned()).or_insert(0) += 1;
            let toml = toml::to_string(&toml::value::Value::try_from(&state).expect("db TOML structure error")).expect("db TOML encoding error");
            std::fs::write(&settings.dbfile, toml.as_bytes()).expect("db write error")
        }
    }
    let mut buf = String::new();
    match text {
        "`top" => write_top(&mut buf, state, settings),
        "`stat" => write_stat(&mut buf, state, settings, realnick),
        _ => {}
    }
    if !buf.is_empty() { client.send_privmsg(chan, buf).expect("can't send") }
}

fn write_stat<W: std::fmt::Write>(buf: &mut W, state: &Counter, settings: &Settings, nick: &str) {
    write!(buf, "{}: ", nick).expect("can't write buffer");
    write_count(buf, settings, *state.get(nick).unwrap_or(&0));
}

fn write_top<W: std::fmt::Write>(buf: &mut W, state: &Counter, settings: &Settings) {
    let medals = [(0, '\u{1F41B}'), (7, '\u{1F947}'), (15, '\u{1F948}'), (8, '\u{1F949}')];
    let mut v: Vec<_> = state.iter().map(|(k, v)| (k.clone(), *v)).collect();
    v.sort_unstable_by(|&(_, ca), &(_, cb)| ca.cmp(&cb).reverse());
    let mut place = 1;
    let diffs = Some(false).into_iter().chain(v.iter().zip(v.iter().next()).map(|(a, b)| a.1 != b.1)).chain(Some(true).into_iter());
    for ((nick, count), same) in v.iter().zip(diffs).take(3) {
        if !same { place += 1 };
        write!(buf, "\u{3}{}{}\u{3} {} ", medals[place].0, medals[place].1, nick.as_str()).expect("oom");
        write_count(buf, settings, *count);
    }
    write!(buf, " :: \u{2211}").unwrap();
    write_count(buf, settings, v.iter().map(|x| x.1).sum());
}

fn write_count<W: std::fmt::Write>(buf: &mut W, settings: &Settings, count: u64) {
    if let Some(color) = settings.count_color { write!(buf, "\u{3}{:02}", color).unwrap(); }
    write!(buf, "{}{}", count, settings.count_unit).unwrap();
    if let Some(_) = settings.count_color { write!(buf, "\u{3}").unwrap(); }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 { panic!("USAGE: {} <config>", args[0]); }
    let config = std::fs::read_to_string(args[1].to_owned()).expect("config file read error");
    let settings: Settings = toml::from_str(&config).expect("config parse error");
    let irc_config = Config {
        nickname: Some(settings.nick.clone()),
        server: Some(settings.server.clone()),
        channels: Some(vec![settings.channel.clone()]),
        ..Config::default()
    };
    let counts = std::fs::read_to_string(&settings.dbfile).expect("db file read error");
    let mut state = toml::from_str(&counts).expect("db TOML parse error");
    let mut reactor = IrcReactor::new().unwrap();
    let client = reactor.prepare_client_and_connect(&irc_config).unwrap();
    client.identify().unwrap();
    reactor.register_client_with_handler(client, move |client, msg| {
        Ok(if let Message { prefix, command: Command::PRIVMSG(target, text), .. } = msg {
            if target != settings.channel { return Ok(()) }
            if let Some(nick) = prefix.and_then(|u| u.split('!').next().map(|s| s.to_owned())) {
                absorb_message(client, &mut state, &settings, &target, &nick, &text);
            }
        })
    });
    reactor.run().unwrap();
}
