use std::{process::Stdio, str::from_utf8};
use tokio::io::AsyncReadExt;

use tokio::select;
use tokio::sync::mpsc::Receiver;
use tokio::{io::BufReader, process::Command, sync::mpsc::Sender, task};

use crate::event::input::AppEvent;

#[derive(Debug)]
pub enum ProcessEvent {
    Start(Vec<String>),
    Stop,
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
            };

            let program = match args.first() {
                Some(arg) => arg,
                None => continue,
            };

            // start the PHP process - detatching stdin for now but capturing stdout/stderr
            let mut process = Command::new(program)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .args(&args[1..])
                .spawn()
                .unwrap();

            let mut stdoutreader = BufReader::new(process.stdout.take().unwrap());
            let mut stderrreader = BufReader::new(process.stderr.take().unwrap());
            let sender = parent_sender.clone();

            let io_task = task::spawn(async move {
                loop {
                    let mut stdout_buff = [0; 255];
                    let mut stderr_buff = [0; 255];

                    select! {
                        read = stdoutreader.read(&mut stdout_buff) => {
                            if let Ok(s) = from_utf8(&stdout_buff[..read.unwrap()]) {
                                if !s.is_empty() {
                                    sender
                                        .send(AppEvent::ChannelLog("stdout".to_string(), s.to_string()))
                                        .await
                                        .unwrap_or_default();
                                }
                            };
                        },
                        read = stderrreader.read(&mut stderr_buff) => {
                            if let Ok(s) = from_utf8(&stderr_buff[..read.unwrap()]) {
                                if !s.is_empty() {
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

            let sender = parent_sender.clone();
            loop {
                select! {
                    exit_code = process.wait() => {
                        if let Ok(exit_code) = exit_code {
                            if exit_code.code().unwrap_or_default() != 0 {
                                let _ = sender.send(
                                    AppEvent::NotifyError(
                                        format!(
                                            "Process '{:?}' exited with code {}",
                                            args,
                                            exit_code.code().unwrap_or_default()
                                        )
                                    )
                                ).await;
                            }
                        }
                        break;
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
                                io_task.abort();
                                break;
                            },
                        };
                    },
                };
            }
        }
    });
}
