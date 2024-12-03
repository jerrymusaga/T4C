use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::metadata::{
    create_master_edition_v3, create_metadata_accounts_v3, mpl_token_metadata::types::DataV2,
    CreateMasterEditionV3, CreateMetadataAccountsV3, MetadataAccount,
};
use anchor_spl::token::{self, Mint, Token, TokenAccount};

// Constants
pub const NFT_CONFIG_SEED: &[u8] = b"nft-config";
pub const REDEEMABLE_MINT_SEED: &[u8] = b"redeemable-mint";
pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_SYMBOL_LENGTH: usize = 10;
pub const MAX_URI_LENGTH: usize = 200;

declare_id!("EfmbcacUa2G3w7hChRSTUeP6yQ8MNh2Jv8oVTgwQijbJ");
#[program]
pub mod trash4coin {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, max_nft_types: u8) -> Result<()> {
        require!(
            max_nft_types > 0 && max_nft_types <= 10,
            ErrorCode::InvalidMaxNFTTypes
        );

        let nft_config = &mut ctx.accounts.nft_config;
        nft_config.authority = ctx.accounts.authority.key();
        nft_config.max_nft_types = max_nft_types;
        nft_config.nft_types = vec![];

        emit!(InitializeEvent {
            authority: ctx.accounts.authority.key(),
            max_nft_types,
        });

