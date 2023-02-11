#![cfg_attr(not(feature = "std"), no_std)]
use concordium_cis2::*;
use concordium_std::*;
use strum::EnumIter;
use strum::IntoEnumIterator;

const TOKEN_ID_OVL: ContractTokenId = TokenIdUnit();
type ContractTokenId = TokenIdUnit;
type OvlCreditAmount = u64;
type OvlAmount = TokenAmountU64;
type ProjectAddress = ContractAddress;
type Threshold = u64;
type ContractTokenAmount = TokenAmountU64;
type OnReceivingCis2Parameter = OnReceivingCis2Params<ContractTokenId, ContractTokenAmount>;

const BONUS_RATE: u16 = 166;
const BONUS_RATE_RATIO: u128 = 1000000;
const BONUS_DURATION: u128 = 100;
const MIN_STAKING_DURATION: u16 = 30;
const MAX_STAKING_DURATION: u16 = 730;

#[derive(Debug, Serialize, SchemaType, Clone)]
struct TierBaseState {
    // セールの最大割り当て数
    max_alloc: u64,
    // OVL Creditの計算倍率
    rate: u64,
}

#[derive(Debug, Serialize, SchemaType, Clone)]
struct LockState {
    // ステーキングされているOVLの量
    amount: OvlAmount,
    // ステーキング期間(日数)
    duration: u16,
    // 報酬倍率(10000分率)
    bonus_rate: u16,
    // 開始時間
    start_at: Timestamp,
    // 終了時間
    end_at: Timestamp,
}

impl LockState {
    fn new(
        amount: OvlAmount,
        duration: u16,
        bonus_rate: u16,
        start_at: Timestamp,
        end_at: Timestamp,
    ) -> Self {
        LockState {
            amount: amount,
            duration: duration,
            bonus_rate: bonus_rate,
            start_at: start_at,
            end_at: end_at,
        }
    }
}

#[derive(Debug, Serial, DeserialWithState, Deletable, StateClone)]
#[concordium(state_parameter = "S")]
struct StakeState<S> {
    // ステーキングされているOVLの総量
    amount: OvlAmount,
    // 現在のTierの値
    tier: u8,
    // OVL Creditを預けているプロジェクトと量
    staked_ovl_credits: StateMap<ProjectAddress, OvlCreditAmount, S>,
    // ユーザーのOVL Creditの総量
    ovl_credit_amount: OvlCreditAmount,
    // 利用できるOVL Creditの総量
    available_ovl_credit_amount: OvlCreditAmount,
    // ロックの一覧
    locks: collections::BTreeMap<Timestamp, LockState>,
}

impl<S: HasStateApi> StakeState<S> {
    fn new(state_builder: &mut StateBuilder<S>) -> Self {
        StakeState {
            amount: 0u64.into(),
            tier: 0u8.into(),
            staked_ovl_credits: state_builder.new_map(),
            ovl_credit_amount: 0u64.into(),
            available_ovl_credit_amount: 0u64.into(),
            locks: collections::BTreeMap::new(),
        }
    }
}

#[derive(Debug, Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
struct State<S: HasStateApi> {
    admin: Address,
    paused: bool,
    token_address: ContractAddress,
    tier_bases: collections::BTreeMap<Threshold, TierBaseState>,
    stakes: StateMap<Address, StakeState<S>, S>,
    total_staked_amount: OvlAmount,
    ovl_safe_amount: OvlAmount,
}

#[derive(Debug, Serialize, SchemaType)]
struct TierBaseParams {
    threshold: Threshold,
    max_alloc: u64,
    rate: u64,
}

type UpdateTierBaseParams = Vec<TierBaseParams>;

#[derive(Debug, Serialize, SchemaType)]
struct InitParams {
    token_address: ContractAddress,
    tier_bases: UpdateTierBaseParams,
}

