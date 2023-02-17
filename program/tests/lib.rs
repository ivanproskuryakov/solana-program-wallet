use sol_template_shared::{unpack_from_slice, ACCOUNT_STATE_SPACE};
use app_wallet::{instruction::ProgramInstruction, processor::process};
use solana_program::hash::Hash;
use solana_program_test::*;

use {
    solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
    solana_program::{
        program_pack::Pack,
        rent::Rent,
        system_instruction,
    },
    solana_program_test::{processor, tokio, ProgramTest},
    app_wallet::processor::transfer,
    spl_token::state::{Account as TokenAccount, Mint},
    std::str::FromStr,
};

/// Sets up the Program test and initializes 'n' program_accounts
async fn setup(program_id: &Pubkey, program_accounts: &[Pubkey]) -> (BanksClient, Keypair, Hash) {
    let mut program_test = ProgramTest::new(
        "app_wallet", // Run the BPF version with `cargo test-bpf`
        *program_id,
        processor!(process), // Run the native version with `cargo test`
    );
    for account in program_accounts {
        program_test.add_account(
            *account,
            Account {
                lamports: 5,
                data: vec![0_u8; ACCOUNT_STATE_SPACE],
                owner: *program_id,
                ..Account::default()
            },
        );
    }
    program_test.start().await
}

