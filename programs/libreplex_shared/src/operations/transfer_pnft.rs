use mpl_token_metadata::{
    accounts::Metadata, instructions::TransferV1Builder, types::TokenStandard,
};

use mpl_token_metadata::types::ProgrammableConfig;

// use mpl_token_metadata::instructions:, InstructionBuilder};

use anchor_lang::prelude::*;
use solana_program::program::{invoke, invoke_signed};

use crate::{sysvar_instructions_program, SharedError};

pub mod auth_rules_program {
    use super::*;
    declare_id!("auth9SigNpDKz4sJJ1DfCTuZrZNSAgh9sFD3rboVmgg");
}
pub struct MetaplexProgrammableTransferExtraAccounts<'f> {
    pub metadata: Option<&'f AccountInfo<'f>>,
    pub edition: Option<&'f AccountInfo<'f>>,
    pub token_record_source: Option<&'f AccountInfo<'f>>,
    pub token_record_target: Option<&'f AccountInfo<'f>>,
    pub sysvar_instructions: Option<&'f AccountInfo<'f>>,
    pub auth_rules_program: Option<&'f AccountInfo<'f>>,
    pub auth_rules: Option<&'f AccountInfo<'f>>,
}

impl <'g>MetaplexProgrammableTransferExtraAccounts::<'g> {
    pub fn new(
        remaining_accounts: &'g [AccountInfo<'g>],
        mint: &Pubkey,
        token_account_source: &Pubkey,
        token_account_target: &Pubkey,
    ) -> MetaplexProgrammableTransferExtraAccounts<'g> {
        let mut extra_accounts = MetaplexProgrammableTransferExtraAccounts {
            metadata: None,
            edition: None,
            token_record_source: None,
            token_record_target: None,
            sysvar_instructions: None,
            auth_rules_program: None,
            auth_rules: None,
        };
        let metadata_address = Pubkey::find_program_address(
            &[
                b"metadata",
                &mpl_token_metadata::ID.as_ref(),
                &mint.key().as_ref(),
            ],
            &mpl_token_metadata::ID,
        )
        .0;
    
        let metadata_account_info = remaining_accounts
            .iter()
            .find(|x| x.key().eq(&metadata_address));
    
        match metadata_account_info {
            Some(x) => {
                let metadata_obj_option = Metadata::try_from(x).ok();
    
                if let Some(metadata_obj) = metadata_obj_option {
                    match metadata_obj.token_standard {
                        Some(TokenStandard::ProgrammableNonFungible) => {
                            let programmable_config = &metadata_obj.programmable_config.unwrap();
    
                            let edition_address = Pubkey::find_program_address(
                                &[
                                    b"metadata",
                                    &mpl_token_metadata::ID.as_ref(),
                                    &mint.key().as_ref(),
                                    b"edition",
                                ],
                                &mpl_token_metadata::ID,
                            )
                            .0;
    
                            let token_record_source = Pubkey::find_program_address(
                                &[
                                    b"metadata",
                                    &mpl_token_metadata::ID.as_ref(),
                                    &mint.key().as_ref(),
                                    b"token_record",
                                    token_account_source.as_ref(),
                                ],
                                &mpl_token_metadata::ID,
                            )
                            .0;
    
                            let token_record_target = Pubkey::find_program_address(
                                &[
                                    b"metadata",
                                    &mpl_token_metadata::ID.as_ref(),
                                    &mint.key().as_ref(),
                                    b"token_record",
                                    token_account_target.as_ref(),
                                ],
                                &mpl_token_metadata::ID,
                            )
                            .0;
    
                            match programmable_config {
                                ProgrammableConfig::V1 { rule_set } => {
                                    let edition = remaining_accounts
                                        .iter()
                                        .find(|x| x.key().eq(&edition_address))
                                        .as_ref()
                                        .cloned();
    
                                    let token_record_source = remaining_accounts
                                        .iter()
                                        .find(|x| x.key().eq(&token_record_source))
                                        .as_ref()
                                        .cloned();
    
                                    let token_record_target = remaining_accounts
                                        .iter()
                                        .find(|x| x.key().eq(&token_record_target))
                                        .as_ref()
                                        .cloned();
    
                                    let sysvar_instructions = remaining_accounts
                                        .iter()
                                        .find(|x| x.key().eq(&sysvar_instructions_program::ID))
                                        .as_ref()
                                        .cloned();
    
                                    let auth_rules_program = remaining_accounts
                                        .iter()
                                        .find(|x| x.key().eq(&auth_rules_program::ID))
                                        .as_ref()
                                        .cloned();
    
                                    let auth_rules = remaining_accounts
                                        .iter()
                                        .find(|x| x.key().eq(&rule_set.unwrap()))
                                        .as_ref()
                                        .cloned();
    
                                    extra_accounts.metadata = Some(&x);
                                    extra_accounts.edition = edition;
                                    extra_accounts.token_record_source = token_record_source;
                                    extra_accounts.token_record_target = token_record_target;
                                    extra_accounts.sysvar_instructions = sysvar_instructions;
                                    extra_accounts.auth_rules_program = auth_rules_program;
                                    extra_accounts.auth_rules = auth_rules;
                                }
                            }
                        }
                        _ => {
                            // not a pnft. return the classic accounts without token records
                            extra_accounts.metadata = Some(&x);
                        }
                    }
                }
            }
            None => {}
        }
        extra_accounts
    }
}

