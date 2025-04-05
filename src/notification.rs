use std::time::SystemTime;

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
        Notification {
            message: "".to_string(),
            level: NotificationLevel::None,
            expires: SystemTime::now(),
        }
    }

    pub fn is_visible(&self) -> bool {
        SystemTime::now() < self.expires
    }
}
