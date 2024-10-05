use zbus::interface;

pub struct CosmicAccounts {
    accounts: Vec<String>,
}

impl CosmicAccounts {
    pub fn new() -> Self {
        Self {
            accounts: vec![
                "user1".to_string(),
                "user2".to_string(),
                "user3".to_string(),
            ],
        }
    }
}

#[interface(name = "com.system76.CosmicAccounts")]
impl CosmicAccounts {
    fn get_accounts(&self) -> Vec<String> {
        self.accounts.clone()
    }
}