#[derive(Debug, Serialize, SchemaType)]
struct UnstakeParams {
    start_at: Timestamp,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewUnstakeParams {
    owner: Address,
    start_at: Timestamp,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewLockStakingRewardParams {
    owner: Address,
    start_at: Timestamp,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewCalcStakingRewardParams {
    amount: OvlAmount,
    duration: u16,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewCalcEarlyFeeParams {
    amount: OvlAmount,
    duration: u16,
    staking_days: u16,
}

#[derive(Debug, Serialize, SchemaType)]
struct TransferFromParams {
    from: Address,
    to: Receiver,
    amount: ContractTokenAmount,
}

#[derive(Debug, Serialize, SchemaType)]
struct WithdrawOvlSafeParameter {
    amount: OvlAmount,
}

#[derive(Debug, Serialize, SchemaType)]
struct DepositOvlCreditParams {
    project_address: ProjectAddress,
    ovl_credit_amount: OvlCreditAmount,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewStakingRewardResponse {
    staking_reward: u64,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewUnstakeResponse {
    staked_amount: u64,
    staking_days: u16,
    early_fee_days: u16,
    left_days: u16,
    early_fee: u64,
    staking_reward: u64,
    send_amount: u64,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewStakeParams {
    owner: Address,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewTierBaseParams {
    threshold: Threshold,
    max_alloc: u64,
    rate: u64,
}

type ViewTierBasesResponse = Vec<(u8, ViewTierBaseParams)>;

#[derive(Debug, Serialize, SchemaType)]
struct ViewStakeResponse {
    amount: OvlAmount,
    tier: u8,
    staked_ovl_credits: Vec<(ProjectAddress, OvlCreditAmount)>,
    ovl_credit_amount: OvlCreditAmount,
    available_ovl_credit_amount: OvlCreditAmount,
    locks: collections::BTreeMap<Timestamp, LockState>,
}

#[derive(Debug, Serialize, SchemaType)]
struct ViewResponse {
    admin: Address,
    paused: bool,
    token_address: ContractAddress,
    total_staked_amount: OvlAmount,
    ovl_safe_amount: OvlAmount,
}

#[derive(Serialize, SchemaType)]
#[repr(transparent)]
struct SetPausedParams {
    paused: bool,
}

#[derive(Debug, Serialize, SchemaType)]
struct UpgradeParams {
    module: ModuleReference,
    migrate: Option<(OwnedEntrypointName, OwnedParameter)>,
}

/// Contract error type
#[derive(Serialize, Debug, PartialEq, Eq, Reject, SchemaType, EnumIter)]
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
    InsufficientOvl,
    InsufficientOvlCredit,
    ProjectNotFound,
    InsufficientDepositedOvlCredit,
    StakeOwnerNotFound,
    InvalidSender,
    LockNotFound,
    InvalidDuration,
    NotEnoughMinStakingDuration,
    OverMaxStakingDuration,
    OverflowError,
    NotEnoughOvlSafe,
    InvalidStakingDuration,
}

type ContractResult<A> = Result<A, ContractError>;

/// Mapping the logging errors to ContractError.
impl From<LogError> for ContractError {
    fn from(le: LogError) -> Self {
        match le {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed,
        }
    }
}

/// Mapping errors related to contract invocations to ContractError.
impl<T> From<CallContractError<T>> for ContractError {
    fn from(_cce: CallContractError<T>) -> Self {
        Self::InvokeContractError
    }
}

/// Mapping errors related to contract invocations to ContractError.
impl From<TransferError> for ContractError {
    fn from(_te: TransferError) -> Self {
        Self::InvokeTransferError
    }
}

/// Mapping errors related to contract upgrades to ContractError.
impl From<UpgradeError> for ContractError {
    #[inline(always)]
    fn from(ue: UpgradeError) -> Self {
        match ue {
            UpgradeError::MissingModule => Self::FailedUpgradeMissingModule,
            UpgradeError::MissingContract => Self::FailedUpgradeMissingContract,
            UpgradeError::UnsupportedModuleVersion => Self::FailedUpgradeUnsupportedModuleVersion,
        }
    }
}

impl<S: HasStateApi> State<S> {
    fn new(
        state_builder: &mut StateBuilder<S>,
        admin: Address,
        token_address: ContractAddress,
        tier_bases: collections::BTreeMap<Threshold, TierBaseState>,
    ) -> Self {
        State {
            admin,
            paused: false,
            stakes: state_builder.new_map(),
            token_address,
            tier_bases,
            total_staked_amount: TokenAmountU64(0),
            ovl_safe_amount: TokenAmountU64(0),
        }
    }

    fn stake(
        &mut self,
        owner: &Address,
        amount: &ContractTokenAmount,
        duration: u16,
        start_at: &Timestamp,
        state_builder: &mut StateBuilder<S>,
    ) -> ContractResult<()> {
        let mut stake = self
            .stakes
            .entry(*owner)
            .or_insert_with(|| StakeState::new(state_builder));

        ensure!(
            MIN_STAKING_DURATION <= duration,
            ContractError::NotEnoughMinStakingDuration
        );
        ensure!(
            duration <= MAX_STAKING_DURATION,
            ContractError::OverMaxStakingDuration
        );

        let end_at = Timestamp::from(*start_at)
            .checked_add(Duration::from_days(duration.into()))
            .ok_or(ContractError::InvalidDuration)?;

        let lock_state = LockState::new(*amount, duration, BONUS_RATE, *start_at, end_at);
        stake.amount += *amount;

        let tier_bases = &self.tier_bases;
        let mut index = tier_bases.len();

        let mut staked_ovl_credit = 0u64;
        for (_project_address, ovl_credit) in stake.staked_ovl_credits.iter() {
            staked_ovl_credit += *ovl_credit;
        }
        for (threshold, tier_base) in tier_bases.iter().rev() {
            if *threshold <= stake.amount.into() {
                // Calculate as a percentage.
                let calculated_ovl_credit = stake.amount * tier_base.rate.into();
                // Convert to a string and remove the last two digits.
                let mut ovl_credit_str = u64::from(calculated_ovl_credit).to_string();
                ovl_credit_str.pop();
                ovl_credit_str.pop();

                stake.tier = index as u8;
                stake.ovl_credit_amount = ovl_credit_str.parse::<u64>().unwrap();
                stake.available_ovl_credit_amount = stake.ovl_credit_amount - staked_ovl_credit;
                stake.locks.insert(*start_at, lock_state.clone());
                self.total_staked_amount += *amount;
                break;
            }
            index -= 1;
        }

        if index == 0 {
            stake.tier = 0;
            stake.ovl_credit_amount = stake.amount.into();
            stake.available_ovl_credit_amount = stake.ovl_credit_amount - staked_ovl_credit;
            stake.locks.insert(*start_at, lock_state.clone());
            self.total_staked_amount += *amount;
        }

        Ok(())
    }

    fn unstake(&mut self, owner: &Address, start_at: &Timestamp) -> ContractResult<()> {
        let mut stake = self
            .stakes
            .get_mut(owner)
            .ok_or(ContractError::StakeOwnerNotFound)?;
        let lock_state = stake
            .locks
            .get(start_at)
            .ok_or(ContractError::LockNotFound)?;

        let curr_amount = u64::from(stake.amount);
        let staked_amount = lock_state.amount;

        ensure!(
            curr_amount >= u64::from(staked_amount),
            ContractError::InsufficientOvl
        );

        let after_amount = curr_amount - u64::from(staked_amount);

        let tier_bases = &self.tier_bases;
        let mut index = tier_bases.len();

        let mut staked_ovl_credit = 0u64;
        for (_project_address, ovl_credit) in stake.staked_ovl_credits.iter() {
            staked_ovl_credit += *ovl_credit;
        }
        for (threshold, tier_base) in tier_bases.iter().rev() {
            if *threshold <= after_amount {
                // Calculate as a percentage.
                let calculated_ovl_credit: u64 = after_amount * tier_base.rate;
                // Convert to a string and remove the last two digits.
                let mut ovl_credit_str = u64::from(calculated_ovl_credit).to_string();
                ovl_credit_str.pop();
                ovl_credit_str.pop();

                let ovl_credit_amount = ovl_credit_str.parse::<u64>().unwrap();

                ensure!(
                    ovl_credit_amount >= staked_ovl_credit,
                    ContractError::InsufficientOvlCredit
                );
                stake.amount = after_amount.into();
                stake.tier = index as u8;
                stake.ovl_credit_amount = ovl_credit_amount;
                stake.available_ovl_credit_amount = ovl_credit_amount - staked_ovl_credit;
                stake.locks.remove(start_at);
                self.total_staked_amount -= staked_amount;
                break;
            }
            index -= 1;
        }

        if index == 0 {
            ensure!(
                after_amount >= staked_ovl_credit,
                ContractError::InsufficientOvlCredit
            );
            stake.amount = after_amount.into();
            stake.tier = 0;
            stake.ovl_credit_amount = after_amount.into();
            stake.available_ovl_credit_amount = stake.ovl_credit_amount - staked_ovl_credit;
            stake.locks.remove(start_at);
            self.total_staked_amount -= staked_amount;
        }

        Ok(())
    }

    fn deposit_ovl_credit(
        &mut self,
        owner: &Address,
        project_address: ProjectAddress,
        ovl_credit_amount: &OvlCreditAmount,
        state_builder: &mut StateBuilder<S>,
    ) -> ContractResult<()> {
        let mut stake = self
            .stakes
            .entry(*owner)
            .or_insert_with(|| StakeState::new(state_builder));

        ensure!(
            stake.available_ovl_credit_amount >= *ovl_credit_amount,
            ContractError::InsufficientOvlCredit
        );

        stake.available_ovl_credit_amount -= ovl_credit_amount;
        let mut staked_ovl_credit = stake
            .staked_ovl_credits
            .entry(project_address)
            .or_insert_with(|| 0u64);
        *staked_ovl_credit += *ovl_credit_amount;

        Ok(())
    }

    fn withdraw_ovl_credit(
        &mut self,
        owner: &Address,
        project_address: ProjectAddress,
        ovl_credit_amount: &OvlCreditAmount,
    ) -> ContractResult<()> {
        let mut stake = self
            .stakes
            .get_mut(owner)
            .ok_or(ContractError::ProjectNotFound)?;
        let staked_ovl_credit = stake
            .staked_ovl_credits
            .get(&project_address)
            .ok_or(ContractError::ProjectNotFound)?;

        ensure!(
            *staked_ovl_credit >= *ovl_credit_amount,
            ContractError::InsufficientDepositedOvlCredit
        );

        stake.available_ovl_credit_amount += ovl_credit_amount;
        *stake.staked_ovl_credits.get_mut(&project_address).unwrap() -= ovl_credit_amount;

        Ok(())
    }

    fn get_staked_ovl_credits(&self, owner: &Address) -> Vec<(ProjectAddress, OvlCreditAmount)> {
        let stake_state = match self.stakes.get(owner) {
            Some(v) => v,
            None => return vec![],
        };
        let mut staked_ovl_credits: Vec<(ProjectAddress, OvlCreditAmount)> = Vec::new();
        for (project_address, ovl_credit) in stake_state.staked_ovl_credits.iter() {
            staked_ovl_credits.push((*project_address, *ovl_credit));
        }
        return staked_ovl_credits;
    }

    fn calc_lock_staking_reward(
        &self,
        owner: &Address,
        start_at: &Timestamp,
        staking_days: Option<u16>,
    ) -> ContractResult<u64> {
        let stake = self
            .stakes
            .get(owner)
            .ok_or(ContractError::StakeOwnerNotFound)?;
        let lock_state = stake
            .locks
            .get(start_at)
            .ok_or(ContractError::LockNotFound)?;

        let amount = lock_state.amount;
        let bonus_rate = lock_state.bonus_rate;
        let duration = lock_state.duration;
        let staking_days: u16 = staking_days.unwrap_or(duration);

        let staking_reward: u64 =
            self.calc_staking_reward(amount, bonus_rate, duration, staking_days, true)?;

        Ok(staking_reward)
    }

    fn calc_staking_reward(
        &self,
        amount: OvlAmount,
        bonus_rate: u16,
        duration: u16,
        staking_days: u16,
        already_staked: bool,
    ) -> ContractResult<u64> {
        let total_ovl_safe_amount: u128 = u64::from(self.ovl_safe_amount).into();
        let mut total_staked_ovl_amount: OvlAmount = self.total_staked_amount;
        if !already_staked {
            total_staked_ovl_amount += amount;
        };
        let total_staked_amount: u128 = u64::from(total_staked_ovl_amount).into();
        let staked_amount: u128 = u64::from(amount).into();
        let bonus_rate: u128 = bonus_rate.into();
        let staking_days: u128 = staking_days.into();
        let duration: u128 = duration.into();

        let daily_bonus_amount: u128 = (total_ovl_safe_amount)
            .checked_mul(bonus_rate)
            .ok_or(ContractError::OverflowError)?;
        let mut total_bonus_amount = daily_bonus_amount
            .checked_mul(staking_days)
            .ok_or(ContractError::OverflowError)?;
        total_bonus_amount = total_bonus_amount
            .checked_mul(staked_amount)
            .ok_or(ContractError::OverflowError)?;

        let mut staking_reward_u128: u128 = total_bonus_amount * (BONUS_DURATION + duration);
        // 大きい数から割り算
        // 全ユーザーのステーキング総量からに対する割合
        staking_reward_u128 = staking_reward_u128 / total_staked_amount;
        // BONUS_RATE_RATIOで割る
        staking_reward_u128 = staking_reward_u128 / BONUS_RATE_RATIO;
        // 倍率があがる日数で割る
        staking_reward_u128 = staking_reward_u128 / BONUS_DURATION;

        // u64の最大値よりも小さい数である必要がある
        ensure!(
            staking_reward_u128 <= u128::from(u64::MAX),
            ContractError::OverflowError
        );
        let staking_reward: u64 = staking_reward_u128 as u64;

        Ok(staking_reward)
    }
}

/// Init function that creates a new contract.
#[init(contract = "ovl_staking", parameter = "InitParams")]
fn contract_init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params: InitParams = ctx.parameter_cursor().get()?;

    let mut tier_bases = collections::BTreeMap::new();

    for TierBaseParams {
        threshold,
        max_alloc,
        rate,
    } in params.tier_bases
    {
        tier_bases
            .entry(threshold)
            .or_insert_with(|| TierBaseState {
                max_alloc: max_alloc,
                rate: rate,
            });
    }

    let invoker = Address::Account(ctx.init_origin());
    let state = State::new(state_builder, invoker, params.token_address, tier_bases);

    Ok(state)
}

#[receive(
    contract = "ovl_staking",
    name = "updateTierBase",
    parameter = "UpdateTierBaseParams",
    mutable
)]
fn contract_update_tier_base<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure_eq!(
        ctx.sender(),
        host.state().admin,
        ContractError::Unauthorized
    );

    let params: UpdateTierBaseParams = ctx.parameter_cursor().get()?;

    let state = host.state_mut();

    state.tier_bases = collections::BTreeMap::new();

    for TierBaseParams {
        threshold,
        max_alloc,
        rate,
    } in params
    {
        state
            .tier_bases
            .entry(threshold)
            .or_insert_with(|| TierBaseState {
                max_alloc: max_alloc,
                rate: rate,
            });
    }

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "depositOvlSafe",
    parameter = "OnReceivingCis2Parameter",
    error = "ContractError",
    mutable
)]
fn contract_deposit_ovl_safe<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    // Sender must be contract address of token.
    let sender = match ctx.sender() {
        Address::Account(_) => return Err(ContractError::InvalidSender),
        Address::Contract(sender) => sender,
    };

    let params: OnReceivingCis2Parameter = ctx.parameter_cursor().get()?;

    ensure!(
        host.state().token_address == sender,
        ContractError::InvalidSender
    );

    host.state_mut().ovl_safe_amount += params.amount;

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "withdrawOvlSafe",
    parameter = "WithdrawOvlSafeParameter",
    error = "ContractError",
    mutable
)]
fn contract_withdraw_ovl_safe<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    // Sender must be account address.
    let sender = match ctx.sender() {
        Address::Account(sender) => sender,
        Address::Contract(_) => return Err(ContractError::InvalidSender),
    };

    ensure_eq!(
        ctx.sender(),
        host.state().admin,
        ContractError::Unauthorized
    );

    let params: WithdrawOvlSafeParameter = ctx.parameter_cursor().get()?;
    let token_address = host.state().token_address;

    ensure!(
        params.amount <= host.state().ovl_safe_amount,
        ContractError::NotEnoughOvlSafe
    );

    let transfer = Transfer {
        token_id: TOKEN_ID_OVL,
        amount: params.amount,
        from: Address::Contract(ctx.self_address()),
        to: Receiver::Account(sender),
        data: AdditionalData::empty(),
    };

    host.invoke_contract(
        &token_address,
        &TransferParams::from(vec![transfer]),
        EntrypointName::new_unchecked("transfer"),
        Amount::zero(),
    )?;

    host.state_mut().ovl_safe_amount -= params.amount;

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "stake",
    parameter = "OnReceivingCis2Parameter",
    error = "ContractError",
    mutable
)]
fn contract_stake<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    // Sender must be contract address of token.
    let sender = match ctx.sender() {
        Address::Account(_) => return Err(ContractError::InvalidSender),
        Address::Contract(sender) => sender,
    };

    ensure!(
        host.state().token_address == sender,
        ContractError::InvalidSender
    );

    let params: OnReceivingCis2Parameter = ctx.parameter_cursor().get()?;
    let duration: u16 = from_bytes(params.data.as_ref())?;
    ensure!(
        duration < MIN_STAKING_DURATION,
        ContractError::InvalidStakingDuration
    );

    ensure!(
        MAX_STAKING_DURATION < duration,
        ContractError::InvalidStakingDuration
    );

    let (state, builder) = host.state_and_builder();
    state.stake(
        &Address::Account(ctx.invoker()),
        &params.amount,
        duration,
        &ctx.metadata().slot_time(),
        builder,
    )?;

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "unstake",
    parameter = "UnstakeParams",
    error = "ContractError",
    mutable
)]
fn contract_unstake<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: UnstakeParams = ctx.parameter_cursor().get()?;

    let token_address = host.state().token_address;
    let now = ctx.metadata().slot_time();

    let invoker = &Address::from(ctx.invoker());
    let stake = host
        .state()
        .stakes
        .get(invoker)
        .ok_or(ContractError::StakeOwnerNotFound)?;
    let lock_state = stake
        .locks
        .get(&params.start_at)
        .ok_or(ContractError::LockNotFound)?;
    let mut staking_reward: u64 = 0;
    let mut send_amount = 0;
    let mut early_fee: u64 = 0;
    let mut left_days: u16 = 0;
    let mut staking_days: u16 = lock_state.duration;
    let mut early_fee_days: u16 = 0;
    let staked_amount = u64::from(lock_state.amount);

    if now < lock_state.end_at {
        // early fee
        staking_days = now.duration_between(lock_state.start_at).days() as u16;
        early_fee_days = lock_state.duration / 2;
        if early_fee_days < MIN_STAKING_DURATION {
            early_fee_days = MIN_STAKING_DURATION
        }
        left_days = lock_state.duration - staking_days;
        early_fee = host.state().calc_lock_staking_reward(
            invoker,
            &params.start_at,
            Some(early_fee_days),
        )?;
        if early_fee < staked_amount {
            send_amount = staked_amount - early_fee;
        } else {
            early_fee = staked_amount;
        }
    } else {
        staking_reward = host
            .state()
            .calc_lock_staking_reward(invoker, &params.start_at, None)?;
        send_amount = staked_amount + staking_reward;
    }

    let transfer = Transfer {
        token_id: TOKEN_ID_OVL,
        amount: TokenAmountU64::from(send_amount),
        from: Address::Contract(ctx.self_address()),
        to: Receiver::Account(ctx.invoker()),
        data: AdditionalData::empty(),
    };

    host.invoke_contract(
        &token_address,
        &TransferParams::from(vec![transfer]),
        EntrypointName::new_unchecked("transfer"),
        Amount::zero(),
    )?;

    let state = host.state_mut();
    state.ovl_safe_amount -= TokenAmountU64(staking_reward);
    state.ovl_safe_amount += TokenAmountU64(early_fee);
    state.unstake(invoker, &params.start_at)?;

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "depositOvlCredit",
    parameter = "DepositOvlCreditParams",
    error = "ContractError",
    mutable
)]
fn contract_deposit_ovl_credit<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: DepositOvlCreditParams = ctx.parameter_cursor().get()?;
    let sender = Address::from(ctx.invoker());

    let (state, builder) = host.state_and_builder();
    state.deposit_ovl_credit(
        &sender,
        params.project_address,
        &params.ovl_credit_amount,
        builder,
    )?;

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "withdrawOvlCredit",
    parameter = "DepositOvlCreditParams",
    error = "ContractError",
    mutable
)]
fn contract_withdraw_ovl_credit<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: DepositOvlCreditParams = ctx.parameter_cursor().get()?;
    let sender = Address::from(ctx.invoker());

    let state = host.state_mut();
    state.withdraw_ovl_credit(&sender, params.project_address, &params.ovl_credit_amount)?;

    Ok(())
}

