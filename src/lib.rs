#![cfg_attr(not(feature = "std"), no_std)]
use concordium_cis2::{*};
use concordium_std::{*};

const TOKEN_ID_OVL: ContractTokenId = TokenIdUnit();

type ContractTokenId = TokenIdUnit;
type OvlCreditAmount = u64;
type OvlAmount = TokenAmountU64;
type ProjectAddress = ContractAddress;
type Threshold = u64;
type ContractTokenAmount = TokenAmountU64;

#[derive(Debug, Serialize, SchemaType, Clone)]
struct TierBaseState {
    // セールの最大割り当て数
    max_alloc: u64,
    // OVL Creditの計算倍率
    rate: u64,
    // TierのOVL Creditの総量
    ovl_credit_amount: OvlCreditAmount
}

#[derive(Debug, Serial, DeserialWithState, Deletable, StateClone)]
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

impl<S: HasStateApi> StakeState<S> {
    fn new(start_at: Timestamp, state_builder: &mut StateBuilder<S>) -> Self {
        StakeState {
            amount: 0u64.into(),
            start_at,
            claimable_ovl: TokenAmountU64(0u64),
            tier: 0u8.into(),
            staked_ovl_credits: state_builder.new_map(),
            ovl_credit_amount: 0u64.into(),
            available_ovl_credit_amount: 0u64.into(),
        }
    }
}

#[derive(Debug, Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
struct State<S: HasStateApi> {
    admin: Address,
    paused: bool,
    tier_bases: collections::BTreeMap<Threshold, TierBaseState>,
    stakes: StateMap<Address, StakeState<S>, S>,
}

#[derive(Debug, Serialize, SchemaType)]
struct TierBaseParams {
    threshold: Threshold,
    max_alloc: u64,
    rate: u64,
}

type UpdateTierBaseParams = Vec<TierBaseParams>;

#[derive(Debug, Serialize, SchemaType)]
struct StakeParams {
    amount: OvlAmount,
}

#[derive(Debug, Serialize, SchemaType)]
struct UnstakeParams {
    amount: OvlAmount,
}

#[derive(Debug, Serialize, SchemaType)]
struct TransferFromParams {
    from:   Address,
    to:     Receiver,
    amount: ContractTokenAmount,
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
    ovl_credit_amount: OvlCreditAmount
}

type ViewTierBasesResponse = Vec<(u8, ViewTierBaseParams)>;

#[derive(Debug, Serialize, SchemaType)]
struct ViewStakeResponse {
    amount: OvlAmount,
    start_at: Timestamp,
    claimable_ovl: OvlAmount,
    tier: u8,
    staked_ovl_credits: Vec<(ProjectAddress, OvlCreditAmount)>,
    ovl_credit_amount: OvlCreditAmount,
    available_ovl_credit_amount: OvlCreditAmount,
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
    InsufficientOvl,
    InsufficientOvlCredit,
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
    fn from(_cce: CallContractError<T>) -> Self { Self::InvokeContractError }
}

