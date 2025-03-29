use crossterm::event::{KeyCode, KeyModifiers};
use tokio::{io::AsyncReadExt, net::{TcpListener, TcpStream}, sync::mpsc::{Receiver, Sender}, task};

use crate::{dbgp::client::DbgpClient, event::input::AppEvent};

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
    client: Option<DbgpClient>,
    quit: bool,
}

impl App {
    pub fn new(receiver: Receiver<AppEvent>, sender: Sender<AppEvent>) -> App {
        App {
            config: Config::new(),
            state: AppState::Listening,
            receiver,
            sender,
            client: None,
            quit: false,
        }
    }

    pub async fn run(&mut self) -> ! {
        let sender = self.sender.clone();
        task::spawn(async move {
            let listener = TcpListener::bind("0.0.0.0:9003").await.unwrap();
            match listener.accept().await {
                Ok(s) => {
                    sender.send(AppEvent::ClientConnected(s.0)).await.unwrap();
                },
                Err(_) => panic!("Could not connect"),
            }
        });

        loop {
            let event = self.receiver.recv().await;

            if event.is_none() {
                continue;
            }

            let event = event.unwrap();

            match self.state {
                AppState::Listening => {
                    match event {
                        AppEvent::ClientConnected(s) => {
                            self.client = Some(DbgpClient::new(s));
                            self.state = AppState::Connected;

                            self.client.as_mut().unwrap().read().await;

                            continue;
                        },
                        _ => ()
                    }
                },
                AppState::Connected => match event {
                    AppEvent::Input(_) => todo!("input"),
                    AppEvent::Tick => todo!("tick"),
                    AppEvent::Quit => todo!("quit"),
                    _ => (),
                },
            }
        }
    }
}