        Ok(())
    }

    pub fn add_nft_type(
        ctx: Context<AddNFTType>,
        name: String,
        symbol: String,
        uri: String,
    ) -> Result<()> {
        // Add authority check
        require!(
            ctx.accounts.authority.key() == ctx.accounts.nft_config.authority,
            ErrorCode::UnauthorizedAccess
        );

        require!(
            ctx.accounts.nft_config.nft_types.len()
                < ctx.accounts.nft_config.max_nft_types as usize,
            ErrorCode::MaxNFTTypesReached
        );
        require!(!name.is_empty(), ErrorCode::EmptyName);
        require!(!symbol.is_empty(), ErrorCode::EmptySymbol);
        require!(!uri.is_empty(), ErrorCode::EmptyURI);

        ctx.accounts.nft_config.nft_types.push(NFTType {
            name: name.clone(),
            symbol: symbol.clone(),
            uri: uri.clone(),
            reward_amount: None,
        });

        emit!(AddNFTTypeEvent { name, symbol, uri });

        Ok(())
    }

    pub fn set_reward_amount(
        ctx: Context<SetRewardAmount>,
        nft_type_index: u8,
        reward_amount: u64,
    ) -> Result<()> {
        // Add authority check
        require!(
            ctx.accounts.authority.key() == ctx.accounts.nft_config.authority,
            ErrorCode::UnauthorizedAccess
        );
        require!(
            (nft_type_index as usize) < ctx.accounts.nft_config.nft_types.len(),
            ErrorCode::InvalidNFTType
        );
        require!(reward_amount > 0, ErrorCode::InvalidRewardAmount);

        let nft_type = &mut ctx.accounts.nft_config.nft_types[nft_type_index as usize];
        nft_type.reward_amount = Some(reward_amount);

        emit!(SetRewardAmountEvent {
            nft_type_index,
            reward_amount,
        });

        Ok(())
    }

    pub fn edit_reward_amount(
        ctx: Context<EditRewardAmount>,
        nft_type_index: u8,
        new_reward_amount: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.nft_config.authority,
            ErrorCode::UnauthorizedAccess
        );
        require!(
            (nft_type_index as usize) < ctx.accounts.nft_config.nft_types.len(),
            ErrorCode::InvalidNFTType
        );
        require!(new_reward_amount > 0, ErrorCode::InvalidRewardAmount);

        let nft_type = &mut ctx.accounts.nft_config.nft_types[nft_type_index as usize];
        let old_reward_amount = nft_type.reward_amount.unwrap_or(0);
        nft_type.reward_amount = Some(new_reward_amount);

        emit!(EditRewardAmountEvent {
            nft_type_index,
            old_reward_amount,
            new_reward_amount,
        });

        Ok(())
    }

    pub fn increase_token_supply(ctx: Context<IncreaseTokenSupply>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.redeemable_mint.to_account_info(),
                    to: ctx.accounts.redeemable_token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;
        emit!(IncreaseTokenSupplyEvent {
            mint: ctx.accounts.redeemable_mint.key(),
            amount,
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn increase_max_nft_types(ctx: Context<IncreaseMaxNFTTypes>, new_max: u8) -> Result<()> {
        require!(
            new_max > ctx.accounts.nft_config.max_nft_types,
            ErrorCode::InvalidNewMaxNFTTypes
        );

        ctx.accounts.nft_config.max_nft_types = new_max;

        emit!(IncreaseMaxNFTTypesEvent {
            authority: ctx.accounts.authority.key(),
            old_max: ctx.accounts.nft_config.max_nft_types,
            new_max,
        });

        Ok(())
    }

    pub fn mint_nft(ctx: Context<MintNFT>, nft_type_index: u8, amount: u64) -> Result<()> {
        require!(
            (nft_type_index as usize) < ctx.accounts.nft_config.nft_types.len(),
            ErrorCode::InvalidNFTType
        );
        require!(amount > 0, ErrorCode::InvalidAmount);

        let nft_type = &ctx.accounts.nft_config.nft_types[nft_type_index as usize];

        // Create metadata for the NFT
        let cpi_accounts = CreateMetadataAccountsV3 {
            metadata: ctx.accounts.metadata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            mint_authority: ctx.accounts.minter.to_account_info(),
            payer: ctx.accounts.minter.to_account_info(),
            update_authority: ctx.accounts.minter.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_context = CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            cpi_accounts,
        );

        let data = DataV2 {
            name: nft_type.name.clone(),
            symbol: nft_type.symbol.clone(),
            uri: nft_type.uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        create_metadata_accounts_v3(cpi_context, data, true, false, None)?;

        // Create master edition (with max supply set to amount)
        let cpi_accounts = CreateMasterEditionV3 {
            edition: ctx.accounts.master_edition.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            update_authority: ctx.accounts.minter.to_account_info(),
            mint_authority: ctx.accounts.minter.to_account_info(),
            metadata: ctx.accounts.metadata.to_account_info(),
            payer: ctx.accounts.minter.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            rent: ctx.accounts.rent.to_account_info(),
        };

        let cpi_context = CpiContext::new(
            ctx.accounts.token_metadata_program.to_account_info(),
            cpi_accounts,
        );

        create_master_edition_v3(cpi_context, Some(amount))?;

        // Mint tokens
        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.token_account.to_account_info(),
                    authority: ctx.accounts.minter.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(MintNFTEvent {
            mint: ctx.accounts.mint.key(),
            owner: ctx.accounts.minter.key(),
            nft_type_index,
            amount,
        });

        Ok(())
    }

    pub fn create_redeemable_token(ctx: Context<CreateRedeemableToken>, amount: u64) -> Result<()> {
        // Add authority check
        require!(
            ctx.accounts.authority.key() == ctx.accounts.nft_config.authority,
            ErrorCode::UnauthorizedAccess
        );
        require!(amount > 0, ErrorCode::InvalidAmount);

        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.redeemable_mint.to_account_info(),
                    to: ctx.accounts.redeemable_token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(CreateRedeemableTokenEvent {
            mint: ctx.accounts.redeemable_mint.key(),
            amount,
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    pub fn redeem_and_burn_nft(ctx: Context<RedeemAndBurnNFT>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);

        // Deserialize the metadata account
        let metadata =
            MetadataAccount::try_deserialize(&mut &ctx.accounts.metadata.data.borrow()[..])
                .map_err(|_| ErrorCode::InvalidMetadata)?;

        // Extract name and uri
        let name = metadata.name.trim_end_matches('\0').to_string();
        let uri = metadata.uri.trim_end_matches('\0').to_string();

        // Find the matching NFT type
        let nft_type = ctx
            .accounts
            .nft_config
            .nft_types
            .iter()
            .find(|t| t.name == name && t.uri == uri)
            .ok_or(ErrorCode::InvalidNFTType)?;

        let reward_amount = nft_type
            .reward_amount
            .ok_or(ErrorCode::RewardAmountNotSet)?;

        // Calculate total reward
        let total_reward = reward_amount
            .checked_mul(amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?;

        // Burn the NFTs
        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.nft_mint.to_account_info(),
                    from: ctx.accounts.nft_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Transfer redeemable tokens to the user
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.redeemable_token_account.to_account_info(),
                    to: ctx.accounts.user_redeemable_token_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            total_reward,
        )?;

        emit!(RedeemAndBurnNFTEvent {
            user: ctx.accounts.user.key(),
            nft_mint: ctx.accounts.nft_mint.key(),
            amount_burned: amount,
            reward_amount: total_reward,
        });

        Ok(())
    }

    pub fn get_user_info(ctx: Context<GetUserInfo>) -> Result<UserInfo> {
        let nft_balance = ctx.accounts.nft_token_account.amount;
        let redeemable_balance = ctx.accounts.user_redeemable_token_account.amount;

        let user_info = UserInfo {
            nft_mint: ctx.accounts.nft_mint.key(),
            nft_balance,
            redeemable_balance,
        };

        emit!(UserInfoFetched {
            user: ctx.accounts.user.key(),
            nft_mint: user_info.nft_mint,
            nft_balance: user_info.nft_balance,
            redeemable_balance: user_info.redeemable_balance,
        });

        Ok(user_info)
    }

}

#[derive(Accounts)]
#[instruction(max_nft_types: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = NFTConfig::space(),
        seeds = [NFT_CONFIG_SEED, authority.key().as_ref()],
        bump
    )]
    pub nft_config: Account<'info, NFTConfig>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(nft_type_index: u8, amount: u64)]
pub struct MintNFT<'info> {
    #[account(mut)]
    pub minter: Signer<'info>,

    #[account(
        init,
        payer = minter,
        mint::decimals = 0,
        mint::authority = minter.key(),
        mint::freeze_authority = minter.key(),
        seeds = [b"nft-mint", nft_config.key().as_ref(), &[nft_type_index]],
        bump
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = minter,
        associated_token::mint = mint,
        associated_token::authority = minter,
    )]
    pub token_account: Account<'info, TokenAccount>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint.key().as_ref()],
        seeds::program = token_metadata_program.key(),
        bump
    )]
    pub metadata: UncheckedAccount<'info>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint.key().as_ref(), b"edition"],
        seeds::program = token_metadata_program.key(),
        bump
    )]
    pub master_edition: UncheckedAccount<'info>,

    #[account(
        seeds = [NFT_CONFIG_SEED, nft_config.authority.as_ref()],
        bump
    )]
    pub nft_config: Account<'info, NFTConfig>,

    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_metadata_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CreateRedeemableToken<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = authority.key(),
        seeds = [REDEEMABLE_MINT_SEED, nft_config.key().as_ref()],
        bump
    )]
    pub redeemable_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = redeemable_mint,
        associated_token::authority = authority,
    )]
    pub redeemable_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [NFT_CONFIG_SEED, authority.key().as_ref()],
        bump,
        constraint = nft_config.authority == authority.key() @ ErrorCode::UnauthorizedAccess
    )]
    pub nft_config: Account<'info, NFTConfig>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct RedeemAndBurnNFT<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = user,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub redeemable_mint: Account<'info, Mint>,
    #[account(mut)]
    pub redeemable_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = redeemable_mint,
        associated_token::authority = user,
    )]
    pub user_redeemable_token_account: Account<'info, TokenAccount>,
    /// CHECK: This account is verified in the instruction logic
    #[account(mut)]
    pub authority: UncheckedAccount<'info>,
    /// CHECK: This account is used for metadata verification
    pub metadata: UncheckedAccount<'info>,
    pub nft_config: Account<'info, NFTConfig>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct NFTConfig {
    pub authority: Pubkey,
    pub max_nft_types: u8,
    pub nft_types: Vec<NFTType>,
}

