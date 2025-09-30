use std::collections::HashMap;

use async_trait::async_trait;
use zbus::{
    fdo::{Error, Result},
    interface,
};

use crate::{
    models::{Account, Provider, Service},
    services::{Service, ServiceConfig},
};

pub struct MailService {
    account_id: String,
}

impl MailService {
    pub fn new(account_id: String) -> Self {
        Self { account_id }
    }
}

#[interface(name = "dev.edfloreshz.Accounts.Mail")]
impl MailService {
    /// Email address - matches GOA's EmailAddress property
    #[zbus(property)]
    async fn email_address(&self) -> Result<String> {
        // In a real implementation, this would fetch from storage
        Ok("user@example.com".to_string())
    }

    /// Display name - matches GOA's Name property
    #[zbus(property)]
    async fn name(&self) -> Result<String> {
        Ok("User Name".to_string())
    }

    // IMAP Properties - matching GOA exactly

    /// IMAP hostname - matches GOA's ImapHost
    #[zbus(property)]
    async fn imap_host(&self) -> Result<String> {
        if self.account_id.contains("google") {
            Ok("imap.gmail.com".to_string())
        } else if self.account_id.contains("microsoft") {
            Ok("outlook.office365.com".to_string())
        } else {
            Err(Error::Failed("Unsupported provider".to_string()))
        }
    }

    /// IMAP username - matches GOA's ImapUserName
    #[zbus(property)]
    async fn imap_user_name(&self) -> Result<String> {
        // Usually the email address for OAuth2
        self.email_address().await
    }

    /// Whether IMAP is supported - matches GOA's ImapSupported
    #[zbus(property)]
    async fn imap_supported(&self) -> Result<bool> {
        Ok(true)
    }

    /// Whether IMAP uses SSL - matches GOA's ImapUseSsl
    #[zbus(property)]
    async fn imap_use_ssl(&self) -> Result<bool> {
        Ok(true) // Modern providers use SSL
    }

    /// Whether IMAP uses TLS - matches GOA's ImapUseTls
    #[zbus(property)]
    async fn imap_use_tls(&self) -> Result<bool> {
        Ok(false) // Usually SSL or TLS, not both
    }

    /// Whether to accept SSL errors - matches GOA's ImapAcceptSslErrors
    #[zbus(property)]
    async fn imap_accept_ssl_errors(&self) -> Result<bool> {
        Ok(false)
    }

    // SMTP Properties - matching GOA exactly

    /// SMTP hostname - matches GOA's SmtpHost
    #[zbus(property)]
    async fn smtp_host(&self) -> Result<String> {
        if self.account_id.contains("google") {
            Ok("smtp.gmail.com".to_string())
        } else if self.account_id.contains("microsoft") {
            Ok("smtp.office365.com".to_string())
        } else {
            Err(Error::Failed("Unsupported provider".to_string()))
        }
    }

    /// SMTP username - matches GOA's SmtpUserName
    #[zbus(property)]
    async fn smtp_user_name(&self) -> Result<String> {
        self.email_address().await
    }

    /// Whether SMTP is supported - matches GOA's SmtpSupported
    #[zbus(property)]
    async fn smtp_supported(&self) -> Result<bool> {
        Ok(true)
    }

    /// Whether SMTP uses authentication - matches GOA's SmtpUseAuth
    #[zbus(property)]
    async fn smtp_use_auth(&self) -> Result<bool> {
        Ok(true)
    }

    /// Whether SMTP uses SSL - matches GOA's SmtpUseSsl
    #[zbus(property)]
    async fn smtp_use_ssl(&self) -> Result<bool> {
        Ok(false) // Usually STARTTLS
    }

    /// Whether SMTP uses TLS - matches GOA's SmtpUseTls
    #[zbus(property)]
    async fn smtp_use_tls(&self) -> Result<bool> {
        Ok(true) // STARTTLS
    }

    /// Whether to accept SMTP SSL errors - matches GOA's SmtpAcceptSslErrors
    #[zbus(property)]
    async fn smtp_accept_ssl_errors(&self) -> Result<bool> {
        Ok(false)
    }

    /// SMTP supports LOGIN auth - matches GOA's SmtpAuthLogin
    #[zbus(property)]
    async fn smtp_auth_login(&self) -> Result<bool> {
        Ok(false) // OAuth2 providers don't typically use LOGIN
    }

    /// SMTP supports PLAIN auth - matches GOA's SmtpAuthPlain
    #[zbus(property)]
    async fn smtp_auth_plain(&self) -> Result<bool> {
        Ok(false) // OAuth2 providers don't typically use PLAIN
    }

    /// SMTP supports XOAUTH2 auth - matches GOA's SmtpAuthXoauth2
    #[zbus(property)]
    async fn smtp_auth_xoauth2(&self) -> Result<bool> {
        Ok(true) // OAuth2 providers use XOAUTH2
    }
}

#[async_trait]
impl Service for MailService {
    fn name(&self) -> &str {
        "Mail"
    }

    fn interface_name(&self) -> &str {
        "dev.edfloreshz.Accounts.Mail"
    }

    fn is_supported(&self, account: &Account) -> bool {
        account.services.contains_key(&Service::Email)
    }

    async fn get_config(&self, account: &Account) -> Result<ServiceConfig> {
        let mut settings = HashMap::new();

        match account.provider {
            Provider::Google => {
                settings.insert("imap_host".to_string(), "imap.gmail.com".into());
                settings.insert("smtp_host".to_string(), "smtp.gmail.com".into());
                settings.insert("imap_use_ssl".to_string(), true.into());
                settings.insert("smtp_use_tls".to_string(), true.into());
                settings.insert("smtp_auth_xoauth2".to_string(), true.into());
            }
            Provider::Microsoft => {
                settings.insert("imap_host".to_string(), "outlook.office365.com".into());
                settings.insert("smtp_host".to_string(), "smtp.office365.com".into());
                settings.insert("imap_use_ssl".to_string(), true.into());
                settings.insert("smtp_use_tls".to_string(), true.into());
                settings.insert("smtp_auth_xoauth2".to_string(), true.into());
            }
        }

        if let Some(email) = &account.email {
            settings.insert("email_address".to_string(), email.clone().into());
            settings.insert("imap_user_name".to_string(), email.clone().into());
            settings.insert("smtp_user_name".to_string(), email.clone().into());
        }

        settings.insert("name".to_string(), account.display_name.clone().into());

        Ok(ServiceConfig {
            service_type: "Mail".to_string(),
            provider_type: account.provider.to_string(),
            settings,
        })
    }

    async fn ensure_credentials(&self, _account: &mut Account) -> Result<()> {
        Ok(())
    }
}
