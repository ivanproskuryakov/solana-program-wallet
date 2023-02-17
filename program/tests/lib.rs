use app_wallet::{
    instruction::ProgramInstruction, 
    processor::process_instruction
};

use {
    solana_sdk::{
        account::Account,
        pubkey::Pubkey,
        signature::{Signer},
        transaction::Transaction,
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
    },
    solana_program_test::{processor, tokio, ProgramTest},
};

#[tokio::test]
async fn test_lamport_transfer() {
    let program_id = Pubkey::new_unique();
    let source_pubkey = Pubkey::new_unique();
    let destination_pubkey = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "app_wallet",
        program_id,
        processor!(process_instruction),
    );
    
    program_test.add_account(
        source_pubkey,
        Account {
            lamports: 5,
            owner: program_id, // Can only withdraw lamports from accounts owned by the program
            ..Account::default()
        },
    );
    program_test.add_account(
        destination_pubkey,
        Account {
            lamports: 890_875,
            ..Account::default()
        },
    );
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    println!("{:?}", source_pubkey);
    println!("{:?}", destination_pubkey);
    println!("{:?}", payer.pubkey());

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_borsh(
            program_id,
            &ProgramInstruction::WalletTransferLamports,
            vec![
                AccountMeta::new(source_pubkey, false),
                AccountMeta::new(destination_pubkey, false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}