/// Mapping errors related to contract invocations to ContractError.
impl From<TransferError> for ContractError {
    fn from(_te: TransferError) -> Self { Self::InvokeTransferError }
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
        tier_bases: collections::BTreeMap<Threshold, TierBaseState>,
    ) -> Self {
        State {
            admin,
            paused: false,
            stakes: state_builder.new_map(),
            tier_bases,
        }
    }

    fn stake(
        &mut self,
        owner: &Address,
        amount: &ContractTokenAmount,
        start_at: &Timestamp,
        state_builder: &mut StateBuilder<S>,
    ) {
        let mut stake = self.stakes.entry(*owner).or_insert_with(|| StakeState::new(*start_at, state_builder));

        // TODO: claimable_ovlの計算
        stake.amount += *amount;
        stake.start_at = *start_at;

        let tier_bases = &self.tier_bases;
        let mut index = tier_bases.len();

        let mut staked_ovl_credit = 0u64;
        for (_project_address, ovl_credit) in stake.staked_ovl_credits.iter() {
            staked_ovl_credit += *ovl_credit;
        }
        for( threshold, tier_base ) in tier_bases.iter().rev() {
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
                break;
            }
            index -= 1;
        }

        if index == 0 {
            stake.tier = 0;
            stake.ovl_credit_amount = stake.amount.into();
            stake.available_ovl_credit_amount = stake.ovl_credit_amount - staked_ovl_credit;
        }
    }

    fn unstake(
        &mut self,
        owner: &Address,
        amount: &ContractTokenAmount,
        start_at: &Timestamp,
        state_builder: &mut StateBuilder<S>,
    ) -> ContractResult<()> {
        let mut stake = self.stakes.entry(*owner).or_insert_with(|| StakeState::new(*start_at, state_builder));

        // TODO: claimable_ovlの計算
        let curr_amount = u64::from(stake.amount);

        ensure!(curr_amount as i128 - u64::from(*amount) as i128 >= 0, ContractError::InsufficientOvl);

        let after_amount = curr_amount - u64::from(*amount);
        let tier_bases = &self.tier_bases;
        let mut index = tier_bases.len();

        let mut staked_ovl_credit = 0u64;
        for (_project_address, ovl_credit) in stake.staked_ovl_credits.iter() {
            staked_ovl_credit += *ovl_credit;
        }
        for( threshold, tier_base ) in tier_bases.iter().rev() {
            if *threshold <= after_amount {
                // Calculate as a percentage.
                let calculated_ovl_credit: u64 = after_amount * tier_base.rate;
                // Convert to a string and remove the last two digits.
                let mut ovl_credit_str = u64::from(calculated_ovl_credit).to_string();
                ovl_credit_str.pop();
                ovl_credit_str.pop();

                let ovl_credit_amount = ovl_credit_str.parse::<u64>().unwrap();

                ensure!(ovl_credit_amount as i128 - staked_ovl_credit as i128 >= 0, ContractError::InsufficientOvlCredit);
                stake.amount = TokenAmountU64::from(after_amount);
                stake.start_at = *start_at;
                stake.tier = index as u8;
                stake.ovl_credit_amount = ovl_credit_amount;
                stake.available_ovl_credit_amount = ovl_credit_amount - staked_ovl_credit;
                break;
            }
            index -= 1;
        }

        if index == 0 {
            ensure!(stake.available_ovl_credit_amount as i128 - staked_ovl_credit as i128 >= 0, ContractError::InsufficientOvlCredit);
            stake.amount = TokenAmountU64::from(after_amount);
            stake.start_at = *start_at;
            stake.tier = 0;
            stake.ovl_credit_amount = stake.amount.into();
            stake.available_ovl_credit_amount = stake.ovl_credit_amount - staked_ovl_credit;
        }

        Ok(())
    }

    fn get_staked_ovl_credits(&self, owner: &Address) -> Vec<(ProjectAddress, OvlCreditAmount)> {
        let stake_state = match self.stakes.get(owner) {
            Some(v) => v,
            None => return vec![]
        };
        let mut staked_ovl_credits: Vec<(ProjectAddress, OvlCreditAmount)> = Vec::new();
        for (project_address, ovl_credit) in stake_state.staked_ovl_credits.iter() {
            staked_ovl_credits.push((*project_address, *ovl_credit));
        }
        return staked_ovl_credits;
    }
}

/// Init function that creates a new contract.
#[init(
    contract = "ovl_staking",
    // enable_logger,
    parameter = "UpdateTierBaseParams",
    // event = "Event"
)]
fn contract_init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
    // logger: &mut impl HasLogger,
) -> InitResult<State<S>> {
    let params: UpdateTierBaseParams = ctx.parameter_cursor().get()?;

    let mut tier_bases = collections::BTreeMap::new();

    for TierBaseParams {
        threshold,
        max_alloc,
        rate
    } in params
    {
        tier_bases.entry(threshold).or_insert_with(|| TierBaseState {
            max_alloc: max_alloc,
            rate: rate,
            ovl_credit_amount: 0u64.into(),
        });
    }

    let invoker = Address::Account(ctx.init_origin());
    let state = State::new(
        state_builder,
        invoker,
        tier_bases,
    );

    Ok(state)
}

