# [Rust异步] TCP回声服务

async/await 将在 11 月的 Rust 1.39 稳定，是时候上车了！

本文将介绍如何用 async-std 编写一个 TCP 回声服务，以熟悉 Rust 中的异步编程模式。

## 项目初始化

新建一个 lib 项目，使用 nightly 编译。您需要安装 Rust 1.39 nightly 版本。

```bash
cargo new --lib async-echo
cd async-echo
echo nightly > rust-toolchain
```

在 Cargo.toml 中添加依赖。

```toml
[dependencies]
futures-preview = { version = "0.3.0-alpha.18", features = [ "async-await", "nightly" ] }
async-std = "0.99"
```

## 编写服务

完善的错误处理不是本文的重点，我们把所有错误直接向上抛。在异步编程中，错误有可能跨越线程，因此需要 `Send + Sync + 'static`.

```rust
pub type EcResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
```

协程 echo_server 接收套接字地址，监听对应端口。

Incoming 是一个异步流类型，迭代它可以取得连接到端口的 TCP 数据流。我们为每个数据流启动一个协程 echo. Incoming 是无限流，这代表 echo_server 永不终止，当关闭服务时，我们需要从外部取消协程。

```rust
use std::net::ToSocketAddrs;

use async_std::net::{Incoming, TcpListener, TcpStream};
use async_std::task;

use futures::StreamExt;

pub async fn echo_server(addr: impl ToSocketAddrs) -> EcResult<()> {
    let listener: TcpListener = TcpListener::bind(addr).await?;
    println!("listening: {}", listener.local_addr()?);
    let mut incoming: Incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream: TcpStream = stream?;
        task::spawn(echo(stream));
    }

    Ok(())
}

async fn echo(_stream: TcpStream) -> EcResult<()> {
    unimplemented!()
}

```


## 回声

TCP 数据流可读可写，从数据流同时产生 reader 和 writer 是安全的，因为 Safe API 允许我们这么做，可以放心。

还可以在 lib.rs 顶部加上 `#![forbid(unsafe_code)]`，保证不调用 unsafe 代码。

以行为单位，每读一行，就向数据流中原样写回一行。当数据流不再产生数据时，说明对方关闭了连接，我们也关闭协程。

```rust
use async_std::io::{BufRead, BufReader, Write};

async fn echo(stream: TcpStream) -> EcResult<()> {
    let addr = stream.peer_addr()?;
    println!("[{} online]", addr);

    let (reader, mut writer) = (&stream, &stream);
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    while let Some(line) = lines.next().await {
        let mut line = line?;
        println!("[{}]: {}", addr, line);
        line.push('\n');
        writer.write_all(line.as_bytes()).await?;
    }

    println!("[{} offline]", addr);
    Ok(())
}
```

## 客户端

协程 echo_client 同时处理网络连接和标准输入，作为客户端。

连接到服务后，来自网络的流和来自标准输入的流可以一起处理。用 `select` 等待多个 future 中的一个就绪，立即处理结果，循环直到两个流中的一个关闭。

```rust
use async_std::io::stdin;
use futures::select;

pub async fn echo_client(addr: impl ToSocketAddrs) -> EcResult<()> {
    let stream = TcpStream::connect(addr).await?;
    println!("connecting: {}", stream.peer_addr()?);
    let (reader, mut writer) = (&stream, &stream);
    let mut server_lines = BufReader::new(reader).lines().fuse();
    let mut stdin_lines = BufReader::new(stdin()).lines().fuse();

    let handle_server_line = |line: String| println!("server: {}", line);

    let handle_stdin_line = move |mut line: String| {
        println!("input: {}", line);
        line.push('\n');
        async move { writer.write_all(line.as_bytes()).await }
    };

    loop {
        select! {
            line = server_lines.next() => match line{
                None => {
                    println!("Connection was closed by server");
                    break;
                }
                Some(line) => handle_server_line(line?),
            },
            line = stdin_lines.next() => match line{
                None => break,
                Some(line) => handle_stdin_line(line?).await?,
            },
        }
    }

    Ok(())
}

```

## 执行

创建两个可执行程序。

```bash
cd src
mkdir bin
cd bin
touch server.rs client.rs
```

在 server.rs 中，等待标准输入中输入 "exit"，在此期间运行服务。
如果出错，就直接打印出错误内容，也可以直接 unwarp，便于查看 backtrace。

```rust
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
```

client.rs 非常简单，直接执行协程 echo_client 即可。

```rust
use async_echo::echo_client;

use async_std::task;

fn main() {
    if let Err(e) = task::block_on(echo_client("127.0.0.1:9102")) {
        eprintln!("{}", e)
    }
}
```

## 测试

在不同的终端运行以下命令

```bash
cargo run --release --bin server
```

```bash
cargo run --release --bin client
```


服务端

    listening: 127.0.0.1:9102
    [127.0.0.1:51234 online]
    [127.0.0.1:51234]: hello
    [127.0.0.1:51234 offline]
    [127.0.0.1:51236 online]
    [127.0.0.1:51244 online]
    [127.0.0.1:51248 online]
    [127.0.0.1:51248]: A
    [127.0.0.1:51244]: B
    [127.0.0.1:51236]: C
    [127.0.0.1:51236 offline]
    [127.0.0.1:51248 offline]
    exit

客户端

    connecting: 127.0.0.1:9102
    B
    input: B
    server: B
    Connection was closed by server

## 仓库

[Nugine/async-echo](https://github.com/Nugine/async-echo)