use crate::{models::Account, proxy::MailProxy};
use zbus::{Connection, fdo::Result};

#[derive(Debug, Clone)]
pub struct MailClient {
    proxy: MailProxy<'static>,
    account: Account,
}

impl MailClient {
    pub async fn new(account: &Account) -> Result<Self> {
        let connection = Connection::session().await?;
        let proxy = MailProxy::new(
            &connection,
            format!("/dev/edfloreshz/Accounts/Mail/{}", account.dbus_id()),
        )
        .await?;
        Ok(Self {
            proxy,
            account: account.clone(),
        })
    }

    pub async fn email_address(&self) -> Result<String> {
        self.proxy.email_address().await
    }

    pub async fn name(&self) -> Result<String> {
        self.proxy.name().await
    }

    pub async fn imap_host(&self) -> Result<String> {
        self.proxy.imap_host().await
    }

    pub async fn imap_user_name(&self) -> Result<String> {
        self.proxy.imap_user_name().await
    }

    pub async fn imap_supported(&self) -> Result<bool> {
        self.proxy.imap_supported().await
    }

    pub async fn imap_use_ssl(&self) -> Result<bool> {
        self.proxy.imap_use_ssl().await
    }

    pub async fn imap_use_tls(&self) -> Result<bool> {
        self.proxy.imap_use_tls().await
    }

    pub async fn imap_accept_ssl_errors(&self) -> Result<bool> {
        self.proxy.imap_accept_ssl_errors().await
    }

    pub async fn smtp_host(&self) -> Result<String> {
        self.proxy.smtp_host().await
    }

    pub async fn smtp_user_name(&self) -> Result<String> {
        self.proxy.smtp_user_name().await
    }

    pub async fn smtp_supported(&self) -> Result<bool> {
        self.proxy.smtp_supported().await
    }

    pub async fn smtp_use_auth(&self) -> Result<bool> {
        self.proxy.smtp_use_auth().await
    }

    pub async fn smtp_use_ssl(&self) -> Result<bool> {
        self.proxy.smtp_use_ssl().await
    }

    pub async fn smtp_use_tls(&self) -> Result<bool> {
        self.proxy.smtp_use_tls().await
    }

    pub async fn smtp_accept_ssl_errors(&self) -> Result<bool> {
        self.proxy.smtp_accept_ssl_errors().await
    }

    pub async fn smtp_auth_login(&self) -> Result<bool> {
        self.proxy.smtp_auth_login().await
    }

    pub async fn smtp_auth_plain(&self) -> Result<bool> {
        self.proxy.smtp_auth_plain().await
    }

    pub async fn smtp_auth_xoauth2(&self) -> Result<bool> {
        self.proxy.smtp_auth_xoauth2().await
    }
}