/// Submit transaction with relevant instruction data
#[allow(clippy::ptr_arg)]
async fn submit_txn(
    program_id: &Pubkey,
    instruction_data: ProgramInstruction,
    accounts: &[AccountMeta],
    payer: &dyn Signer,
    recent_blockhash: Hash,
    banks_client: &mut BanksClient,
) -> Result<(), BanksClientError> {
    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_borsh(
            *program_id,
            &instruction_data,
            accounts.to_vec(),
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await
}

#[tokio::test]
async fn test_transfer() {
    // Setup some pubkeys for the accounts
    let program_id = Pubkey::from_str("TransferTokens11111111111111111111111111111").unwrap();
    let source = Keypair::new();
    let mint = Keypair::new();
    let destination = Keypair::new();
    let (authority_pubkey, _) = Pubkey::find_program_address(&[b"authority"], &program_id);

    // Add the program to the test framework
    let program_test = ProgramTest::new(
        "transfer",
        program_id,
        processor!(transfer),
    );
    let amount = 10_000;
    let decimals = 9;
    let rent = Rent::default();

    // Start the program test
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Setup the mint, used in `spl_token::instruction::transfer_checked`
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                &payer.pubkey(),
                None,
                decimals,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Setup the source account, owned by the program-derived address
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &source.pubkey(),
                rent.minimum_balance(TokenAccount::LEN),
                TokenAccount::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &source.pubkey(),
                &mint.pubkey(),
                &authority_pubkey,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &source],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Setup the destination account, used to receive tokens from the account
    // owned by the program-derived address
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &destination.pubkey(),
                rent.minimum_balance(TokenAccount::LEN),
                TokenAccount::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &destination.pubkey(),
                &mint.pubkey(),
                &payer.pubkey(),
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
        &[&payer, &destination],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Mint some tokens to the PDA account
    let transaction = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            &mint.pubkey(),
            &source.pubkey(),
            &payer.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Create an instruction following the account order expected by the program
    let transaction = Transaction::new_signed_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &(),
            vec![
                AccountMeta::new(source.pubkey(), false),
                AccountMeta::new_readonly(mint.pubkey(), false),
                AccountMeta::new(destination.pubkey(), false),
                AccountMeta::new_readonly(authority_pubkey, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // See that the transaction processes successfully
    banks_client.process_transaction(transaction).await.unwrap();

    // Check that the destination account now has `amount` tokens
    let account = banks_client
        .get_account(destination.pubkey())
        .await
        .unwrap()
        .unwrap();
    let token_account = TokenAccount::unpack(&account.data).unwrap();
    assert_eq!(token_account.amount, amount);
}

#[tokio::test]
/// Wallet new test
async fn test_wallet_new() {
    let program_id = Pubkey::new_unique();
    let account_pubkey = Pubkey::new_unique();

    println!("{:?}", program_id);
    println!("{:?}", account_pubkey);

    // Setup runtime testing and accounts
    let (
        mut banks_client, 
        payer, 
        recent_blockhash
    ) = setup(&program_id, &[account_pubkey]).await;
    
    let _result = submit_txn(
        &program_id,
        ProgramInstruction::InitializeAccount,
        &[AccountMeta::new(account_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;

    let result = submit_txn(
        &program_id,
        ProgramInstruction::WalletNew,
        &[AccountMeta::new(account_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;

    // Check the data
    let (is_initialized, btree_map) = match banks_client.get_account(account_pubkey).await.unwrap()
    {
        Some(account) => unpack_from_slice(&account.data).unwrap(),
        None => panic!(),
    };

    println!(">>>>> {:?}", is_initialized);
    println!(">>>>> {:?}", btree_map);
    println!(">>>>> {:?}", result.is_ok());
}


#[tokio::test]
/// Initialization test
async fn test_initialize_pass() {
    println!(" -------------------------------------------- 1");

    let program_id = Pubkey::new_unique();
    let account_pubkey = Pubkey::new_unique();

    // Setup runtime testing and accounts
    let (mut banks_client, payer, recent_blockhash) = setup(&program_id, &[account_pubkey]).await;

    // Verify account is not yet initialized
    let (is_initialized, _btree_map) = match banks_client.get_account(account_pubkey).await.unwrap()
    {
        Some(account) => unpack_from_slice(&account.data).unwrap(),
        None => panic!(),
    };
    assert!(!is_initialized);
    let result = submit_txn(
        &program_id,
        ProgramInstruction::InitializeAccount,
        &[AccountMeta::new(account_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());
}

#[tokio::test]
/// Mint test
async fn test_mint_pass() {
    println!(" -------------------------------------------- 2");

    let program_id = Pubkey::new_unique();
    let account_pubkey = Pubkey::new_unique();

    // Setup runtime testing and accounts
    let (mut banks_client, payer, recent_blockhash) = setup(&program_id, &[account_pubkey]).await;

    let result = submit_txn(
        &program_id,
        ProgramInstruction::InitializeAccount,
        &[AccountMeta::new(account_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());

    // Do mint
    let mint_key = String::from("test_key_1");
    let mint_value = String::from("value for test_key_1");

    let result = submit_txn(
        &program_id,
        ProgramInstruction::MintToAccount(mint_key.clone(), mint_value.clone()),
        &[AccountMeta::new(account_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());
    // Check the data
    let (is_initialized, btree_map) = match banks_client.get_account(account_pubkey).await.unwrap()
    {
        Some(account) => unpack_from_slice(&account.data).unwrap(),
        None => panic!(),
    };
    assert!(is_initialized);
    assert!(btree_map.contains_key(&mint_key));
    assert_eq!(btree_map.get(&mint_key).unwrap(), &mint_value);
}

#[tokio::test]
/// Transfer test
async fn test_mint_transfer_pass() {
    println!(" -------------------------------------------- 3");

    let program_id = Pubkey::new_unique();
    let start_pubkey = Pubkey::new_unique();
    let target_pubkey = Pubkey::new_unique();

    // Setup runtime testing and accounts
    let (mut banks_client, payer, recent_blockhash) =
        setup(&program_id, &[start_pubkey, target_pubkey]).await;

    for acc_key in [&start_pubkey, &target_pubkey] {
        let result = submit_txn(
            &program_id,
            ProgramInstruction::InitializeAccount,
            &[AccountMeta::new(*acc_key, false)],
            &payer,
            recent_blockhash,
            &mut banks_client,
        )
        .await;
        assert!(result.is_ok());
    }

    let mint_key = String::from("test_key_1");
    let mint_value = String::from("value for test_key_1");

    // Do mint
    let result = submit_txn(
        &program_id,
        ProgramInstruction::MintToAccount(mint_key.clone(), mint_value.clone()),
        &[AccountMeta::new(start_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());

    // Do transfer
    let result = submit_txn(
        &program_id,
        ProgramInstruction::TransferBetweenAccounts(mint_key.clone()),
        &[
            AccountMeta::new(start_pubkey, false),
            AccountMeta::new(target_pubkey, false),
        ],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());

    let (is_initialized, btree_map) = match banks_client.get_account(start_pubkey).await.unwrap() {
        Some(account) => unpack_from_slice(&account.data).unwrap(),
        None => panic!(),
    };
    assert!(is_initialized);
    assert!(!btree_map.contains_key(&mint_key));

    let (is_initialized, btree_map) = match banks_client.get_account(target_pubkey).await.unwrap() {
        Some(account) => unpack_from_slice(&account.data).unwrap(),
        None => panic!(),
    };
    assert!(is_initialized);
    assert!(btree_map.contains_key(&mint_key));
    assert_eq!(btree_map.get(&mint_key).unwrap(), &mint_value);
}

#[tokio::test]
/// Burn test
async fn test_mint_transfer_burn_pass() {
    println!(" -------------------------------------------- 4");

    let program_id = Pubkey::new_unique();
    let start_pubkey = Pubkey::new_unique();
    let target_pubkey = Pubkey::new_unique();

    // Setup runtime testing and accounts
    let (mut banks_client, payer, recent_blockhash) =
        setup(&program_id, &[start_pubkey, target_pubkey]).await;
    for acc_key in [&start_pubkey, &target_pubkey] {
        let result = submit_txn(
            &program_id,
            ProgramInstruction::InitializeAccount,
            &[AccountMeta::new(*acc_key, false)],
            &payer,
            recent_blockhash,
            &mut banks_client,
        )
        .await;
        assert!(result.is_ok());
    }

    // Do mint
    let mint_key = String::from("test_key_1");
    let mint_value = String::from("value for test_key_1");

    // Do mint
    let result = submit_txn(
        &program_id,
        ProgramInstruction::MintToAccount(mint_key.clone(), mint_value.clone()),
        &[AccountMeta::new(start_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());

    // Do transfer
    let result = submit_txn(
        &program_id,
        ProgramInstruction::TransferBetweenAccounts(mint_key.clone()),
        &[
            AccountMeta::new(start_pubkey, false),
            AccountMeta::new(target_pubkey, false),
        ],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());

    // Do burn
    let result = submit_txn(
        &program_id,
        ProgramInstruction::BurnFromAccount(mint_key.clone()),
        &[AccountMeta::new(target_pubkey, false)],
        &payer,
        recent_blockhash,
        &mut banks_client,
    )
    .await;
    assert!(result.is_ok());
}
