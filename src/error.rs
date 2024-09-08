use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    #[error("TokenAleardyExist")]
    TokenAleardyExist,

    #[error("TokenNotFound")]
    TokenNotFound,

    #[error("TokenUnsupportMint")]
    TokenUnsupportMint,

    #[error("DirectiveAlreadyHandled")]
    DirectiveAlreadyHandled,

    #[error("TicketAlreadyHandled")]
    TicketAlreadyHandled,

    #[error("FeeHasNotSet")]
    FeeHasNotSet,

    #[error("InsufficientFee, required: {0}, attach: {1}, funds: {2}")]
    InsufficientFee(u128, u128, String),

    #[error("ChainNotFound")]
    ChainNotFound,

    #[error("Semver parsing error: {0}")]
    SemVer(String),

}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}