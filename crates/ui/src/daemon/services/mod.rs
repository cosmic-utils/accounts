mod calendar;
mod contacts;
mod mail;
mod todo;

use accounts_core::{
    AccountService,
    models::{Account, Service},
};
pub use calendar::*;
pub use contacts::*;
pub use mail::*;
pub use todo::*;
use zbus::fdo::{Error, Result};

/// D-Bus object path of the `Account` that an `Endpoint.*` interface is served
/// on — the endpoints share the account's path rather than living under their
/// own service tree.
pub(crate) fn endpoint_object_path(account: &Account) -> String {
    format!("/dev/edfloreshz/Accounts/Accounts/{}", account.dbus_id())
}

/// Refresh the account's stored credentials if they have expired, so the
/// endpoint's consumer can rely on `Credentials.GetAccessToken` succeeding.
pub(crate) async fn refresh_account_credentials(account: &mut Account) -> Result<()> {
    let mut auth = crate::daemon::auth::AuthManager::new()
        .await
        .map_err(|e| Error::Failed(format!("could not open the auth manager: {e}")))?;
    auth.ensure_credentials(account)
        .await
        .map_err(|e| Error::Failed(format!("could not refresh credentials: {e}")))
}

pub struct ServiceFactory;

impl ServiceFactory {
    pub fn create_services(account: &Account) -> Vec<Box<dyn AccountService>> {
        let mut services: Vec<Box<dyn AccountService>> = Vec::new();

        if let Some((_, value)) = account.services.get_key_value(&Service::Calendar)
            && *value
        {
            services.push(Box::new(CalendarService::new(account.clone())));
        }

        if let Some((_, value)) = account.services.get_key_value(&Service::Email)
            && *value
        {
            services.push(Box::new(MailService::new(account.clone())));
        }

        if let Some((_, value)) = account.services.get_key_value(&Service::Todo)
            && *value
        {
            services.push(Box::new(TodoService::new(account.clone())));
        }

        if let Some((_, value)) = account.services.get_key_value(&Service::Contacts)
            && *value
        {
            services.push(Box::new(ContactsService::new(account.clone())));
        }

        services
    }

    pub fn create_service(account: &Account, service: &Service) -> Option<Box<dyn AccountService>> {
        match service {
            Service::Calendar => Some(Box::new(CalendarService::new(account.clone()))),
            Service::Email => Some(Box::new(MailService::new(account.clone()))),
            Service::Todo => Some(Box::new(TodoService::new(account.clone()))),
            Service::Contacts => Some(Box::new(ContactsService::new(account.clone()))),
        }
    }
}
