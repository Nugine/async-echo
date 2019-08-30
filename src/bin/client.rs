use async_echo::echo_client;

use async_std::task;

fn main() {
    if let Err(e) = task::block_on(echo_client("127.0.0.1:9102")) {
        eprintln!("{}", e)
    }
}
