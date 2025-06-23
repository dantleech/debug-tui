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

    pub fn is_visible(&self) -> bool {
        SystemTime::now() < self.expires
    }

    #[allow(dead_code)]
    pub(crate) fn info(message: String) -> Notification {
        Notification {
            message,
            level: NotificationLevel::Info,
            expires: SystemTime::now()
                .checked_add(Duration::from_secs(5))
                .unwrap(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn warning(message: String) -> Notification {
        Notification {
            message,
            level: NotificationLevel::Warning,
            expires: SystemTime::now()
                .checked_add(Duration::from_secs(5))
                .unwrap(),
        }
    }
}
