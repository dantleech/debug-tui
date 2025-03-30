use std::time::{self, Duration, SystemTime};


pub enum NotificationLevel {
    Error,
    None,
}
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
    expires: SystemTime,
}
impl Notification {
    pub(crate) fn none() -> Notification {
        Notification{
            message: "".to_string(),
            level: NotificationLevel::None,
            expires: SystemTime::now(),
        }
    }

    pub(crate) fn error(message: String) -> Self {
        Notification{
            message,
            level: NotificationLevel::Error,
            expires: SystemTime::now().checked_add(Duration::from_secs(5)).unwrap(),
        }
    }

    pub fn is_visible(&self) -> bool {
        SystemTime::now() < self.expires
    }
}

