use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn request_returns_subscriber_list() {
    // Arrange
    let app = spawn_app().await;
    let bodies_confirmed = vec![
        "name=le%20guin&email=ursula_le_guin%40gmail.com",
        "name=Leonardo&email=leo%40gmail.com",
    ];
    let bodies_unconfirmed = vec!["name=Joe&email=joe%40gmail.com"];

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Subscribe
    for body in bodies_confirmed {
        app.post_subscriptions(body.into()).await;
        let email_request = &app.email_server.received_requests().await.unwrap()[0];
        let confirmation_links = app.get_confirmation_links(email_request);

        reqwest::get(confirmation_links.html)
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }

    for body in bodies_unconfirmed {
        app.post_subscriptions(body.into()).await;
    }

    // Act - Part 2 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    }))
    .await;

    // Act - Part 3 - Get Subscribed users
    let json_text = dbg!(app.get_subscriber_list().await);
    assert_eq!(json_text, "[".to_string()+
        "{\"email\":\"joe@gmail.com\",\"name\":\"Joe\",\"status\":\"pending_confirmation\"},"+
        "{\"email\":\"ursula_le_guin@gmail.com\",\"name\":\"le guin\",\"status\":\"confirmed\"},"+
        "{\"email\":\"leo@gmail.com\",\"name\":\"Leonardo\",\"status\":\"pending_confirmation\"}"+
        "]")
}
