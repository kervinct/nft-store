use anchor_lang::prelude::*;
use anchor_spl::token::{
    self,
    Token,
    TokenAccount,
    Transfer,
    Mint,
};
use solana_program::{
    self,
    system_instruction,
};
use std::ops::Deref;

declare_id!("cCSrAM5p4R3tzUnja7hHCMTzdWgvwKhdKwe3cchRVLz");

pub(crate) const TOKEN_SEEDS: &'static [u8] = b"nft_account";

#[program]
pub mod nftstore {
    use super::*;

    pub fn initialize_store(
        ctx: Context<InitializeStore>,
        store_name: String,
        bumps: u8,
    ) -> ProgramResult {
        let store_account = &mut ctx.accounts.store_account;

        let name_bytes = store_name.as_bytes();
        let mut name_data = [b' '; 10];
        name_data[..name_bytes.len()].copy_from_slice(name_bytes);

        store_account.store_name = name_data;
        store_account.bumps = bumps;
        store_account.owner = ctx.accounts.creator.key();

        Ok(())
    }

    pub fn freeze_store(
        ctx: Context<FreezeStore>,
    ) -> ProgramResult {
        msg!("STORE FREEZED");
        let store_account = &mut ctx.accounts.store_account;
        store_account.frozen = true;

        Ok(())
    }

    pub fn thaw_store(
        ctx: Context<ThawStore>,
    ) -> ProgramResult {
        msg!("THAW FREEZED STORE");
        let store_account = &mut ctx.accounts.store_account;
        store_account.frozen = false;

        Ok(())
    }

    pub fn initialize_record(
        ctx: Context<InitializeRecord>, 
        bumps: RecordBumps,
    ) -> ProgramResult {
        let record_account = &mut ctx.accounts.record_account;
        record_account.mint = ctx.accounts.nft_mint.key();
        record_account.initializer = ctx.accounts.authority.key();
        record_account.bumps = bumps;
        record_account.on_sale = false;

        Ok(())
    }

    pub fn sell_nft(ctx: Context<SellNft>, price: u64, rate: u16) -> ProgramResult {
        if rate < 1 {
            return Err(ErrorCode::InvalidRate.into());
        }
        let record_account = &mut ctx.accounts.record_account;
        record_account.seller = ctx.accounts.authority.key();
        record_account.price = price;
        record_account.rate = std::cmp::max(1, std::cmp::min(rate, 5_000)); 
        record_account.on_sale = true;

        token::transfer(
            ctx.accounts.into_token_transfer_ctx(),
            1,
        )?;

        let mut fee = (price as u128)
            .checked_mul(rate as u128)
            .unwrap()
            .checked_div(10_000u128)
            .unwrap();

        fee = std::cmp::max(fee, 10_000_000);

        let fee_ix = system_instruction::transfer(
            ctx.accounts.authority.key,
            &ctx.accounts.record_account.key(),
            fee as u64,
        );
        solana_program::program::invoke_signed(
            &fee_ix,
            &[
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.record_account.to_account_info(),
            ],
            &[],
        )?;

        emit!(LaunchEvent {
            seller: ctx.accounts.authority.key(),
            mint: ctx.accounts.record_account.mint,
            price,
            rate,
            label: "sell_nft".to_string(),
        });

        Ok(())
    }

    pub fn redeem_nft(ctx: Context<RedeemNft>) -> ProgramResult {
        let record_account = &mut ctx.accounts.record_account;
        record_account.on_sale = false;

        let store_name = ctx.accounts.store_account.store_name.as_ref();
        let seeds = &[
            store_name.trim_ascii_whitespace(),
            &[ctx.accounts.store_account.bumps],
        ];
        let signer = &[&seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.record_token_account.to_account_info(),
            to: ctx.accounts.authority_token_account.to_account_info(),
            authority: ctx.accounts.store_account.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, 1u64)?;

        let mut fee = (ctx.accounts.record_account.price as u128)
            .checked_mul(ctx.accounts.record_account.rate as u128)
            .unwrap()
            .checked_div(10_000u128)
            .unwrap();
        fee = std::cmp::max(fee, 10_000_000);

        **ctx.accounts.record_account.to_account_info().try_borrow_mut_lamports()? -= fee as u64;
        **ctx.accounts.authority.try_borrow_mut_lamports()? += fee as u64;

        emit!(RedeemEvent{
            redeem: ctx.accounts.authority.key(),
            mint: ctx.accounts.record_account.mint,
            label: "redeem_nft".to_string(),
        });

        Ok(())
    }

