use crate::{
    account_state::ProgramAccountState, error::SampleError, instruction::ProgramInstruction,
};
use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack},
        pubkey::Pubkey,
    },
    spl_token::{
        instruction::transfer_checked,
        state::{Account, Mint},
    },
};

/// Service fees debited from participating accounts and
/// credited to a 'service' account
enum SampleServiceFees {
    Minting = 10,
    Transfering = 30,
    Burning = 15,
}

/// Checks each tracking account to confirm it is owned by our program
/// This function assumes that the program account is always the last
/// in the array
fn check_account_ownership(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    // Accounts must be owned by the program.
    for account in accounts.iter().take(accounts.len() - 1) {
        if account.owner != program_id {
            msg!(
                "Fail: The tracking account owner is {} and it should be {}.",
                account.owner,
                program_id
            );
            return Err(ProgramError::IncorrectProgramId);
        }
    }
    Ok(())
}

/// Extracts the service fee from the users program account and
/// credits the service account
fn charge_service_fee(
    program_account: &AccountInfo,
    service_account: &AccountInfo,
    amount: u64,
) -> ProgramResult {
    // If tracking can not afford transaction fee
    if **program_account.try_borrow_lamports()? < amount {
        return Err(SampleError::InsufficientFundsForTransaction.into());
    }
    // Debit user and credit service
    **program_account.try_borrow_mut_lamports()? -= amount;
    **service_account.try_borrow_mut_lamports()? += amount;
    Ok(())
}

fn wallet_new(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("wallet_new");

    let key = "a2".to_string();
    let value = "b2".to_string();
    
    let account_info_iter = &mut accounts.iter();
    let program_account = next_account_info(account_info_iter)?;
    let mut account_data = program_account.data.borrow_mut();

    let mut account_state = ProgramAccountState::unpack(&account_data)?;
    account_state.add(key, value)?;
    ProgramAccountState::pack(account_state, &mut account_data)?;

    Ok(())
}

pub fn wallet_transfer_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Create an iterator to safely reference accounts in the slice
    let account_info_iter = &mut accounts.iter();

    // As part of the program specification the instruction gives:
    let source_info = next_account_info(account_info_iter)?; // 1.
    let mint_info = next_account_info(account_info_iter)?; // 2.
    let destination_info = next_account_info(account_info_iter)?; // 3.
    let authority_info = next_account_info(account_info_iter)?; // 4.
    let token_program_info = next_account_info(account_info_iter)?; // 5.

    // In order to transfer from the source account, owned by the program-derived
    // address, we must have the correct address and seeds.
    let (expected_authority, bump_seed) = Pubkey::find_program_address(&[b"authority"], program_id);
    if expected_authority != *authority_info.key {
        return Err(ProgramError::InvalidSeeds);
    }

    // The program transfers everything out of its account, so extract that from
    // the account data.
    let source_account = Account::unpack(&source_info.try_borrow_data()?)?;
    let amount = source_account.amount;

    // The program uses `transfer_checked`, which requires the number of decimals
    // in the mint, so extract that from the account data too.
    let mint = Mint::unpack(&mint_info.try_borrow_data()?)?;
    let decimals = mint.decimals;

    // Invoke the transfer
    msg!("Attempting to transfer {} tokens", amount);
    invoke_signed(
        &transfer_checked(
            token_program_info.key,
            source_info.key,
            mint_info.key,
            destination_info.key,
            authority_info.key,
            &[], // no multisig allowed
            amount,
            decimals,
        )
        .unwrap(),
        &[
            source_info.clone(),
            mint_info.clone(),
            destination_info.clone(),
            authority_info.clone(),
            token_program_info.clone(), // not required, but better for clarity
        ],
        &[&[b"authority", &[bump_seed]]],
    )
}

pub fn wallet_transfer_lamports(
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;

    msg!("-------------- wallet_transfer_lamports");

    msg!("source_info {:?} {:?}", source_info.key, source_info.lamports);
    msg!("destination_info {:?} {:?}", destination_info.key, destination_info.lamports);

    **source_info.try_borrow_mut_lamports()? -= 5;
    **destination_info.try_borrow_mut_lamports()? += 5;

    Ok(())
}

/// Initialize the programs account, which is the first in accounts
fn initialize_account(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Initialize account");
    let account_info_iter = &mut accounts.iter();
    let program_account = next_account_info(account_info_iter)?;
    let mut account_data = program_account.data.borrow_mut();
    // Just using unpack will check to see if initialized and will
    // fail if not
    let mut account_state = ProgramAccountState::unpack_unchecked(&account_data)?;
    // Where this is a logic error in trying to initialize the same
    // account more than once
    if account_state.is_initialized() {
        return Err(SampleError::AlreadyInitializedState.into());
    } else {
        account_state.set_initialized();
    }

    let key = "a1".to_string();
    let value = "b1".to_string();
    account_state.add(key, value)?;

    ProgramAccountState::pack(account_state, &mut account_data).unwrap();
    Ok(())
}

