#[macro_use]
extern crate serde_derive;

use irc::client::prelude::*;
use smallstr::SmallString;

#[derive(Debug, Deserialize)]
struct Settings {
    nick: String,
    server: String,
    channel: String,
    dbfile: String,
    count_words: Vec<String>,
    count_color: Option<u8>,
    count_unit: String,
    replacements: std::collections::HashMap<Nick, Nick>,
}

#[derive(Debug, Deserialize, Serialize)]
struct State {
    counter: std::collections::HashMap<Nick, u64>,
}

type Nick = SmallString<[u8; 9]>;

impl State {
    fn save_state_file(&mut self, file: &str) -> std::io::Result<()> {
        let value = toml::value::Value::try_from(&self).expect("db TOML structure error");
        std::fs::write(file, toml::to_string(&value).expect("db TOML encoding error").as_bytes())
    }
}

fn absorb_message<C: Client>(client: &C, state: &mut State, settings: &Settings, chan: &str, nick: &Nick, text: &str) {
    let realnick = settings.replacements.get(nick).unwrap_or(&nick);
    for cw in settings.count_words.iter() {
        if text.starts_with(cw) {
            *state.counter.entry(realnick.clone()).or_insert(0) += 1;
            state.save_state_file(&settings.dbfile).expect("can't write db");
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

fn write_stat<W: std::fmt::Write>(buf: &mut W, state: &State, settings: &Settings, nick: &Nick) {
    let count = *state.counter.get(nick).unwrap_or(&0);
    write!(buf, "{}: ", nick.as_str()).expect("can't write buffer");
    write_count(buf, settings, count);
}

fn write_top<W: std::fmt::Write>(buf: &mut W, state: &State, settings: &Settings) {
    let mut v: Vec<_> = state.counter.iter().map(|(k, v)| (k.clone(), *v)).collect();
    v.sort_unstable_by(|&(_, ca), &(_, cb)| ca.cmp(&cb).reverse());
    let mut prev = std::u64::MAX;
    let mut place = 0;
    for (nick, count) in v.iter().take(3) {
        if *count != prev {
            place += 1
        };
        prev = *count;
        let (color, icon) = match place {
            1 => (7, '\u{1F947}'),
            2 => (15, '\u{1F948}'),
            3 => (8, '\u{1F949}'),
            _ => (6, '\u{1F41B}'),
        };
        write!(buf, "\u{3}{}{}\u{3} {} ", color, icon, nick.as_str()).expect("oom");
        write_count(buf, settings, *count);
    }
    write!(buf, " :: \u{2211}").unwrap();
    write_count(buf, settings, v.iter().map(|(_, c)| c).sum());
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
    let input = std::fs::read_to_string(&settings.dbfile).expect("db file read error");
    let mut state = toml::from_str(&input).expect("db TOML parse error");
    let mut reactor = IrcReactor::new().unwrap();
    let client = reactor.prepare_client_and_connect(&irc_config).unwrap();
    client.identify().unwrap();
    reactor.register_client_with_handler(client, move |client, msg| {
        let Message { prefix, command, .. } = msg;
        if let Command::PRIVMSG(ref target, ref text) = command {
            if target != &settings.channel {
            } else if let Some(nick) = prefix.and_then(|u| u.split('!').next().map(SmallString::from_str)) {
                absorb_message(client, &mut state, &settings, target, &nick, text);
            }
        };
        Ok(())
    });
    reactor.run().unwrap();
}