    pub fn buy_nft(ctx: Context<BuyNft>) -> ProgramResult {
        let record_account = &mut ctx.accounts.record_account;
        let sold_record = &mut ctx.accounts.sold_record;

        sold_record.index = record_account.current_index;
        sold_record.price = record_account.price;
        sold_record.seller = record_account.seller;
        sold_record.customer = ctx.accounts.authority.key();
        sold_record.rate = record_account.rate;
        sold_record.mint = record_account.mint;
        sold_record.created_at = ctx.accounts.clock.unix_timestamp;

        record_account.current_index += 1;
        record_account.on_sale = false;
        record_account.volume = record_account.volume
            .checked_add(record_account.price as u128)
            .unwrap();

        let price = record_account.price;

        let store_name = ctx.accounts.store_account.store_name.as_ref();
        let seeds = &[
            store_name.trim_ascii_whitespace(),
            &[ctx.accounts.store_account.bumps],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.record_token_account.to_account_info(),
            to: ctx.accounts.authority_token_account.to_account_info(),
            authority: ctx.accounts.store_account.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, 1u64)?;

        let mut fee = (ctx.accounts.record_account.price as u128)
            .checked_mul(ctx.accounts.record_account.rate as u128)
            .unwrap()
            .checked_div(10_000u128)
            .unwrap();
        fee = std::cmp::max(fee, 10_000_000);

        if price != 0 {
            let ix = system_instruction::transfer(
                ctx.accounts.authority.key,
                ctx.accounts.receiver.key,
                price as u64,
            );
            solana_program::program::invoke_signed(
                &ix,
                &[
                    ctx.accounts.authority.to_account_info(),
                    ctx.accounts.receiver.to_account_info(),
                ],
                &[],
            )?;
        }

        **ctx.accounts.record_account.to_account_info().try_borrow_mut_lamports()? -= fee as u64;
        **ctx.accounts.holder.try_borrow_mut_lamports()? += fee as u64;

        emit!(SoldEvent{
            seller: ctx.accounts.record_account.seller,
            mint: ctx.accounts.record_account.mint,
            customer: ctx.accounts.authority.key(),
            index: sold_record.index,
            price: sold_record.price,
            rate: sold_record.rate,
            created_at: sold_record.created_at,
            label: "buy_nft".to_string(),
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Empty {}

#[derive(Accounts)]
#[instruction(store_name: String, bumps: u8)]
pub struct InitializeStore<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(init,
        seeds = [store_name.as_bytes()],
        bump = bumps,
        payer = creator,
    )]
    pub store_account: Account<'info, StoreAccount>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct FreezeStore<'info> {
    pub creator: Signer<'info>,

    #[account(mut,
        seeds = [store_account.store_name.as_ref().trim_ascii_whitespace()],
        bump = store_account.bumps,
        constraint = !store_account.frozen,
        constraint = store_account.owner == creator.key(),
    )]
    pub store_account: Account<'info, StoreAccount>,
}

#[derive(Accounts)]
pub struct ThawStore<'info> {
    pub creator: Signer<'info>,

    #[account(mut,
        seeds = [store_account.store_name.as_ref().trim_ascii_whitespace()],
        bump = store_account.bumps,
        constraint = store_account.frozen,
        constraint = store_account.owner == creator.key(),
    )]
    pub store_account: Account<'info, StoreAccount>,
}

#[derive(Accounts)]
#[instruction(bumps: RecordBumps)]
pub struct InitializeRecord<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(constraint = nft_mint.decimals == 0,
        constraint = nft_mint.supply == 1,
    )]
    pub nft_mint: Account<'info, Mint>,

    #[account(init,
        token::mint = nft_mint,
        token::authority = store_account,
        seeds = [nft_mint.key().as_ref(), TOKEN_SEEDS],
        bump = bumps.record_token_account,
        payer = authority,
    )]
    pub record_token_account: Account<'info, TokenAccount>,

    #[account(init, 
        seeds = [nft_mint.key().as_ref(), RecordAccount::SEEDS],
        bump = bumps.record_account,
        payer = authority, 
    )]
    pub record_account: Account<'info, RecordAccount>,

    #[account(
        seeds = [store_account.store_name.as_ref().trim_ascii_whitespace()],
        bump = store_account.bumps,
        constraint = !store_account.frozen,
    )]
    pub store_account: Account<'info, StoreAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(AnchorDeserialize, AnchorSerialize, Default, Clone)]
pub struct RecordBumps {
   pub record_token_account: u8,
   pub record_account: u8,
}

