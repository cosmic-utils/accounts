use crate::{models::Account, proxy::ContactsProxy};
use zbus::{Connection, fdo::Result};

#[derive(Debug, Clone)]
pub struct ContactsClient {
    proxy: ContactsProxy<'static>,
    account: Account,
}

impl ContactsClient {
    pub async fn new(account: &Account) -> Result<Self> {
        let connection = Connection::session().await?;
        let proxy = ContactsProxy::new(
            &connection,
            format!("/dev/edfloreshz/Accounts/Accounts/{}", account.dbus_id()),
        )
        .await?;
        Ok(Self {
            proxy,
            account: account.clone(),
        })
    }

    pub async fn uri(&self) -> Result<String> {
        self.proxy.uri().await
    }

    pub async fn auth_method(&self) -> Result<String> {
        self.proxy.auth_method().await
    }
}
