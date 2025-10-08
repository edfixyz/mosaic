use miden_client::account::AccountId;

pub struct Account {
    miden_account: miden_client::account::Account,
}

impl Account {
    pub fn miden_id(self) -> AccountId {
        self.miden_account.id()
    }

    // Constructor for testing purposes - expose it publicly for test modules
    pub fn from_miden_account(miden_account: miden_client::account::Account) -> Self {
        Account { miden_account }
    }
}