pub fn transfer_pnft<'info>(
    token_program: &AccountInfo<'info>,
    source_token_account: &AccountInfo<'info>,
    target_token_account: &AccountInfo<'info>,
    source_wallet: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    target_wallet: &AccountInfo<'info>,
    associated_token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    authority_seeds: Option<&[&[&[u8]]]>,
    payer: &AccountInfo<'info>,
    extra_accounts: &MetaplexProgrammableTransferExtraAccounts<'info>,
) -> Result<()> {
    // move the token from source token account to the target token account

    let expected_token_account = anchor_spl::associated_token::get_associated_token_address(
        &target_wallet.key(),
        &mint.key(),
    );

    if expected_token_account != target_token_account.key() {
        return Err(SharedError::InvalidTokenAccount.into());
    }

    if target_token_account.data_is_empty() {
        msg!("Create token account");
        anchor_spl::associated_token::create(CpiContext::new(
            associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: payer.to_account_info(),
                associated_token: target_token_account.to_account_info(),
                authority: target_wallet.to_account_info(),
                mint: mint.to_account_info(),
                system_program: system_program.to_account_info(),
                token_program: token_program.to_account_info(),
            },
        ))?;
    }

    let mut transfer_builder = TransferV1Builder::new();

    msg!("Transfer");
    transfer_builder
        .token(source_token_account.key()) // 1
        .token_owner(source_wallet.key()) // 2
        .destination_token(target_token_account.key()) //3
        .destination_owner(target_wallet.key()) //4
        .mint(mint.key()) //5
        .metadata(extra_accounts.metadata.unwrap().key()) //6
        .edition(Some(extra_accounts.edition.unwrap().key())) //7
        .token_record(Some(extra_accounts.token_record_source.unwrap().key())) //8
        .destination_token_record(Some(extra_accounts.token_record_target.unwrap().key())) //9
        .authority(source_wallet.key()) //10
        .payer(payer.key()) //11
        .system_program(system_program.key()) //12
        .sysvar_instructions(extra_accounts.sysvar_instructions.unwrap().key()) //13
        .spl_token_program(token_program.key()) //14
        .spl_ata_program(associated_token_program.key()) //15
        .authorization_rules_program(Some(extra_accounts.auth_rules_program.unwrap().key())) //16
        .authorization_rules(Some(extra_accounts.auth_rules.unwrap().key())); //17

    let transfer_infos = [
        source_token_account.to_account_info(),
        source_wallet.to_account_info(),
        target_token_account.to_account_info(),
        target_wallet.to_account_info(),
        mint.to_account_info(),
        extra_accounts.metadata.unwrap().to_account_info(), // fix ! token record
        extra_accounts.edition.unwrap().to_account_info(),
        extra_accounts
            .token_record_source
            .unwrap()
            .to_account_info(),
        extra_accounts
            .token_record_target
            .unwrap()
            .to_account_info(),
        source_wallet.to_account_info(),
        payer.to_account_info(),
        system_program.to_account_info(),
        extra_accounts
            .sysvar_instructions
            .unwrap()
            .to_account_info(),
        token_program.to_account_info(),
        associated_token_program.to_account_info(),
        extra_accounts.auth_rules_program.unwrap().to_account_info(),
        extra_accounts.auth_rules.unwrap().to_account_info(),
    ];

    let ix = transfer_builder.amount(1).instruction();

    match authority_seeds {
        Some(x) => {
            msg!("invoke_signer");
            invoke_signed(&ix, &transfer_infos, x)?;
        }
        None => {
            msg!("invoke");
            invoke(&ix, &transfer_infos)?;
        }
    }

    Ok(())
}

// pub fn extract_metaplex_metadata_accounts_from_remaining_accounts<'f, 'g: 'f>(
//     remaining_accounts: &'g [AccountInfo<'g>],
//     mint: &Pubkey,
//     token_account_source: &Pubkey,
//     token_account_target: &Pubkey,
// ) -> MetaplexProgrammableTransferExtraAccounts<'g> {
    
// }
