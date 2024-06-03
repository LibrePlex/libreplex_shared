use anchor_lang::prelude::*;

use mpl_token_metadata::{accounts::Metadata, types::TokenStandard};
use solana_program::
    program::{invoke, invoke_signed}
;
use transfer_pnft::MetaplexProgrammableTransferExtraAccounts;

use crate::{operations::transfer_pnft, SharedError};

pub fn transfer_generic_spl<'info>(
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
    decimals: u8,
    amount: u64,
    remaining_accounts: &'info [AccountInfo<'info>],
) -> Result<()> {
    msg!("{}", token_program.key());
    let expected_token_account =
        anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &target_wallet.key(),
            &mint.key(),
            &token_program.key(),
        );

    if expected_token_account != target_token_account.key() {
        msg!("{} {}", expected_token_account, target_token_account.key);
        return Err(SharedError::InvalidTokenAccount.into());
    }
    msg!("Creating token account");

    if target_token_account.data_is_empty() {
        // msg!("{}",payer.key() );
        anchor_spl::associated_token::create(CpiContext::new(
            associated_token_program.clone(),
            anchor_spl::associated_token::Create {
                payer: payer.clone(),
                associated_token: target_token_account.clone(),
                authority: target_wallet.clone(),
                mint: mint.clone(),
                system_program: system_program.clone(),
                token_program: token_program.clone(),
            },
        ))?;
    }

    let extra_accounts = MetaplexProgrammableTransferExtraAccounts::new(
        remaining_accounts,
        &mint.key,
        &source_token_account.key,
        &target_token_account.key,
    );

    if let Some(x) = &extra_accounts.metadata {
        // ok we may have a pnft. let's check
        let metadata_obj = Metadata::try_from(*x)?;

        if let Some(x) = metadata_obj.token_standard {
            if x == TokenStandard::ProgrammableNonFungible {
                transfer_pnft(
                    &token_program.to_account_info(),
                    &source_token_account.to_account_info(),
                    &target_token_account.to_account_info(),
                    &source_wallet.to_account_info(),
                    &mint.to_account_info(),
                    &target_wallet.to_account_info(),
                    &associated_token_program.to_account_info(),
                    &system_program.to_account_info(),
                    authority_seeds,
                    &payer.to_account_info(),
                    &extra_accounts,
                )?;
                // done - bail out
                return Ok(());
            }
        }
    }

    /// the regular (non-pnft) route

    let mut ix = spl_token_2022::instruction::transfer_checked(
        token_program.key,
        source_token_account.key,
        mint.key,
        target_token_account.key,
        source_wallet.key,
        &[],
        amount,
        decimals,
    )?;

    remaining_accounts.iter().for_each(|meta| {
        ix.accounts.push(AccountMeta {
            pubkey: meta.key(),
            is_signer: false,
            is_writable: false,
        })
    });

    let infos = [
        &[
            source_token_account.clone(),
            mint.clone(),
            target_token_account.clone(),
            source_wallet.clone(),
        ],
        remaining_accounts,
    ]
    .concat();

    match authority_seeds {
        Some(x) => {
            invoke_signed(&ix, infos.as_slice(), x)?;
        }
        None => {
            invoke(&ix, infos.as_slice())?;
        }
    }

    Ok(())
}
