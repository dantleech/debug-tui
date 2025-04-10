use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    pub listen: Option<String>,
}

pub fn load_config() -> Config {
    let args = Args::parse();
    return Config {
        listen: args.listen.unwrap_or("0.0.0.0:9000".to_string()),
    };
}

#[derive(Clone)]
pub struct Config {
    pub listen: String,
}

impl Default for Config {
    fn default() -> Self {
        Self::new("0.0.0.0:9003".to_string())
    }
}

impl Config {
    pub fn new(listen: String) -> Config {
        Config { listen }
    }
}
