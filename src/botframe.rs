use irc::client::prelude::*;

pub fn run_bot<F>(nick: String, server: String, chan: String, mut handler: F)
where
    F: FnMut(&mut String, &str, &str) -> crate::Result<()> + 'static,
{
    let irc_config = Config {
        nickname: Some(nick),
        server: Some(server),
        channels: Some(vec![chan.clone()]),
        ..Config::default()
    };

    let mut reactor = IrcReactor::new().unwrap();
    let client = reactor.prepare_client_and_connect(&irc_config).unwrap();
    client.identify().unwrap();
    reactor.register_client_with_handler(client, move |client, msg| {
        let mut buf = String::new();
        if let Message {
            prefix,
            command: Command::PRIVMSG(target, text),
            ..
        } = msg
        {
            if target != chan {
                return Ok(());
            }
            if let Some(nick) = prefix.and_then(|u| u.split('!').next().map(|s| s.to_owned())) {
                handler(&mut buf, &nick, &text).expect("handler blew up");
            }
        }
        if !buf.is_empty() {
            client.send_privmsg(&chan, buf)?
        }
        Ok(())
    });
    reactor.run().expect("reactor meltdown")
}
