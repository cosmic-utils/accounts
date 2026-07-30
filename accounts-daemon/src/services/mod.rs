mod calendar;
// mod contacts;
// pub use contacts::*;
mod mail;
mod todo;

use accounts::{
    AccountService,
    models::{Account, Service},
};
pub use calendar::*;
pub use mail::*;
pub use todo::*;

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

        services
    }

    pub fn create_service(account: &Account, service: &Service) -> Option<Box<dyn AccountService>> {
        match service {
            Service::Calendar => Some(Box::new(CalendarService::new(account.clone()))),
            Service::Email => Some(Box::new(MailService::new(account.clone()))),
            Service::Todo => Some(Box::new(TodoService::new(account.clone()))),
            _ => None,
        }
    }
}
