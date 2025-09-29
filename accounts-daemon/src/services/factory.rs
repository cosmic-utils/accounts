use crate::models::Capability;

use super::*;

pub struct ServiceFactory;

impl ServiceFactory {
    /// Create all supported services for an account
    pub fn create_services_for_account(account: &Account) -> Vec<Box<dyn Service>> {
        let mut services: Vec<Box<dyn Service>> = Vec::new();
        let account_id = account.id.to_string();

        if account.capabilities.contains_key(&Capability::Email) {
            services.push(Box::new(MailService::new(account_id.clone())));
        }

        if account.capabilities.contains_key(&Capability::Calendar) {
            services.push(Box::new(CalendarService::new(account_id.clone())));
        }

        if account.capabilities.contains_key(&Capability::Contacts) {
            services.push(Box::new(ContactsService::new(account_id.clone())));
        }

        services
    }

    /// Create a specific service by name
    pub fn create_service(service_name: &str, account_id: String) -> Option<Box<dyn Service>> {
        match service_name.to_lowercase().as_str() {
            "mail" => Some(Box::new(MailService::new(account_id))),
            "calendar" => Some(Box::new(CalendarService::new(account_id))),
            "contacts" => Some(Box::new(ContactsService::new(account_id))),
            "todo" => Some(Box::new(TodoService::new(account_id))),
            _ => None,
        }
    }

    /// Get all available service names
    pub fn available_services() -> Vec<&'static str> {
        vec!["mail", "calendar", "contacts", "tasks"]
    }
}
