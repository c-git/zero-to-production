use crate::domain::SubscriberEmail;
use anyhow::Context;
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, Message, SmtpTransport,
    Transport,
};
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

pub struct EmailClient {
    sender: SubscriberEmail,
    timeout: std::time::Duration,
    kind_email_provider: KindEmailProvider,
}

pub enum KindEmailProvider {
    URL(EmailProviderURL),
    SMTP(EmailProviderSMTP),
}

pub struct EmailProviderURL {
    http_client: Client,
    base_url: String,
    authorization_token: Secret<String>,
}

pub struct EmailProviderSMTP {
    /// The name to put on outgoing emails
    name: Option<String>,
    /// The username to use to log into the SMTP server, if not provided [`EmailClient::sender`] is
    /// used (eg. For Gmail they are the same and this can be None)
    username: Option<String>,
    password: Secret<String>,
    smtp_server: String,
}

impl EmailClient {
    pub fn new(
        sender: SubscriberEmail,
        timeout: std::time::Duration,
        kind_email_provider: KindEmailProvider,
    ) -> Self {
        Self {
            sender,
            timeout,
            kind_email_provider,
        }
    }

    pub fn new_url(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        let kind_email_provider = EmailProviderURL {
            http_client,
            base_url,
            authorization_token,
        };
        Self {
            sender,
            timeout,
            kind_email_provider: KindEmailProvider::URL(kind_email_provider),
        }
    }

    pub fn new_smtp(
        sender: SubscriberEmail,
        timeout: std::time::Duration,
        name: Option<String>,
        username: Option<String>,
        password: Secret<String>,
        smtp_server: String,
    ) -> Self {
        let kind_email_provider = EmailProviderSMTP {
            name,
            username,
            password,
            smtp_server,
        };
        Self {
            sender,
            timeout,
            kind_email_provider: KindEmailProvider::SMTP(kind_email_provider),
        }
    }

    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), anyhow::Error> {
        match &self.kind_email_provider {
            KindEmailProvider::URL(kind_url) => {
                let url = format!("{}/email", kind_url.base_url);
                let request_body = SendEmailRequest {
                    from: self.sender.as_ref(),
                    to: recipient.as_ref(),
                    subject,
                    html_body: html_content,
                    text_body: text_content,
                };
                kind_url
                    .http_client
                    .post(&url)
                    .header(
                        "X-Postmark-Server-Token",
                        kind_url.authorization_token.expose_secret(),
                    )
                    .json(&request_body)
                    .send()
                    .await?
                    .error_for_status()?;
            }
            KindEmailProvider::SMTP(kind_smtp) => {
                let from = Mailbox {
                    name: kind_smtp.name.to_owned(),
                    email: self
                        .sender
                        .as_ref()
                        .parse()
                        .context("Failed to parse email address")?,
                };

                // TODO: Implement use of html_body
                let email = Message::builder()
                    .from(from)
                    .to(recipient.as_ref().parse()?)
                    .subject(subject)
                    .body(text_content.to_owned())?;

                let username = match &kind_smtp.username {
                    None => self.sender.to_string(),
                    Some(username) => username.clone(),
                };

                let mailer = SmtpTransport::relay(&kind_smtp.smtp_server)?
                    .credentials(Credentials::new(
                        username,
                        kind_smtp.password.expose_secret().to_owned(),
                    ))
                    .timeout(Some(self.timeout))
                    .build();

                // Sends the email
                mailer.send(&email)?;
            }
        }

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    // TODO: Add tests for Gmail (not quite sure how that would work yet)
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claim::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    /// Generate a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Generate a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Generate a random subscriber email
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    /// Get a test instance of `EmailClient`.
    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new_url(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            // Not a 200 anymore!
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }
}
