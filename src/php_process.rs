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

        let mut stdoutreader = BufReader::new(process.stdout.take().unwrap());
        let buffer = channels.get_mut("stdout").buffer.clone();
        let sender = parent_sender.clone();

        task::spawn(async move {
            loop {
                let mut buf = [0; 255];
                match stdoutreader
                    .read(&mut buf)
                    .await {
                        Ok(read) => handle_read(buffer.clone(), sender.clone(), read, "stdout".to_string(), buf).await,
                        Err(_) => (),
                    }

                ;
            }
        });

        let mut stderrreader = BufReader::new(process.stderr.take().unwrap());
        let buffer = channels.get_mut("stderr").buffer.clone();
        let sender = parent_sender.clone();
        task::spawn(async move {
            loop {
                let mut buf = [0; 255];
                match stderrreader
                    .read(&mut buf)
                    .await {
                        Ok(read) => handle_read(buffer.clone(), sender.clone(), read, "stderr".to_string(), buf).await,
                        Err(_) => (),
                    }
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
    channel: String,
    buf: [u8; 255]
) {
    if read == 0 {
        return;
    }
    match from_utf8(&buf[..read]) {
        Ok(s) => {
            buffer.lock().await.push(s.to_string());
            sender
                .send(AppEvent::FocusChannel(channel))
                .await
                .unwrap_or_default();
        },
        Err(_) => (),
    };
}
