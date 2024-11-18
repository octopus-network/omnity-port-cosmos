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

    #[error("IncorrectFee, required: {0}, attach: {1}, funds: {2}")]
    IncorrectFee(u128, u128, String),

    #[error("ChainNotFound")]
    ChainNotFound,

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("RedeemAmountLessThanMinAmount, min: {0}, redeem: {1}")]
    RedeemAmountLessThanMinAmount(String, String),

    #[error("Custom error message: {0}")]
    CustomError(String),

    #[error("Error message: {0}")]
    ReplyError(String),

    #[error("TargetChainNotFound")]
    TargetChainNotFound,

    #[error("TargetChainActive")]
    TargetChainDeactive,

    #[error("ChainDeactive")]
    ChainDeactive,

}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}