/// Mint a key/pair to the programs account, which is the first in accounts
fn mint_keypair_to_account(accounts: &[AccountInfo], key: String, value: String) -> ProgramResult {
    msg!("Mint to account");
    let account_info_iter = &mut accounts.iter();
    let program_account = next_account_info(account_info_iter)?;
    let mut account_data = program_account.data.borrow_mut();
    // Unpacking an uninitialized account state will fail
    let mut account_state = ProgramAccountState::unpack(&account_data)?;
    account_state.add(key, value)?;
    ProgramAccountState::pack(account_state, &mut account_data)?;
    Ok(())
}
/// Mint a key/value pair extracting a service fee for the effort
fn mint_keypair_to_account_with_fee(
    accounts: &[AccountInfo],
    key: String,
    value: String,
) -> ProgramResult {
    // Charge for service
    let account_info_iter = &mut accounts.iter();
    let program_account = next_account_info(account_info_iter)?;
    let service_account = next_account_info(account_info_iter)?;
    charge_service_fee(
        program_account,
        service_account,
        SampleServiceFees::Minting as u64,
    )?;
    // Invoke the actual mint
    mint_keypair_to_account(accounts, key, value)?;
    Ok(())
}
/// Transfer a key/pair from one program account to another
/// "from" account is first and "to" account is second  in accounts
fn transfer_keypair_to_account(accounts: &[AccountInfo], key: String) -> ProgramResult {
    msg!("Transfer from account");
    let account_info_iter = &mut accounts.iter();
    // Transfer from this account
    let from_program_account = next_account_info(account_info_iter)?;
    let mut from_account_data = from_program_account.data.borrow_mut();
    let mut from_account_state = ProgramAccountState::unpack(&from_account_data)?;
    // To this account
    let to_program_account = next_account_info(account_info_iter)?;
    let mut to_account_data = to_program_account.data.borrow_mut();
    let mut to_account_state = ProgramAccountState::unpack(&to_account_data)?;
    // Transfer the goods
    match from_account_state.remove(&key) {
        Ok(value) => {
            to_account_state.add(key, value)?;
            ProgramAccountState::pack(from_account_state, &mut from_account_data)?;
            ProgramAccountState::pack(to_account_state, &mut to_account_data)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
/// Transfer key/value pair extracting a service fee for the effort
fn transfer_keypair_to_account_with_fee(accounts: &[AccountInfo], key: String) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let from_account = next_account_info(account_info_iter)?;
    let to_account = next_account_info(account_info_iter)?;
    let service_account = next_account_info(account_info_iter)?;

    // Cost to "from account"
    charge_service_fee(
        from_account,
        service_account,
        SampleServiceFees::Transfering as u64,
    )?;
    // Cost to "to account"
    charge_service_fee(
        to_account,
        service_account,
        SampleServiceFees::Minting as u64,
    )?;
    // Invoke the actual transfer
    transfer_keypair_to_account(accounts, key)?;
    Ok(())
}
/// Burn a key/pair from the programs account, which is the first in accounts
fn burn_keypair_from_account(accounts: &[AccountInfo], key: String) -> ProgramResult {
    msg!("Burn from account");
    let account_info_iter = &mut accounts.iter();
    let program_account = next_account_info(account_info_iter)?;
    let mut account_data = program_account.data.borrow_mut();
    let mut account_state = ProgramAccountState::unpack_unchecked(&account_data)?;
    match account_state.remove(&key) {
        Ok(_) => {
            ProgramAccountState::pack(account_state, &mut account_data)?;
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
/// Burn a key/pair extracting a service fee for the effort
fn burn_keypair_from_account_with_fee(accounts: &[AccountInfo], key: String) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let program_account = next_account_info(account_info_iter)?;
    let service_account = next_account_info(account_info_iter)?;
    // Charge for service
    charge_service_fee(
        program_account,
        service_account,
        SampleServiceFees::Burning as u64,
    )?;
    // Invoke the actual burn
    burn_keypair_from_account(accounts, key)?;
    Ok(())
}
/// Main processing entry point dispatches to specific
/// instruction handlers
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Received process request");
    
    // Check the account for program relationship
    if let Err(error) = check_account_ownership(program_id, accounts) {
        return Err(error);
    }

    // Unpack the inbound data, mapping instruction to appropriate structure
    match ProgramInstruction::unpack(instruction_data)? {
        ProgramInstruction::WalletNew => wallet_new(accounts),
        ProgramInstruction::WalletTransferSpl => wallet_transfer_spl(program_id, accounts, instruction_data),
        ProgramInstruction::WalletTransferLamports => wallet_transfer_lamports(accounts),
        ProgramInstruction::InitializeAccount => initialize_account(accounts),
        ProgramInstruction::MintToAccount(key, value) => {
            mint_keypair_to_account(accounts, key, value)
        }
        ProgramInstruction::TransferBetweenAccounts(key) => {
            transfer_keypair_to_account(accounts, key)
        }
        ProgramInstruction::BurnFromAccount(key) => burn_keypair_from_account(accounts, key),
        ProgramInstruction::MintToAccountWithFee(key, value) => {
            mint_keypair_to_account_with_fee(accounts, key, value)
        }
        ProgramInstruction::TransferBetweenAccountsWithFee(key) => {
            transfer_keypair_to_account_with_fee(accounts, key)
        }
        ProgramInstruction::BurnFromAccountWithFee(key) => {
            burn_keypair_from_account_with_fee(accounts, key)
        }
    }
}
