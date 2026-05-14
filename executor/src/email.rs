use crate::error::ExecError;
use serde_json::Value;

#[cfg(feature = "email")]
pub fn execute_send_email(to: &str, subject: &str, body: &str) -> Result<Value, ExecError> {
    use lettre::Message;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{SmtpTransport, Transport};
    use std::env;
    let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string());
    let smtp_port: u16 = env::var("SMTP_PORT").unwrap_or_else(|_| "587".to_string()).parse().unwrap_or(587);
    let smtp_user = env::var("SMTP_USER").map_err(|_| ExecError::HttpError("SMTP_USER not configured".into()))?;
    let smtp_pass = env::var("SMTP_PASS").map_err(|_| ExecError::HttpError("SMTP_PASS not configured".into()))?;
    let email = Message::builder()
        .from(smtp_user.parse().map_err(|e| ExecError::HttpError(format!("Invalid from: {}", e)))?)
        .to(to.parse().map_err(|e| ExecError::HttpError(format!("Invalid to: {}", e)))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| ExecError::HttpError(format!("Email build: {}", e)))?;
    let creds = Credentials::new(smtp_user, smtp_pass);
    let mailer = SmtpTransport::starttls_relay(&smtp_host)
        .map_err(|e| ExecError::HttpError(format!("SMTP relay: {}", e)))?
        .port(smtp_port).credentials(creds).build();
    mailer.send(&email).map_err(|e| ExecError::HttpError(format!("Send email: {}", e)))?;
    Ok(serde_json::json!({"status": "sent", "to": to, "subject": subject}))
}

#[cfg(not(feature = "email"))]
pub fn execute_send_email(to: &str, subject: &str, body: &str) -> Result<Value, ExecError> {
    tracing::info!(target: "executor.email", to = %to, subject = %subject, "Email logged (enable 'email' feature): {}", body);
    Ok(serde_json::json!({"status": "logged", "to": to, "subject": subject}))
}
