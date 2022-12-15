use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn subscribe_form(
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Change Password</title>
</head>
<body>
    {msg_html}
    <form action="/subscriptions" method="post">
        <label>Email
            <input
                type="email"
                placeholder="Enter email address"
                name="email"
            >
        </label>
        <br>
        <label>Name
            <input
                type="text"
                placeholder="Enter name"
                name="name"
            >
        </label>        
        <br>
        <button type="submit">Subscribe to Newsletter</button>
    </form>
    <p><a href="/">&lt;- Home</a></p>
</body>
</html>"#,
        )))
}
