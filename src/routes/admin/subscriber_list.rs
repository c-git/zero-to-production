use crate::domain::{Subscriber, SubscriberEmail, SubscriberName};
use crate::utils::{e500, wrap_in_quotes};
use actix_web::web;
use anyhow::Context;
use sqlx::PgPool;

#[tracing::instrument(name = "Get subscriber list", skip(pool))]
pub async fn subscriber_list(
    pool: web::Data<PgPool>,
) -> Result<web::Json<Vec<Subscriber>>, actix_web::Error> {
    let data = sqlx::query!("SELECT email, name, status FROM subscriptions ORDER BY name",)
        .fetch_all(&**pool)
        .await
        .context("Failed to perform a query to retrieve subscriber list")
        .map_err(e500)?;
    let mut subscribers = vec![];
    for record in data {
        subscribers.push(Subscriber {
            email: SubscriberEmail::parse(record.email).map_err(e500)?,
            name: SubscriberName::parse(record.name).map_err(e500)?,
            status: serde_json::from_str(&wrap_in_quotes(&record.status)).map_err(e500)?,
        })
    }
    Ok(web::Json(subscribers))
}
