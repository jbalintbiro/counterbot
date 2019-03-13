. secrets.sh
cargo build --release && scp target/release/counterbot "$DEPLOY_HOST:counterbot.stage.tmp"
