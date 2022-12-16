use crate::domain::subscriber_status::SubscriberStatus;
use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;
use serde::{Deserialize, Serialize};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct Subscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
    pub status: SubscriberStatus,
}
