pub mod contract;
mod error;
pub mod helpers;
pub mod integration_tests;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;

mod osmosis {
    pub mod tokenfactory {
        pub mod v1beta1 {
            include!("prost/tokenfactory.v1beta1.rs");
        }
    }
}

mod cosmos {
    pub mod base {
        pub mod v1beta1 {
            include!("prost/cosmos.base.v1beta1.rs");
        }
    }
    pub mod bank {
        pub mod v1beta1 {
            include!("prost/cosmos.bank.v1beta1.rs");
        }
    }
}
