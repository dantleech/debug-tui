use std::{process::Stdio, str::from_utf8};
use tokio::io::AsyncReadExt;

use tokio::process::Child;
use tokio::{io::BufReader, process::Command, sync::mpsc::Sender, task};

use crate::{channel::Channels, event::input::AppEvent};

pub fn start(
    channels: &mut Channels,
    script: &Vec<String>,
    parent_sender: Sender<AppEvent>,
) -> Option<Child> {
    let cmd = script.first();
    if let Some(program) = cmd {
        let mut process = Command::new(&program)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .args(&script[1..])
            .spawn()
            .unwrap();

        let buffer = channels.get_mut("php").buffer.clone();
        let sender = parent_sender.clone();

        let mut stdoutreader = BufReader::new(process.stdout.take().unwrap());
        let mut stderrreader = BufReader::new(process.stderr.take().unwrap());

        task::spawn(async move {
            loop {
                let mut buf = [0; 255];
                let read = stdoutreader
                    .read(&mut buf)
                    .await
                    .expect("TODO: handle this error");

                handle_read(buffer.clone(), sender.clone(), read, buf).await;
            }
        });

        let buffer = channels.get_mut("php").buffer.clone();
        let sender = parent_sender.clone();
        task::spawn(async move {
            loop {
                let mut buf = [0; 255];
                let read = stderrreader
                    .read(&mut buf)
                    .await
                    .expect("TODO: handle this error");
                handle_read(buffer.clone(), sender.clone(), read, buf).await;
            }
        });

        return Some(process);
    }
    return None;
}

async fn handle_read(
    buffer: std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
    sender: Sender<AppEvent>,
    read: usize,
    buf: [u8; 255]
) {
    if read == 0 {
        return;
    }
    buffer.lock().await.push(
        from_utf8(&buf[..read])
            .expect("TODO: handle this error")
            .to_string(),
    );
    sender
        .send(AppEvent::FocusChannel("php".to_string()))
        .await
        .unwrap();
}
