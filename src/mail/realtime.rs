use serde_json::{json, Value};
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MailRealtimeEvent {
    pub user_id: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub struct MailRealtimeHub {
    sender: broadcast::Sender<MailRealtimeEvent>,
}

impl Default for MailRealtimeHub {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }
}

impl MailRealtimeHub {
    pub fn subscribe(&self) -> broadcast::Receiver<MailRealtimeEvent> {
        self.sender.subscribe()
    }

    pub fn publish_account_updated(
        &self,
        user_id: &str,
        account_id: Uuid,
        synced_messages: usize,
        reason: &str,
    ) {
        self.publish(
            Some(user_id.to_owned()),
            json!({
                "type": "mail.updated",
                "accountId": account_id,
                "syncedMessages": synced_messages,
                "reason": reason,
            }),
        );
    }

    pub fn publish_global_updated(&self, reason: &str) {
        self.publish(
            None,
            json!({
                "type": "mail.updated",
                "reason": reason,
            }),
        );
    }

    fn publish(&self, user_id: Option<String>, payload: Value) {
        let _ = self.sender.send(MailRealtimeEvent { user_id, payload });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn account_update_contains_mail_event_metadata() {
        let hub = MailRealtimeHub::default();
        let mut receiver = hub.subscribe();
        let account_id = Uuid::new_v4();

        hub.publish_account_updated("user-a", account_id, 3, "idle");

        let event = receiver.recv().await.unwrap();
        assert_eq!(event.user_id.as_deref(), Some("user-a"));
        assert_eq!(event.payload["type"], "mail.updated");
        assert_eq!(event.payload["accountId"], account_id.to_string());
        assert_eq!(event.payload["syncedMessages"], 3);
        assert_eq!(event.payload["reason"], "idle");
    }
}
