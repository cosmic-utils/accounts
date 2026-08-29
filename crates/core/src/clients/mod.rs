#![allow(dead_code)]

mod account;
mod calendar;
mod contacts;
mod mail;
mod tasks;

pub use account::AccountsClient;
pub use calendar::CalendarClient;
pub use contacts::ContactsClient;
pub use mail::MailClient;
pub use tasks::TasksClient;
