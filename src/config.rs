use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    pub listen: Option<String>,
    #[arg(long)]
    pub log: Option<String>,
}

pub fn load_config() -> Config {
    let args = Args::parse();
    Config {
        listen: args.listen.unwrap_or("0.0.0.0:9003".to_string()),
        log_path: args.log,
    }
}

#[derive(Clone)]
pub struct Config {
    pub listen: String,
    pub log_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new("0.0.0.0:9003".to_string())
    }
}

impl Config {
    pub fn new(listen: String) -> Config {
        Config { listen , log_path: None}
    }
}

#[cfg(test)]
mod test {
    use crate::notification::Notification;

    #[test]
    fn test_countdown_char() -> () {
        let notification = Notification::info("Hello".to_string());
        notification.countdown_char();
    }
}
