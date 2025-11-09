use std::{process::Stdio, str::from_utf8};
use tokio::io::AsyncReadExt;

use tokio::process::Child;
use tokio::select;
use tokio::sync::mpsc::Receiver;
use tokio::{io::BufReader, process::Command, sync::mpsc::Sender, task};

use crate::{channel::Channels, event::input::AppEvent};

#[derive(Debug)]
pub enum ProcessEvent {
    Start(Vec<String>),
    Stop,
    Restart,
}

pub fn process_manager_start(
    mut receiver: Receiver<ProcessEvent>,
    parent_sender: Sender<AppEvent>,
) {
    task::spawn(async move {
        loop {
            let cmd = receiver.recv().await;
            let event = match cmd {
                Some(event) => event,
                None => continue,
            };
            let args = match event {
                ProcessEvent::Start(args) => args,
                ProcessEvent::Stop => continue,
                ProcessEvent::Restart => continue,
            };

            let program = match args.first() {
                Some(arg) => arg,
                None => continue,
            };

            let mut process = Command::new(program)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .args(&args[1..])
                .spawn()
                .unwrap();

            let mut stdoutreader = BufReader::new(process.stdout.take().unwrap());
            let mut stderrreader = BufReader::new(process.stderr.take().unwrap());
            let sender = parent_sender.clone();

            task::spawn(async move {
                loop {
                    let mut stdout_buff = [0; 255];
                    let mut stderr_buff = [0; 255];

                    select! {
                        read = stdoutreader.read(&mut stdout_buff) => {
                            if let Ok(s) = from_utf8(&stdout_buff[..read.unwrap()]) {
                                if s.len() > 0 {
                                    sender
                                        .send(AppEvent::ChannelLog("stdout".to_string(), s.to_string()))
                                        .await
                                        .unwrap_or_default();
                                }
                            };
                        },
                        read = stderrreader.read(&mut stderr_buff) => {
                            if let Ok(s) = from_utf8(&stderr_buff[..read.unwrap()]) {
                                if s.len() > 0 {
                                    sender
                                        .send(AppEvent::ChannelLog("stderr".to_string(), s.to_string()))
                                        .await
                                        .unwrap_or_default();
                                }
                            };
                        },
                    };
                }
            });

            loop {
                select! {
                    _ = process.wait() => {
                        return;
                    },
                    cmd = receiver.recv() => {
                        let event = match cmd {
                            Some(event) => event,
                            None => continue,
                        };
                        match event {
                            ProcessEvent::Start(_) => continue,
                            ProcessEvent::Stop => {
                                process.kill().await.unwrap_or_default();
                                break;
                            },
                            ProcessEvent::Restart => {
                                // TODO: restart
                                process.kill().await.unwrap_or_default();
                                break;
                            },
                        };
                    },
                };
            }
        }
    });
}
