use crate::{models::Account, proxy::TodoProxy};
use zbus::{Connection, fdo::Result};

#[derive(Debug, Clone)]
pub struct TodoClient {
    proxy: TodoProxy<'static>,
    account: Account,
}

impl TodoClient {
    pub async fn new(account: &Account) -> Result<Self> {
        let connection = Connection::session().await?;
        let proxy = TodoProxy::new(
            &connection,
            format!("/dev/edfloreshz/Accounts/Todo/{}", account.dbus_id()),
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

    pub async fn accept_ssl_errors(&self) -> Result<bool> {
        self.proxy.accept_ssl_errors().await
    }
}