type ContractErrorParams = Vec<ContractError>;

#[receive(
    contract = "ovl_staking",
    name = "viewErrors",
    return_value = "ContractErrorParams",
    error = "ContractError"
)]
fn contract_view_errors<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    _host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ContractErrorParams> {
    let mut response = vec![];
    for err in ContractError::iter() {
        response.push(err);
    }
    Ok(response)
}

#[receive(
    contract = "ovl_staking",
    name = "view",
    return_value = "ViewResponse",
    error = "ContractError"
)]
fn contract_view<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewResponse> {
    let state = host.state();
    let response: ViewResponse = ViewResponse {
        admin: state.admin,
        paused: state.paused,
        token_address: state.token_address,
        total_staked_amount: state.total_staked_amount,
        ovl_safe_amount: state.ovl_safe_amount,
    };
    Ok(response)
}

#[receive(
    contract = "ovl_staking",
    name = "viewLockStakingReward",
    parameter = "ViewLockStakingRewardParams",
    return_value = "ViewStakingRewardResponse",
    error = "ContractError"
)]
fn contract_view_lock_staking_reward<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewStakingRewardResponse> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: ViewLockStakingRewardParams = ctx.parameter_cursor().get()?;
    let staking_reward: u64 =
        host.state()
            .calc_lock_staking_reward(&params.owner, &params.start_at, None)?;

    Ok(ViewStakingRewardResponse { staking_reward })
}