#[receive(
    contract = "ovl_staking",
    name = "updateTierBase",
    parameter = "UpdateTierBaseParams",
    // enable_logger,
    mutable
)]
fn contract_update_tier_base<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    // logger: &mut impl HasLogger,
) -> ContractResult<()> {
    ensure!(!host.state().paused, ContractError::ContractPaused);
    ensure_eq!(ctx.sender(), host.state().admin, ContractError::Unauthorized);

    let params: UpdateTierBaseParams = ctx.parameter_cursor().get()?;

    let state = host.state_mut();

    state.tier_bases = collections::BTreeMap::new();

    for TierBaseParams {
        threshold,
        max_alloc,
        rate
    } in params
    {
        state.tier_bases.entry(threshold).or_insert_with(|| TierBaseState {
            max_alloc: max_alloc,
            rate: rate,
            ovl_credit_amount: 0u64.into(),
        });
    }

    // TODO: 再計算

    Ok(())
}

#[receive(
    contract = "ovl_staking",
    name = "stake",
    parameter = "StakeParams",
    error = "ContractError",
    mutable
)]
fn contract_stake<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(!host.state().paused, ContractError::ContractPaused);

    let params: StakeParams = ctx.parameter_cursor().get()?;
    let sender = ctx.sender();

    // transferできたらstakeの実行をする
    let transfer = Transfer {
        token_id: TOKEN_ID_OVL,
        amount: params.amount,
        from: sender,
        to: Receiver::Contract(ctx.self_address(),
                OwnedEntrypointName::new_unchecked("receiveTransfer".to_string())),
        data: AdditionalData::empty(),
    };

    host.invoke_contract(
        &ContractAddress{index: 2185, subindex: 0},
        &TransferParams::from(vec![transfer]),
        EntrypointName::new_unchecked("transferFrom"),
        Amount::zero()
    )?;

    let (state, builder) = host.state_and_builder();
    state.stake(&sender, &params.amount, &ctx.metadata().slot_time(), builder);

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

    let sender = match ctx.sender() {
        Address::Account(sender) => sender,
        Address::Contract(_) => return Err(ContractError::Unauthorized),
    };

    let (state, builder) = host.state_and_builder();
    state.unstake(&ctx.sender(), &params.amount, &ctx.metadata().slot_time(), builder)?;

    let transfer = Transfer {
        token_id: TOKEN_ID_OVL,
        amount: params.amount,
        from: Address::Contract(ctx.self_address()),
        to: Receiver::Account(sender),
        data: AdditionalData::empty(),
    };

    host.invoke_contract(
        &ContractAddress{index: 2185, subindex: 0},
        &TransferParams::from(vec![transfer]),
        EntrypointName::new_unchecked("transfer"),
        Amount::zero()
    )?;

    Ok(())
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
        start_at: stake_state.start_at,
        claimable_ovl: stake_state.claimable_ovl,
        tier: stake_state.tier,
        staked_ovl_credits: staked_ovl_credits,
        ovl_credit_amount: stake_state.ovl_credit_amount,
        available_ovl_credit_amount: stake_state.available_ovl_credit_amount,
    };
    Ok(state)
}

#[receive(
    contract = "ovl_staking",
    name = "ViewTierBases",
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
                ovl_credit_amount: tier_base.ovl_credit_amount
            }
        ));
        index += 1;
    }
    Ok(tier_bases_response)
}

