use lettre::Message;
use std::env;
use tracing::{error, info};

pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub frontend_url: String,
}

impl EmailConfig {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            // **FOR DEVELOPMENT ONLY**
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .map_err(|e| format!("Invalid SMTP_PORT: {}", e))?,
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
            from_email: env::var("FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@libiamo.com".to_string()),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
        })
    }
}

pub async fn send_verification_email(
    email: &str,
    verification_token: &str,
    config: &EmailConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let verification_link = format!(
        "{}/verify-email?token={}",
        config.frontend_url, verification_token
    );

    let subject = "Libiamo - Verify Your Email".to_string();
    let body = format!(
        r#"
        <html>
            <body>
                <h2>Welcome to Libiamo!</h2>
                <p>Thank you for registering. Please click the link below to verify your email address:</p>
                <p>
                    <a href="{}" style="background-color: #007bff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px;">
                        Verify Email
                    </a>
                </p>
                <p>Or copy the link below and open it in your browser:</p>
                <p>{}</p>
                <p>This link will expire in 24 hours.</p>
                <p>If you did not register for a Libiamo account, please ignore this email.</p>
            </body>
        </html>
        "#,
        verification_link, verification_link
    );

    let _message = Message::builder()
        .from(config.from_email.parse()?)
        .to(email.parse()?)
        .subject(subject)
        .multipart(
            lettre::message::MultiPart::alternative()
                .singlepart(lettre::message::SinglePart::html(body)),
        )?;

    // For development, we log the email content instead of sending it
    info!("Verification email prepared for: {}", email);
    info!(
        "Verification link: {}/verify-email?token={}",
        config.frontend_url, verification_token
    );

    // **TODO: Implement actual email sending using lettre's SmtpTransport with the provided config**

    Ok(())
}

pub fn spawn_send_verification_email(
    email: String,
    verification_token: String,
    config: EmailConfig,
) {
    tokio::spawn(async move {
        match send_verification_email(&email, &verification_token, &config).await {
            Ok(_) => info!("Verification email sent successfully"),
            Err(e) => error!("Failed to send verification email: {}", e),
        }
    });
}