#[receive(
    contract = "ovl_staking",
    name = "viewCalcStakingReward",
    parameter = "ViewCalcStakingRewardParams",
    return_value = "ViewStakingRewardResponse",
    error = "ContractError"
)]
fn contract_view_calc_staking_reward<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewStakingRewardResponse> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: ViewCalcStakingRewardParams = ctx.parameter_cursor().get()?;
    let staking_reward: u64 = host.state().calc_staking_reward(
        params.amount,
        BONUS_RATE,
        params.duration,
        params.duration,
        false,
    )?;

    Ok(ViewStakingRewardResponse { staking_reward })
}

#[receive(
    contract = "ovl_staking",
    name = "viewUnstake",
    parameter = "ViewUnstakeParams",
    return_value = "ViewUnstakeResponse",
    error = "ContractError"
)]
fn contract_view_unstake<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewUnstakeResponse> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: ViewUnstakeParams = ctx.parameter_cursor().get()?;
    let sender = &params.owner;
    let now = ctx.metadata().slot_time();
    let stake = host
        .state()
        .stakes
        .get(sender)
        .ok_or(ContractError::StakeOwnerNotFound)?;
    let lock_state = stake
        .locks
        .get(&params.start_at)
        .ok_or(ContractError::LockNotFound)?;
    let mut staking_reward: u64 = 0;
    let mut send_amount = 0;
    let mut early_fee: u64 = 0;
    let mut left_days: u16 = 0;
    let mut staking_days: u16 = lock_state.duration;
    let mut early_fee_days: u16 = 0;
    let staked_amount = u64::from(lock_state.amount);

    if now < lock_state.end_at {
        // early fee
        staking_days = now.duration_between(lock_state.start_at).days() as u16;
        early_fee_days = lock_state.duration / 2;
        if early_fee_days < MIN_STAKING_DURATION {
            early_fee_days = MIN_STAKING_DURATION
        }
        left_days = lock_state.duration - staking_days;
        early_fee = host.state().calc_lock_staking_reward(
            sender,
            &params.start_at,
            Some(early_fee_days),
        )?;
        if early_fee < staked_amount {
            send_amount = staked_amount - early_fee;
        } else {
            early_fee = staked_amount;
        }
    } else {
        staking_reward = host
            .state()
            .calc_lock_staking_reward(sender, &params.start_at, None)?;
        send_amount = staked_amount + staking_reward;
    }

    Ok(ViewUnstakeResponse {
        staked_amount,
        staking_days,
        early_fee_days,
        left_days,
        early_fee,
        staking_reward,
        send_amount,
    })
}

