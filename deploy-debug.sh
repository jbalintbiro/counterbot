. secrets.sh
cargo build && scp target/debug/counterbot "$DEPLOY_HOST:counterbot.debug.tmp"
