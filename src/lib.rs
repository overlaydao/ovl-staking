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
    // ステーキングを始めた時間
    start_at: Timestamp,
    claimable_ovl: OvlAmount,
    // 現在のTierの値
    tier: u8,
    // OVL Creditを預けているプロジェクトと量
    staked_ovl_credits: StateMap<ProjectAddress, OvlCreditAmount, S>,
    // ユーザーのOVL Creditの総量
    ovl_credit_amount: OvlCreditAmount,
    // 利用できるOVL Creditの総量
    available_ovl_credit_amount: OvlCreditAmount,
}

#[derive(Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
struct State<S: HasStateApi> {
    admin: Address,
    paused: bool,
    tier_bases: StateMap<Threshold, TierBaseState, S>,
    stakes: StateMap<AccountAddress, StakeState<S>, S>,
}

#[derive(Serialize, SchemaType)]
struct UpdateTierBaseParams {
    threshold: Threshold,
    max_alloc: u64,
    rate: u64,
}

#[derive(Serialize, SchemaType)]
struct StakeParams {
    amount: OvlAmount,
}

#[derive(Serialize, SchemaType)]
struct UnstakeParams {
    amount: OvlAmount,
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
    Unauthorized,
    ContractSender,
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
        admin: Address,
    ) -> Self {
        State {
            admin,
            paused: false,
            stakes: state_builder.new_map(),
            tier_bases: state_builder.new_map(),
        }
    }
}

/// Init function that creates a new contract.
#[init(
    contract = "overlay_staking",
    // enable_logger,
    // parameter = "InitParams",
    // event = "Event"
)]
fn contract_init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
    // logger: &mut impl HasLogger,
) -> InitResult<State<S>> {
    // let params: InitParams = ctx.parameter_cursor().get()?;
    let invoker = Address::Account(ctx.init_origin());
    let state = State::new(
        state_builder,
        invoker
    );
    Ok(state)
}

#[receive(
    contract = "overlay_staking",
    name = "updateTierBase",
    parameter = "UpdateTierBaseParams",
    // enable_logger,
    mutable
)]
fn contract_update_tier_base<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    // logger: &mut impl HasLogger,
) -> ContractResult {
    ensure!(!host.state().paused, ContractError::ContractPaused);
    ensure_eq!(ctx.sender(), host.state().admin, ContractError::Unauthorized);

    let params: UpdateTierBaseParams = ctx.parameter_cursor().get()?;

    host.state_mut().tier_bases.entry(params.threshold).or_insert_with(|| TierBaseState {
        max_alloc: params.max_alloc,
        rate: params.rate,
        ovl_credit_amount: 0u64.into(),
    });

    // TODO: 再計算

    Ok(())
}


#[receive(
    contract = "overlay_staking",
    name = "stake",
    parameter = "StakeParams",
    mutable
)]
fn contract_stake<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: StakeParams = ctx.parameter_cursor().get()?;
    // Ensure that the sender is an account.
    let sender = match ctx.sender() {
        Address::Account(sender) => sender,
        Address::Contract(_) => return Err(ContractError::ContractSender),
    };
    let slot_time = ctx.metadata().slot_time();
    let new_staked_ovl_credits = host.state_builder().new_map();
    let host = host.state_mut();

    let stake = &mut *host.stakes.entry(sender).or_insert_with(|| StakeState {
        amount: TokenAmountU64(0u64),
        start_at: slot_time,
        claimable_ovl: TokenAmountU64(0u64),
        tier: 0u8.into(),
        staked_ovl_credits: new_staked_ovl_credits,
        ovl_credit_amount: 0u64.into(),
        available_ovl_credit_amount: 0u64.into(),
    });

    // TODO: claimable_ovlの計算
    stake.amount += params.amount;
    stake.start_at = slot_time;

    let tier_bases = &host.tier_bases;
    let mut index = vec!(tier_bases).len() as u8;

    let mut staked_ovl_credit = 0u64;
    for (_project_address, ovl_credit) in stake.staked_ovl_credits.iter() {
        staked_ovl_credit += *ovl_credit;
    }
    for( threshold, tier_base ) in tier_bases.iter() {
        if *threshold <= stake.amount.into() {
            stake.tier = index;
            stake.ovl_credit_amount = (stake.amount * tier_base.rate).into();
            stake.available_ovl_credit_amount = stake.ovl_credit_amount - staked_ovl_credit;
            break;
        }
        index -= 1;
    }

    if index == 0 {
        stake.tier = 0;
        stake.ovl_credit_amount = stake.amount.into();
        stake.available_ovl_credit_amount = stake.ovl_credit_amount - staked_ovl_credit;
    }

    Ok(())
}

#[receive(
    contract = "overlay_staking",
    name = "unstake",
    parameter = "UnstakeParams",
    mutable
)]
fn contract_unstake<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let _params: UnstakeParams = ctx.parameter_cursor().get()?;
    let _sender = ctx.sender();
    Ok(())
}