#[receive(
    contract = "ovl_staking",
    name = "viewCalcEarlyFee",
    parameter = "ViewCalcEarlyFeeParams",
    return_value = "ViewUnstakeResponse",
    error = "ContractError"
)]
fn contract_view_calc_early_fee<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewUnstakeResponse> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: ViewCalcEarlyFeeParams = ctx.parameter_cursor().get()?;

    let staking_reward: u64 = 0;
    let mut send_amount: u64 = 0;
    let staked_amount: u64 = u64::from(params.amount);
    let duration: u16 = params.duration;

    // early fee
    let staking_days = &params.staking_days;
    let mut early_fee_days = duration / 2;
    if early_fee_days < MIN_STAKING_DURATION {
        early_fee_days = MIN_STAKING_DURATION
    }
    let left_days = duration - staking_days;
    let early_fee: u64 = host.state().calc_staking_reward(
        params.amount,
        BONUS_RATE,
        params.duration,
        early_fee_days,
        false,
    )?;
    if early_fee < staked_amount {
        send_amount = staked_amount - early_fee;
    }

    Ok(ViewUnstakeResponse {
        staked_amount,
        staking_days: params.staking_days,
        early_fee_days: early_fee_days.into(),
        left_days: left_days.into(),
        early_fee,
        staking_reward,
        send_amount,
    })
}

