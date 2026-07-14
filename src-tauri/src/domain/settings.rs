use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppSettings {
    pub timezone: String,
    pub date_format: String,
    pub time_format: u8,
    pub week_starts_on: u8,
    #[serde(default = "default_desktop_notifications")]
    pub desktop_notifications: bool,
    #[serde(default)]
    pub operator_name: String,
    pub admin_email: String,
    pub default_accounts: Vec<i64>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            timezone: "UTC".to_string(),
            date_format: "human".to_string(),
            time_format: 12,
            week_starts_on: 1,
            desktop_notifications: true,
            operator_name: String::new(),
            admin_email: String::new(),
            default_accounts: Vec::new(),
        }
    }
}

fn default_desktop_notifications() -> bool {
    true
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.timezone.trim().is_empty() {
            return Err("timezone is required".to_string());
        }

        if !matches!(self.time_format, 12 | 24) {
            return Err("time_format must be 12 or 24".to_string());
        }

        if !matches!(self.week_starts_on, 0 | 1) {
            return Err("week_starts_on must be 0 for Sunday or 1 for Monday".to_string());
        }

        if !self.admin_email.is_empty() && !self.admin_email.contains('@') {
            return Err("admin_email must be empty or a valid email address".to_string());
        }

        if self.operator_name.len() > 120 {
            return Err("operator_name must be 120 characters or fewer".to_string());
        }

        Ok(())
    }
}
