use crossterm::event::{KeyCode, KeyModifiers};
use tokio::{io::AsyncReadExt, net::{TcpListener, TcpStream}, sync::mpsc::{Receiver, Sender}, task};

use crate::{dbgp::client::{DbgpClient, Message}, event::input::AppEvent, session::Session};

pub enum AppState {
    Listening,
    Connected,
}

struct Config {
    pub port: u16,
}

impl Config {
    pub fn new() -> Config {
        Config { port: 9003 }
    }
}

pub struct App {
    state: AppState,
    config: Config,
    receiver: Receiver<AppEvent>,
    sender: Sender<AppEvent>,
    session: Option<Session>,
    quit: bool,
}

impl App {
    pub fn new(receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        App {
            config: Config::new(),
            state: AppState::Listening,
            receiver,
            sender,
            session: None,
            quit: false,
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let sender = self.sender.clone();
        task::spawn(async move {
            let listener = TcpListener::bind("0.0.0.0:9003").await.unwrap();

            loop {
                match listener.accept().await {
                    Ok(s) => {
                        sender.send(AppEvent::ClientConnected(s.0)).await.unwrap();
                    },
                    Err(_) => panic!("Could not connect"),
                }
            }
        });

        loop {
            let event = self.receiver.recv().await;
            println!("Event: {:?}\n", event);

            if event.is_none() {
                continue;
            }

            let event = event.unwrap();

            match self.state {
                AppState::Listening => match event {
                    AppEvent::ClientConnected(s) => {
                        let mut session = Session::new(DbgpClient::new(s), self.sender.clone());
                        session.init().await?;
                        self.session = Some(session);
                        self.state = AppState::Connected;
                        ()
                    },
                    _ => ()
                },
                AppState::Connected => match event {
                    AppEvent::Quit => return Ok(()),
                    AppEvent::Disconnect => {
                        self.session.as_mut().expect(
                            "Session not set but it should be"
                        ).disconnect();
                        self.session = None;
                        self.state = AppState::Listening;
                    },
                    AppEvent::Input(e) => match e.code {
                        KeyCode::Char(char) => match char {
                            'r' => self.sender.send(AppEvent::Run).await?,
                            'n' => self.sender.send(AppEvent::StepInto).await?,
                            _ => (),
                        },
                        _ => (),
                    },
                    _ => self.session.as_mut().expect(
                        "Session not set but it should be"
                    ).handle(event).await?,
                },
            }
        }
    }
}