#[receive(
    contract = "ovl_staking",
    name = "viewTierBases",
    return_value = "ViewTierBasesResponse",
    error = "ContractError"
)]
fn contract_view_tier_bases<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewTierBasesResponse> {
    let tier_bases = &host.state().tier_bases;
    let mut tier_bases_response: ViewTierBasesResponse = vec![];
    let mut index = 1;
    for (threshold, tier_base) in tier_bases.iter() {
        tier_bases_response.push((
            index,
            ViewTierBaseParams {
                threshold: *threshold,
                max_alloc: tier_base.max_alloc,
                rate: tier_base.rate,
            },
        ));
        index += 1;
    }
    Ok(tier_bases_response)
}

#[receive(
    contract = "ovl_staking",
    name = "viewStake",
    parameter = "ViewStakeParams",
    return_value = "ViewStakeResponse",
    error = "ContractError"
)]
fn contract_view_stake<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<ViewStakeResponse> {
    let params: ViewStakeParams = ctx.parameter_cursor().get()?;
    let staked_ovl_credits = host.state().get_staked_ovl_credits(&params.owner);
    let stake_state = host.state().stakes.get(&params.owner).unwrap();
    let state = ViewStakeResponse {
        amount: stake_state.amount,
        tier: stake_state.tier,
        staked_ovl_credits: staked_ovl_credits,
        ovl_credit_amount: stake_state.ovl_credit_amount,
        available_ovl_credit_amount: stake_state.available_ovl_credit_amount,
        locks: stake_state.locks.clone(),
    };
    Ok(state)
}

#[receive(
    contract = "ovl_staking",
    name = "receiveTransfer",
    parameter = "OnReceivingCis2Parameter",
    return_value = "OnReceivingCis2Parameter",
    error = "ContractError"
)]
fn contract_receive_transfer<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    _host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<OnReceivingCis2Parameter> {
    Ok(ctx.parameter_cursor().get()?)
}

