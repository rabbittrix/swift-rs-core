use std::sync::Mutex;

use crate::system::{PaymentSystem, SystemError};

pub struct AppState {
    pub system: Mutex<PaymentSystem>,
}

impl AppState {
    pub fn new() -> Result<Self, SystemError> {
        Ok(Self {
            system: Mutex::new(PaymentSystem::bootstrap()?),
        })
    }
}
