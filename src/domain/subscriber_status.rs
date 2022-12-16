use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriberStatus {
    Confirmed,
    PendingConfirmation,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use claim::assert_err;

    #[test]
    fn valid_status_is_parsed_successfully() {
        let statuses = vec!["confirmed", "pending_confirmation"];
        for status in statuses {
            serde_json::from_str::<SubscriberStatus>(&format!("\"{status}\""))
                .context(format!("Failed to convert '{status}'"))
                .unwrap();
        }
    }

    #[test]
    fn invalid_status_parse_fails() {
        assert_err!(serde_json::from_str::<SubscriberStatus>(
            "\"invalid_status\""
        ));
    }
}