/// Transfer the admin address to a new admin address.
///
/// It rejects if:
/// - Sender is not the current admin of the contract instance.
/// - It fails to parse the parameter.
#[receive(
    contract = "ovl_staking",
    name = "updateAdmin",
    parameter = "Address",
    error = "ContractError",
    mutable
)]
fn contract_update_admin<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    // Check that only the current admin is authorized to update the admin address.
    ensure_eq!(
        ctx.sender(),
        host.state().admin,
        ContractError::Unauthorized
    );

    // Parse the parameter.
    let new_admin = ctx.parameter_cursor().get()?;

    // Update the admin variable.
    host.state_mut().admin = new_admin;

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "setPaused",
    parameter = "SetPausedParams",
    error = "ContractError",
    mutable
)]
fn contract_set_paused<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure_eq!(
        ctx.sender(),
        host.state().admin,
        ContractError::Unauthorized
    );

    let params: SetPausedParams = ctx.parameter_cursor().get()?;

    host.state_mut().paused = params.paused;

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "upgrade",
    parameter = "UpgradeParams",
    error = "ContractError",
    low_level
)]
fn contract_upgrade<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<S>,
) -> ContractResult<()> {
    // Read the top-level contract state.
    let state: State<S> = host.state().read_root()?;

    // Check that only the admin is authorized to upgrade the smart contract.
    ensure_eq!(ctx.sender(), state.admin, ContractError::Unauthorized);
    // Parse the parameter.
    let params: UpgradeParams = ctx.parameter_cursor().get()?;
    // Trigger the upgrade.
    host.upgrade(params.module)?;
    // Call the migration function if provided.
    if let Some((func, parameters)) = params.migrate {
        host.invoke_contract_raw(
            &ctx.self_address(),
            parameters.as_parameter(),
            func.as_entrypoint_name(),
            Amount::zero(),
        )?;
    }
    Ok(())
}

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    // const ACCOUNT_0: AccountAddress = AccountAddress([0u8; 32]);
    // const ADDRESS_0: Address = Address::Account(ACCOUNT_0);
    // const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
    // const ADDRESS_1: Address = Address::Account(ACCOUNT_1);
    const ADMIN_ACCOUNT: AccountAddress = AccountAddress([2u8; 32]);
    const ADMIN_ADDRESS: Address = Address::Account(ADMIN_ACCOUNT);
    const NEW_ADMIN_ACCOUNT: AccountAddress = AccountAddress([3u8; 32]);
    const NEW_ADMIN_ADDRESS: Address = Address::Account(NEW_ADMIN_ACCOUNT);

    fn initial_state<S: HasStateApi>(state_builder: &mut StateBuilder<S>) -> State<S> {
        let tier1 = TierBaseState {
            max_alloc: 2,
            rate: 105,
        };

        let tier2 = TierBaseState {
            max_alloc: 4,
            rate: 110,
        };

        let tier3 = TierBaseState {
            max_alloc: 8,
            rate: 115,
        };

        let tier4 = TierBaseState {
            max_alloc: 16,
            rate: 120,
        };

        let tier5 = TierBaseState {
            max_alloc: 32,
            rate: 125,
        };

        let tier_bases = collections::BTreeMap::from([
            (1000u64, tier1),
            (2000u64, tier2),
            (3000u64, tier3),
            (4000u64, tier4),
            (5000u64, tier5),
        ]);

        let token_address = ContractAddress::new(2250, 0);
        let state = State::new(state_builder, ADMIN_ADDRESS, token_address, tier_bases);
        state
    }
    #[concordium_test]
    fn test_init() {
        // Set up the context
        let mut ctx = TestInitContext::empty();
        ctx.set_init_origin(ADMIN_ACCOUNT);

        // Set up the parameter.
        let tier1 = TierBaseParams {
            threshold: 100_000_000_000,
            max_alloc: 2,
            rate: 105,
        };

        let tier2 = TierBaseParams {
            threshold: 200_000_000_000,
            max_alloc: 4,
            rate: 110,
        };

        let tier3 = TierBaseParams {
            threshold: 500_000_000_000,
            max_alloc: 8,
            rate: 115,
        };

        let tier4 = TierBaseParams {
            threshold: 1_250_000_000_000,
            max_alloc: 16,
            rate: 120,
        };

        let tier5 = TierBaseParams {
            threshold: 2_500_000_000_000,
            max_alloc: 32,
            rate: 125,
        };

        let params = vec![tier1, tier2, tier3, tier4, tier5];
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let mut builder = TestStateBuilder::new();

        let result = contract_init(&ctx, &mut builder);
        // Check the result.
        result.expect_report("Contract initialization failed");
    }

    #[concordium_test]
    fn test_view() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let self_address = ContractAddress::new(2250, 0);
        ctx.set_self_address(self_address);

        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        let result: ContractResult<ViewResponse> = contract_view(&ctx, &mut host);
        println!("{:?}", result);

        // Check the result.
        claim!(result.is_ok(), "Results in rejection");
    }

    #[concordium_test]
    fn test_calc_lock_staking_reward() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let self_address = ContractAddress::new(2250, 0);
        ctx.set_self_address(self_address);

        ctx.metadata_mut()
            .set_slot_time(Timestamp::from_timestamp_millis(100));

        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder);
        let now = &Timestamp::from_timestamp_millis(100);
        let result1 = state.stake(
            &ADMIN_ADDRESS,
            &TokenAmountU64::from(33000000_000000),
            30u16.into(),
            now,
            &mut state_builder,
        );
        println!("{:?}", result1);
        let mut host = TestHost::new(state, state_builder);

        host.state_mut().ovl_safe_amount += TokenAmountU64(100000000_000000);

        let params2 = ViewLockStakingRewardParams {
            owner: ADMIN_ADDRESS,
            start_at: *now,
        };
        let parameter_bytes2 = to_bytes(&params2);
        ctx.set_parameter(&parameter_bytes2);
        let result2: ContractResult<ViewStakingRewardResponse> =
            contract_view_lock_staking_reward(&ctx, &mut host);
        println!("{:?}", result2);
    }

    #[concordium_test]
    fn test_calc_staking_reward() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let self_address = ContractAddress::new(2250, 0);
        ctx.set_self_address(self_address);

        ctx.metadata_mut()
            .set_slot_time(Timestamp::from_timestamp_millis(100));

        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        host.state_mut().ovl_safe_amount += TokenAmountU64(100000000_000000);

        let params1 = ViewCalcStakingRewardParams {
            amount: TokenAmountU64(33000000_000000),
            duration: 30u16,
        };
        let parameter_bytes1 = to_bytes(&params1);
        ctx.set_parameter(&parameter_bytes1);
        let result1: ContractResult<ViewStakingRewardResponse> =
            contract_view_calc_staking_reward(&ctx, &mut host);
        println!("{:?}", result1);
    }

    #[concordium_test]
    fn test_stake() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let self_address = ContractAddress::new(2250, 0);
        ctx.set_self_address(self_address);

        ctx.metadata_mut()
            .set_slot_time(Timestamp::from_timestamp_millis(100));

        // Set up the parameter.
        // let params = OnReceivingCis2Parameter {
        //     duration:  30u16.into(),
        // };
        // let parameter_bytes = to_bytes(&params);
        // ctx.set_parameter(&parameter_bytes);

        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder);
        state.stake(
            &ADMIN_ADDRESS,
            &TokenAmountU64::from(500),
            30u16.into(),
            &ctx.metadata().slot_time(),
            &mut state_builder,
        );
        let mut host = TestHost::new(state, state_builder);

        // TODO: need contract mock
        // let result: ContractResult<()> = contract_stake(&ctx, &mut host);
        // println!("{:?}", result);

        // // Check the result.
        // claim!(result.is_ok(), "Results in rejection");

        let params2 = ViewStakeParams {
            owner: ADMIN_ADDRESS,
        };
        let parameter_bytes2 = to_bytes(&params2);
        ctx.set_parameter(&parameter_bytes2);
        let result2: ContractResult<ViewStakeResponse> = contract_view_stake(&ctx, &mut host);
        println!("{:?}", result2);
    }

    #[concordium_test]
    fn test_unstake() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let self_address = ContractAddress::new(2250, 0);
        ctx.set_self_address(self_address);

        ctx.metadata_mut()
            .set_slot_time(Timestamp::from_timestamp_millis(100));

        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder);

        // first stake
        state.stake(
            &ADMIN_ADDRESS,
            &TokenAmountU64::from(5000),
            30u16.into(),
            &ctx.metadata().slot_time(),
            &mut state_builder,
        );

        // first unstake
        let unstake_result = state.unstake(&ADMIN_ADDRESS, &ctx.metadata().slot_time());
        println!("{:?}", unstake_result);

        let mut host = TestHost::new(state, state_builder);

        let params = ViewStakeParams {
            owner: ADMIN_ADDRESS,
        };
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);
        let result: ContractResult<ViewStakeResponse> = contract_view_stake(&ctx, &mut host);
        println!("{:?}", result);
    }

    #[concordium_test]
    fn test_deposit_and_withdraw_ovl_credit() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let self_address = ContractAddress::new(2250, 0);
        ctx.set_self_address(self_address);

        ctx.metadata_mut()
            .set_slot_time(Timestamp::from_timestamp_millis(100));

        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder);

        // stake
        state.stake(
            &ADMIN_ADDRESS,
            &TokenAmountU64::from(5000),
            30u16.into(),
            &ctx.metadata().slot_time(),
            &mut state_builder,
        );

        let mut host = TestHost::new(state, state_builder);

        // deposit
        let deposit_params = DepositOvlCreditParams {
            project_address: ContractAddress::new(2250, 0),
            ovl_credit_amount: 2000,
        };
        let deposit_parameter_bytes = to_bytes(&deposit_params);
        ctx.set_parameter(&deposit_parameter_bytes);
        let deposit_result: ContractResult<()> = contract_deposit_ovl_credit(&ctx, &mut host);
        println!("{:?}", deposit_result);

        // withdraw
        let withdraw_params = DepositOvlCreditParams {
            project_address: ContractAddress::new(2250, 0),
            ovl_credit_amount: 1200,
        };
        let withdraw_parameter_bytes = to_bytes(&withdraw_params);
        ctx.set_parameter(&withdraw_parameter_bytes);
        let withdraw_result: ContractResult<()> = contract_withdraw_ovl_credit(&ctx, &mut host);
        println!("{:?}", withdraw_result);

        // view stake
        let view_params = ViewStakeParams {
            owner: ADMIN_ADDRESS,
        };

        let view_parameter_bytes = to_bytes(&view_params);
        ctx.set_parameter(&view_parameter_bytes);
        let view_result: ContractResult<ViewStakeResponse> = contract_view_stake(&ctx, &mut host);
        println!("{:?}", view_result);
    }

    /// Test admin can update to a new admin address.
    #[concordium_test]
    fn test_update_admin() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        // Set up the parameter.
        let parameter_bytes = to_bytes(&[NEW_ADMIN_ADDRESS]);
        ctx.set_parameter(&parameter_bytes);

        // Set up the state and host.
        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        // Check the admin state.
        claim_eq!(
            host.state().admin,
            ADMIN_ADDRESS,
            "Admin should be the old admin address"
        );

        // Call the contract function.
        let result: ContractResult<()> = contract_update_admin(&ctx, &mut host);

        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        // Check the admin state.
        claim_eq!(
            host.state().admin,
            NEW_ADMIN_ADDRESS,
            "Admin should be the new admin address"
        );
    }

    #[concordium_test]
    fn test_update_tier_base() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        // Set up the parameter.
        let tier1 = TierBaseParams {
            threshold: 150_000_000_000,
            max_alloc: 2,
            rate: 105,
        };

        let tier2 = TierBaseParams {
            threshold: 200_000_000_000,
            max_alloc: 4,
            rate: 110,
        };

        let tier3 = TierBaseParams {
            threshold: 300_000_000_000,
            max_alloc: 8,
            rate: 120,
        };

        let tier4 = TierBaseParams {
            threshold: 1_250_000_000_000,
            max_alloc: 16,
            rate: 130,
        };

        let tier5 = TierBaseParams {
            threshold: 3_000_000_000_000,
            max_alloc: 32,
            rate: 140,
        };

        let params = vec![tier1, tier2, tier3, tier4, tier5];
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        let result: ContractResult<()> = contract_update_tier_base(&ctx, &mut host);
        println!("{:?}", result);
        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let result2: ContractResult<ViewTierBasesResponse> =
            contract_view_tier_bases(&ctx, &mut host);
        println!("{:?}", result2);
    }
}
