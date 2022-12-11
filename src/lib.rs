#![cfg_attr(not(feature = "std"), no_std)]
use concordium_cis2::{*};
use concordium_std::{*};

type OvlCreditAmount = u64;
type OvlAmount = TokenAmountU64;
type ProjectAddress = ContractAddress;
type Threshold = u64;

#[derive(Serialize, SchemaType)]
struct TierBaseState {
    // セールの最大割り当て数
    max_alloc: u64,
    // OVL Creditの計算倍率
    rate: u64,
    // TierのOVL Creditの総量
    ovl_credit_amount: OvlCreditAmount
}

// impl TierBaseState {
//     fn new(max_alloc: u64, rate: u64, ovl_credit_amount: OvlCreditAmount) -> Self {
//         TierBaseState {
//             max_alloc,
//             rate,
//             ovl_credit_amount
//         }
//     }
// }

#[derive(Serial, DeserialWithState, Deletable, StateClone)]
#[concordium(state_parameter = "S")]
struct StakeState<S> {
    // ステーキングされているOVLの総量
    amount: OvlAmount,
    start_at: Timestamp,
    tier: u8,
    // ユーザーのOVL Creditの総量
    ovl_credit_amount: OvlCreditAmount,
    // 利用できるOVL Creditの総量
    available_ovl_credit_amount: OvlCreditAmount,
    // OVL Creditを預けているプロジェクトと量
    staked_ovl_credit: StateMap<ProjectAddress, OvlCreditAmount, S>,
}

#[derive(Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
struct State<S: HasStateApi> {
    tier_base: StateMap<Threshold, TierBaseState, S>,
    stake: StateMap<AccountAddress, StakeState<S>, S>,
    paused: bool,
}

/// Contract error type
#[derive(Serialize, Debug, PartialEq, Eq, Reject, SchemaType)]
enum ContractError {
    /// Failed parsing the parameter.
    #[from(ParseError)]
    ParseParams,
    /// Failed logging: Log is full.
    LogFull,
    /// Failed logging: Log is malformed.
    LogMalformed,
    /// Contract is paused.
    ContractPaused,
    /// Failed to invoke a contract.
    InvokeContractError,
    /// Failed to invoke a transfer.
    InvokeTransferError,
    /// Upgrade failed because the new module does not exist.
    FailedUpgradeMissingModule,
    /// Upgrade failed because the new module does not contain a contract with a
    /// matching name.
    FailedUpgradeMissingContract,
    /// Upgrade failed because the smart contract version of the module is not
    /// supported.
    FailedUpgradeUnsupportedModuleVersion,
}

type ContractResult = Result<(), ContractError>;

impl From<LogError> for ContractError {
    fn from(le: LogError) -> Self {
        match le {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed,
        }
    }
}

impl<S: HasStateApi> State<S> {
    fn new(
        state_builder: &mut StateBuilder<S>,
    ) -> Self {
        State {
            stake: state_builder.new_map(),
            tier_base: state_builder.new_map(),
            paused: false,
        }
    }
}

/// Init function that creates a new contract.
#[init(
    contract = "overlay_staking",
    enable_logger,
    // parameter = "InitParams",
    // event = "Event"
)]
fn contract_init<S: HasStateApi>(
    _ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
    _logger: &mut impl HasLogger,
) -> InitResult<State<S>> {
    // let params: InitParams = ctx.parameter_cursor().get()?;
    let state = State::new(
        state_builder,
    );
    Ok(state)
}
