use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{borsh::try_from_slice_unchecked, program_error::ProgramError};

#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
/// All custom program instructions
pub enum ProgramInstruction {
    InitializeAccount,
    WalletNew,
    WalletTransferSpl,
    WalletTransferLamports,
    MintToAccount(String, String),
    TransferBetweenAccounts(String),
    BurnFromAccount(String),
    MintToAccountWithFee(String, String),
    TransferBetweenAccountsWithFee(String),
    BurnFromAccountWithFee(String),
}

impl ProgramInstruction {
    /// Unpack inbound buffer to associated Instruction
    /// The expected format for input is a Borsh serialized vector
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let payload = try_from_slice_unchecked::<ProgramInstruction>(input).unwrap();
        match payload {
            ProgramInstruction::InitializeAccount => Ok(payload),
            ProgramInstruction::WalletNew => Ok(payload),
            ProgramInstruction::WalletTransferSpl => Ok(payload),
            ProgramInstruction::WalletTransferLamports => Ok(payload),
            ProgramInstruction::MintToAccount(_, _) => Ok(payload),
            ProgramInstruction::TransferBetweenAccounts(_) => Ok(payload),
            ProgramInstruction::BurnFromAccount(_) => Ok(payload),
            ProgramInstruction::MintToAccountWithFee(_, _) => Ok(payload),
            ProgramInstruction::TransferBetweenAccountsWithFee(_) => Ok(payload),
            ProgramInstruction::BurnFromAccountWithFee(_) => Ok(payload),
        }
    }
}