impl NFTConfig {
    // Calculate the total space required for the NFTConfig account
    pub const fn space() -> usize {
        // Account discriminator (8 bytes)
        8 + 
        // Pubkey (32 bytes)
        32 +
        // max_nft_types (1 byte)
        1 +
        // Vec discriminator (4 bytes for storing length)
        4 +
        // Maximum space for Vec contents
        // Calculate space for 10 NFT types initially
        10 * NFTType::space()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct NFTType {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub reward_amount: Option<u64>,
}

impl NFTType {
    // Calculate space for a single NFTType
    pub const fn space() -> usize {
        // String format: 4 bytes for length + actual content

        // name: 4 + MAX_NAME_LENGTH
        4 + MAX_NAME_LENGTH +
        
        // symbol: 4 + MAX_SYMBOL_LENGTH
        4 + MAX_SYMBOL_LENGTH +
        
        // uri: 4 + MAX_URI_LENGTH
        4 + MAX_URI_LENGTH +
        
        // Option<u64>: 1 byte for discriminator + 8 bytes for u64
        9
    }
}

#[derive(Accounts)]
pub struct AddNFTType<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub nft_config: Account<'info, NFTConfig>,
}

#[derive(Accounts)]
pub struct SetRewardAmount<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub nft_config: Account<'info, NFTConfig>,
}

