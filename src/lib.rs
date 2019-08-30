#![forbid(unsafe_code)]

use std::net::ToSocketAddrs;

use async_std::io::{BufRead, BufReader, Write};
use async_std::net::{Incoming, TcpListener, TcpStream};
use async_std::task;

use futures::StreamExt;

pub type EcResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

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
