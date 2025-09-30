use crate::{models::Account, proxy::CalendarProxy};
use zbus::{Connection, fdo::Result};

#[derive(Debug, Clone)]
pub struct CalendarClient {
    proxy: CalendarProxy<'static>,
    account: Account,
}

impl CalendarClient {
    pub async fn new(account: &Account) -> Result<Self> {
        let connection = Connection::session().await?;
        let proxy = CalendarProxy::new(
            &connection,
            format!("/dev/edfloreshz/Accounts/Calendar/{}", account.dbus_id()),
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
