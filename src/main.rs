#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate nom;

mod botframe;

type Counter = std::collections::HashMap<String, u64>;
type Result<T> = std::result::Result<T, Box<std::error::Error>>;

#[derive(Deserialize)]
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

fn handle_message<W: std::fmt::Write>(buf: &mut W, state: &mut Counter, settings: &Settings, nick: &str, text: &str) -> Result<()> {
    let realnick: &str = settings.replacements.get(nick).map(|s| &**s).unwrap_or(nick);
    for cw in settings.count_words.iter() {
        if text.starts_with(cw) {
            *state.entry(realnick.to_owned()).or_insert(0) += 1;
            let toml = toml::to_string(&toml::value::Value::try_from(&state)?)?;
            std::fs::write(&settings.dbfile, toml.as_bytes())?
        }
    }
    match text {
        "`top" => write_top(buf, state, settings),
        "`stat" => write_stat(buf, state, settings, realnick),
        "`rules" => Ok(write!(buf, "\u{24B6}")?),
        _ => Ok(()),
    }
}

fn write_stat<W: std::fmt::Write>(buf: &mut W, state: &Counter, settings: &Settings, nick: &str) -> Result<()> {
    write!(buf, "{}: ", nick)?;
    write_count(buf, settings, *state.get(nick).unwrap_or(&0))
}

fn write_top<W: std::fmt::Write>(buf: &mut W, state: &Counter, settings: &Settings) -> Result<()> {
    let medals = [(0, '\u{1F41B}'), (7, '\u{1F947}'), (15, '\u{1F948}'), (8, '\u{1F949}')];
    let mut v: Vec<_> = state.iter().map(|(k, v)| (k.clone(), *v)).collect();
    v.sort_unstable_by(|&(_, ca), &(_, cb)| ca.cmp(&cb).reverse());
    let mut place = 1;
    let mut add = 1;
    let diffs = v.iter().zip(v.iter().skip(1)).map(|(a, b)| a.1 != b.1);
    for ((nick, count), diff) in v.iter().zip(diffs).take(3) {
        write!(buf, "\u{3}{}{}\u{3}{} ", medals[place].0, medals[place].1, nick.as_str())?;
        write_count(buf, settings, *count)?;
        if diff {
            place += add;
        } else {
            add += 1;
        };
    }
    write!(buf, ":: \u{2211}")?;
    write_count(buf, settings, v.iter().map(|x| x.1).sum())
}

fn write_count<W: std::fmt::Write>(buf: &mut W, settings: &Settings, count: u64) -> Result<()> {
    let color = settings.count_color.unwrap_or(0);
    Ok(write!(buf, "\u{3}{:02}{}{}\u{3}", color, count, settings.count_unit)?)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("USAGE: {} <config>", args[0]);
        std::process::exit(-1)
    }
    let config = std::fs::read_to_string(args[1].to_owned()).expect("config file read error");
    let settings: Settings = toml::from_str(&config).expect("config parse error");
    let counts = std::fs::read_to_string(&settings.dbfile).expect("db file read error");
    let mut state = toml::from_str(&counts).expect("db TOML parse error");

    botframe::run_bot(
        settings.nick.clone(),
        settings.server.clone(),
        settings.channel.clone(),
        move |mut buf, nick, text| handle_message(&mut buf, &mut state, &settings, &nick, &text),
    ).unwrap();
}
