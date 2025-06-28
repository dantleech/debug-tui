use std::time::Duration;
use std::time::SystemTime;

pub enum NotificationLevel {
    Error,
    None,
    Info,
    Warning,
}
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    expires: SystemTime,
}
impl Notification {
    const DURATION: u64 = 5;
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    pub(crate) fn none() -> Notification {
        Notification {
            message: "".to_string(),
            level: NotificationLevel::None,
            expires: SystemTime::now(),
        }
    }

    pub(crate) fn error(message: String) -> Self {
        Notification {
            message,
            level: NotificationLevel::Error,
            expires: SystemTime::now()
                .checked_add(Duration::from_secs(5))
                .unwrap(),
        }
    }

    pub fn countdown_char(&self) -> char {
        match self.expires.duration_since(SystemTime::now()) {
            Ok(duration) => {
                let pct = duration.as_secs_f64() / Self::DURATION as f64;
                let block_offset = (8.0 * pct).ceil();
                Self::BLOCKS[block_offset as usize - 1]
            }
            Err(_) => ' ',
        }
    }

    pub fn is_visible(&self) -> bool {
        SystemTime::now() < self.expires
    }

    #[allow(dead_code)]
    pub(crate) fn info(message: String) -> Notification {
        Notification {
            message,
            level: NotificationLevel::Info,
            expires: SystemTime::now()
                .checked_add(Duration::from_secs(Self::DURATION))
                .unwrap(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn warning(message: String) -> Notification {
        Notification {
            message,
            level: NotificationLevel::Warning,
            expires: SystemTime::now()
                .checked_add(Duration::from_secs(Self::DURATION))
                .unwrap(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_countdown_char() -> () {
        let notification = Notification::info("hello".to_string());
        assert_eq!('█', notification.countdown_char());

        let notification = Notification {
            message: "hello".to_string(),
            level: NotificationLevel::Info,
            expires: SystemTime::now(),
        };
        assert_eq!(' ', notification.countdown_char());

        let notification = Notification {
            message: "hello".to_string(),
            level: NotificationLevel::Info,
            expires: SystemTime::now() - Duration::from_secs(10),
        };
        assert_eq!(' ', notification.countdown_char());
    }
}