#[derive(Accounts)]
pub struct IncreaseTokenSupply<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub redeemable_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = redeemable_mint,
        associated_token::authority = authority,
    )]
    pub redeemable_token_account: Account<'info, TokenAccount>,
    #[account(constraint = nft_config.authority == authority.key() @ ErrorCode::UnauthorizedAccess)]
    pub nft_config: Account<'info, NFTConfig>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct IncreaseMaxNFTTypes<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority @ ErrorCode::UnauthorizedAccess)]
    pub nft_config: Account<'info, NFTConfig>,
}

#[derive(Accounts)]
pub struct EditRewardAmount<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority @ ErrorCode::UnauthorizedAccess)]
    pub nft_config: Account<'info, NFTConfig>,
}

#[derive(Accounts)]
pub struct GetUserInfo<'info> {
    pub user: Signer<'info>,
    pub nft_mint: Account<'info, Mint>,
    #[account(
        associated_token::mint = nft_mint,
        associated_token::authority = user,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    pub redeemable_mint: Account<'info, Mint>,
    #[account(
        associated_token::mint = redeemable_mint,
        associated_token::authority = user,
    )]
    pub user_redeemable_token_account: Account<'info, TokenAccount>,
}

// struct to represent the return value
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UserInfo {
    pub nft_mint: Pubkey,
    pub nft_balance: u64,
    pub redeemable_balance: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid NFT type")]
    InvalidNFTType,
    #[msg("Maximum number of NFT types reached")]
    MaxNFTTypesReached,
    #[msg("Reward amount not set for this NFT type")]
    RewardAmountNotSet,
    #[msg("Empty name provided")]
    EmptyName,
    #[msg("Empty symbol provided")]
    EmptySymbol,
    #[msg("Empty URI provided")]
    EmptyURI,
    #[msg("Invalid reward amount")]
    InvalidRewardAmount,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid metadata")]
    InvalidMetadata,
    #[msg("Unauthorized access")]
    UnauthorizedAccess,
    #[msg("New maximum number of NFT types must be greater than the current maximum")]
    InvalidNewMaxNFTTypes,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Invalid maximum number of NFT types (must be between 1 and 10)")]
    InvalidMaxNFTTypes,
}

#[event]
pub struct InitializeEvent {
    pub authority: Pubkey,
    pub max_nft_types: u8,
}

#[event]
pub struct AddNFTTypeEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[event]
pub struct SetRewardAmountEvent {
    pub nft_type_index: u8,
    pub reward_amount: u64,
}

#[event]
pub struct MintNFTEvent {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub nft_type_index: u8,
    pub amount: u64,
}

#[event]
pub struct CreateRedeemableTokenEvent {
    pub mint: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
}

#[event]
pub struct RedeemAndBurnNFTEvent {
    pub user: Pubkey,
    pub nft_mint: Pubkey,
    pub amount_burned: u64,
    pub reward_amount: u64,
}

#[event]
pub struct IncreaseTokenSupplyEvent {
    pub mint: Pubkey,
    pub amount: u64,
    pub authority: Pubkey,
}

#[event]
pub struct IncreaseMaxNFTTypesEvent {
    pub authority: Pubkey,
    pub old_max: u8,
    pub new_max: u8,
}

#[event]
pub struct EditRewardAmountEvent {
    pub nft_type_index: u8,
    pub old_reward_amount: u64,
    pub new_reward_amount: u64,
}

#[event]
pub struct UserInfoFetched {
    pub user: Pubkey,
    pub nft_mint: Pubkey,
    pub nft_balance: u64,
    pub redeemable_balance: u64,
}