type OnReceivingCis2Parameter = OnReceivingCis2Params<ContractTokenId, ContractTokenAmount>;

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
    // const NEW_ADMIN_ACCOUNT: AccountAddress = AccountAddress([3u8; 32]);
    // const NEW_ADMIN_ADDRESS: Address = Address::Account(NEW_ADMIN_ACCOUNT);

    // The metadata url for the wCCD token.
    // const INITIAL_TOKEN_METADATA_URL: &str = "https://some.example/token/wccd";

    /// Test helper function which creates a contract state where ADDRESS_0 owns
    /// 400 tokens.
    fn initial_state<S: HasStateApi>(state_builder: &mut StateBuilder<S>) -> State<S> {
        let tier1 = TierBaseState {
            max_alloc: 2,
            rate: 105,
            ovl_credit_amount: 0u64.into(),
        };

        let tier2 = TierBaseState {
            max_alloc: 4,
            rate: 110,
            ovl_credit_amount: 0u64.into(),
        };

        let tier3 = TierBaseState {
            max_alloc: 8,
            rate: 115,
            ovl_credit_amount: 0u64.into(),
        };

        let tier4 = TierBaseState {
            max_alloc: 16,
            rate: 120,
            ovl_credit_amount: 0u64.into(),
        };

        let tier5 = TierBaseState {
            max_alloc: 32,
            rate: 125,
            ovl_credit_amount: 0u64.into(),
        };

        let tier_bases = collections::BTreeMap::from([
            (1000u64, tier1),
            (2000u64,tier2),
            (3000u64, tier3),
            (4000u64, tier4),
            (5000u64, tier5),
        ]);
        let state = State::new(state_builder, ADMIN_ADDRESS, tier_bases);
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

        let params = vec![tier1,tier2, tier3, tier4, tier5];
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let mut builder = TestStateBuilder::new();

        let result = contract_init(&ctx, &mut builder);
        // Check the result.
        result.expect_report("Contract initialization failed");
    }

    #[concordium_test]
    fn test_stake() {
        // Set up the context
        let mut ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let self_address = ContractAddress::new(2249, 0);
        ctx.set_self_address(self_address);

        ctx.metadata_mut().set_slot_time(Timestamp::from_timestamp_millis(100));

        // Set up the parameter.
        let params = StakeParams {
            amount:   ContractTokenAmount::from(1000),
        };
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let _logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        let result: ContractResult<()> = contract_stake(&ctx, &mut host);
        println!("{:?}", result);

        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        let params2 = ViewStakeParams {
            owner:   ADMIN_ADDRESS
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

        let self_address = ContractAddress::new(2249, 0);
        ctx.set_self_address(self_address);

        ctx.metadata_mut().set_slot_time(Timestamp::from_timestamp_millis(100));

        // Set up the parameter.
        let params = StakeParams {
            amount:   ContractTokenAmount::from(5000),
        };
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let _logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        let result: ContractResult<()> = contract_stake(&ctx, &mut host);
        println!("{:?}", result);

        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        let params2 = ViewStakeParams {
            owner:   ADMIN_ADDRESS
        };
        let parameter_bytes2 = to_bytes(&params2);
        ctx.set_parameter(&parameter_bytes2);
        let result2: ContractResult<ViewStakeResponse> = contract_view_stake(&ctx, &mut host);
        println!("{:?}", result2);

        let params3 = UnstakeParams {
            amount:   ContractTokenAmount::from(1000),
        };
        let parameter_bytes3 = to_bytes(&params3);
        ctx.set_parameter(&parameter_bytes3);
        let result3: ContractResult<()> = contract_unstake(&ctx, &mut host);
        println!("{:?}", result3);

        let parameter_bytes2 = to_bytes(&params2);
        ctx.set_parameter(&parameter_bytes2);
        let result4: ContractResult<ViewStakeResponse> = contract_view_stake(&ctx, &mut host);
        println!("{:?}", result4);

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

        let params = vec![tier1,tier2, tier3, tier4, tier5];
        let parameter_bytes = to_bytes(&params);
        ctx.set_parameter(&parameter_bytes);

        let _logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let state = initial_state(&mut state_builder);
        let mut host = TestHost::new(state, state_builder);

        let result: ContractResult<()> = contract_update_tier_base(&ctx, &mut host);
        println!("{:?}", result);
        // Check the result.
        claim!(result.is_ok(), "Results in rejection");

        ctx = TestReceiveContext::empty();
        ctx.set_sender(ADMIN_ADDRESS);

        let result2: ContractResult<ViewTierBasesResponse> = contract_view_tier_bases(&ctx, &mut host);
        println!("{:?}", result2);
    }
}
