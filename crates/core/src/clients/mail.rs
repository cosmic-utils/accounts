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
            format!("/dev/edfloreshz/Accounts/Accounts/{}", account.dbus_id()),
        )
        .await?;
        Ok(Self {
            proxy,
            account: account.clone(),
        })
    }

    pub async fn imap_host(&self) -> Result<String> {
        self.proxy.imap_host().await
    }

    pub async fn imap_port(&self) -> Result<u16> {
        self.proxy.imap_port().await
    }

    pub async fn smtp_host(&self) -> Result<String> {
        self.proxy.smtp_host().await
    }

    pub async fn smtp_port(&self) -> Result<u16> {
        self.proxy.smtp_port().await
    }

    pub async fn auth_method(&self) -> Result<String> {
        self.proxy.auth_method().await
    }
}