#[derive(Accounts)]
pub struct SellNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut,
        constraint = authority_token_account.owner == authority.key(),
        constraint = authority_token_account.mint == record_account.mint,
    )]
    pub authority_token_account: Account<'info, TokenAccount>,

    #[account(mut,
        seeds = [record_account.mint.as_ref(), TOKEN_SEEDS],
        bump = record_account.bumps.record_token_account,
    )]
    pub record_token_account: Account<'info, TokenAccount>, 

    #[account(mut,
        seeds = [record_account.mint.as_ref(), RecordAccount::SEEDS],
        bump = record_account.bumps.record_account,
        constraint = !record_account.on_sale,
    )]
    pub record_account: Account<'info, RecordAccount>,

    #[account(
        seeds = [store_account.store_name.as_ref().trim_ascii_whitespace()],
        bump = store_account.bumps,
        constraint = !store_account.frozen,
    )]
    pub store_account: Account<'info, StoreAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> SellNft<'info> {
    pub fn into_token_transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.authority_token_account.to_account_info(),
            to: self.record_token_account.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        CpiContext::new(cpi_program, cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct RedeemNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut,
        constraint = authority_token_account.owner == authority.key(),
        constraint = authority_token_account.mint == record_account.mint,
    )]
    pub authority_token_account: Account<'info, TokenAccount>,

    #[account(mut,
        seeds = [record_account.mint.as_ref(), TOKEN_SEEDS],
        bump = record_account.bumps.record_token_account,
    )]
    pub record_token_account: Account<'info, TokenAccount>,

    #[account(mut,
        seeds = [record_account.mint.as_ref(), RecordAccount::SEEDS],
        bump = record_account.bumps.record_account,
        constraint = record_account.on_sale,
    )]
    pub record_account: Account<'info, RecordAccount>,

    #[account(
        seeds = [store_account.store_name.as_ref().trim_ascii_whitespace()],
        bump = store_account.bumps,
    )]
    pub store_account: Account<'info, StoreAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyNft<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut,
        constraint = receiver.key() == record_account.seller)]
    pub receiver: AccountInfo<'info>,

    #[account(mut,
        constraint = holder.key() == store_account.owner)]
    pub holder: AccountInfo<'info>,

    #[account(init,
        seeds = [record_account.mint.as_ref(),
            SoldRecord::SEEDS, &record_account.current_index.to_le_bytes()],
        bump,
        payer = authority,
    )]
    pub sold_record: Account<'info, SoldRecord>,

    #[account(mut,
        constraint = authority_token_account.owner == authority.key(),
        constraint = authority_token_account.mint == record_account.mint,
    )]
    pub authority_token_account: Account<'info, TokenAccount>,

    #[account(mut,
        seeds = [record_account.mint.as_ref(), TOKEN_SEEDS],
        bump = record_account.bumps.record_token_account,
    )]
    pub record_token_account: Account<'info, TokenAccount>,

    #[account(mut,
        seeds = [record_account.mint.as_ref(), RecordAccount::SEEDS],
        bump = record_account.bumps.record_account,
    )]
    pub record_account: Account<'info, RecordAccount>,

    #[account(
        seeds = [store_account.store_name.as_ref().trim_ascii_whitespace()],
        bump = store_account.bumps,
    )]
    pub store_account: Account<'info, StoreAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[account]
#[derive(Default)]
pub struct StoreAccount {
    pub store_name: [u8; 10],
    pub bumps: u8,
    pub frozen: bool,
    pub owner: Pubkey,
}

#[account]
#[derive(Default)]
pub struct RecordAccount {
    pub on_sale: bool,
    pub volume: u128,
    pub initializer: Pubkey,
    pub seller: Pubkey,
    pub bumps: RecordBumps,
    pub mint: Pubkey,
    pub current_index: u32,
    pub rate: u16,
    pub price: u64,
}

impl RecordAccount {
    pub const SEEDS: &'static [u8] = b"nft_record_account";
    pub const LEN: usize = (8 + 1 + 16 + 32 + 32 + 2 + 32 + 4 + 2 + 8) as usize;
}

#[account]
#[derive(Default)]
pub struct SoldRecord {
    pub index: u32,
    pub price: u64,
    pub seller: Pubkey,
    pub customer: Pubkey,
    pub rate: u16,
    pub mint: Pubkey,
    pub created_at: i64,
}

impl SoldRecord {
    pub const SEEDS: &'static [u8] = b"sold_record";
}

#[error]
pub enum ErrorCode {
    #[msg("Unauthorized operation")]
    Unauthorized,
    #[msg("Invalid rate")]
    InvalidRate,
}

#[event]
pub struct LaunchEvent {
    seller: Pubkey,
    mint: Pubkey,
    price: u64,
    rate: u16,
    #[index]
    label: String,
}

#[event]
pub struct RedeemEvent {
    redeem: Pubkey,
    mint: Pubkey,
    #[index]
    label: String,
}

#[event]
pub struct SoldEvent {
    seller: Pubkey,
    mint: Pubkey,
    customer: Pubkey,
    index: u32,
    price: u64,
    rate: u16,
    created_at: i64,
    #[index]
    label: String,
}

pub trait TrimAsciiWhitespace {
    fn trim_ascii_whitespace(&self) -> &[u8];
}

impl<T: Deref<Target = [u8]>> TrimAsciiWhitespace for T {
    fn trim_ascii_whitespace(&self) -> &[u8] {
        let from = match self.iter().position(|x| !x.is_ascii_whitespace()) {
            Some(i) => i,
            None => return &self[0..0],
        };
        let to = self.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
        &self[from..=to]
    }
}
