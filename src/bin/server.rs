use async_echo::{echo_server, EcResult};

use async_std::io::{stdin, BufRead, BufReader};
use async_std::stream::Stream;
use async_std::task;

async fn wait_exitus() -> EcResult<()> {
    let mut lines = BufReader::new(stdin()).lines();

    while let Some(line) = lines.next().await {
        if line? == "exit" {
            break;
        }
    }

    Ok(())
}

fn main() {
    task::spawn(async {
        if let Err(e) = echo_server("127.0.0.1:9102").await {
            eprintln!("{}", e);
        }
    });
    let _ = task::block_on(wait_exitus());
}
