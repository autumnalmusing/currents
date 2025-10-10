use anyhow::Result;
use notify_rust::Notification;
use tracing::{info, error};
use crate::config::NotificationConfig;
use crate::alerts::TriggeredAlert;

pub struct NotificationManager {
    config: NotificationConfig,
}

impl NotificationManager {
    pub fn new(config: NotificationConfig) -> Self {
        Self { config }
    }
    
    pub async fn send_alert(&self, alert: &TriggeredAlert) -> Result<()> {
        info!("Sending notification: {}", alert.rule_name);
        
        let urgency = match self.config.urgency.as_str() {
            "low" => notify_rust::Urgency::Low,
            "critical" => notify_rust::Urgency::Critical,
            _ => notify_rust::Urgency::Normal,
        };
        
        let mut notification = Notification::new();
        notification
            .summary(&format!("Weather Alert: {}", alert.rule_name))
            .body(&alert.message)
            .urgency(urgency)
            .timeout(self.config.timeout as i32);
        
        if self.config.sound {
            notification.sound_name("message");
        }
        
        match notification.show() {
            Ok(_) => {
                info!("Notification sent successfully for: {}", alert.rule_name);
            }
            Err(e) => {
                error!("Failed to send notification for {}: {}", alert.rule_name, e);
                return Err(e.into());
            }
        }
        
        Ok(())
    }
    
    pub async fn send_test_notification(&self) -> Result<()> {
        let test_alert = TriggeredAlert {
            rule_name: "Test Alert".to_string(),
            message: "This is a test notification from the currents daemon.".to_string(),
        };
        
        self.send_alert(&test_alert).await
    }
    
    pub async fn send_custom_notification(&self, title: &str, message: &str) -> Result<()> {
        let custom_alert = TriggeredAlert {
            rule_name: title.to_string(),
            message: message.to_string(),
        };
        
        self.send_alert(&custom_alert).await
    }
}
