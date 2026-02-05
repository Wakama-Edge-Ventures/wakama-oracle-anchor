use anchor_lang::prelude::*;

declare_id!("HgdtgGR8pw6T3eC4cwBFtRYcNoGzDN2ujkXkbw2oCnzj"); 

/// Taille de l’account RWA (nous avons laissé un petit buffer pour évoluer)
pub const RWA_ASSET_SIZE: usize = 8   // discriminator
    + 32  // authority
    + 32  // oracle_authority
    + 32  // asset_mint
    + 1   // bump
    + 1   // status (u8)
    + 8   // total_points
    + 8   // total_batches
    + 8   // invested_usdc
    + 8   // last_oracle_update_slot
    + 8   // created_at
    + 8   // updated_at
    + 32; // marge future

#[program]
pub mod wakama_oracle_anchor {
    use super::*;

    /// Crée un “RWA asset” d’exemple.
    ///
    /// - `asset_mint` : mint associé à l’asset (ex: USDC-vault, NFT de parcelle, etc).
    /// - `oracle_authority` : clé qui aura le droit de pousser des mises à jour de points.
    pub fn initialize_asset(
        ctx: Context<InitializeAsset>,
        asset_mint: Pubkey,
        oracle_authority: Pubkey,
    ) -> Result<()> {
        let asset = &mut ctx.accounts.asset;
        let clock = Clock::get()?;

        asset.authority = ctx.accounts.payer.key();
        asset.oracle_authority = oracle_authority;
        asset.asset_mint = asset_mint;
        asset.bump = ctx.bumps.asset;

        asset.status = RwaStatus::Pending;
        asset.total_points = 0;
        asset.total_batches = 0;
        asset.invested_usdc = 0;
        asset.last_oracle_update_slot = 0;
        asset.created_at = clock.unix_timestamp;
        asset.updated_at = clock.unix_timestamp;

        emit!(AssetInitialized {
            asset: asset.key(),
            authority: asset.authority,
            oracle_authority,
            asset_mint,
            created_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Mise à jour oracle : ici nous devons ajouter des “points” et des “batches” sur l’asset.
    ///
    /// Utilisée pour connecter le pipeline Wakama Oracle (150k points, etc.).
    pub fn push_oracle_update(
        ctx: Context<PushOracleUpdate>,
        points_delta: u64,
        batch_count: u64,
    ) -> Result<()> {
        let asset = &mut ctx.accounts.asset;
        let clock = Clock::get()?;

        // sécurité : seule la clé d’oracle peut appeler
        require_keys_eq!(
            ctx.accounts.oracle.key(),
            asset.oracle_authority,
            ErrorCode::UnauthorizedOracle
        );

        asset.total_points = asset.total_points.saturating_add(points_delta);
        asset.total_batches = asset.total_batches.saturating_add(batch_count);
        asset.last_oracle_update_slot = clock.slot;
        asset.updated_at = clock.unix_timestamp;

        emit!(OracleUpdate {
            asset: asset.key(),
            oracle: ctx.accounts.oracle.key(),
            points_delta,
            batch_count,
            new_total_points: asset.total_points,
            new_total_batches: asset.total_batches,
            slot: clock.slot,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Change le statut “lifecycle” de l’asset RWA.
    ///
    /// 0 = Pending, 1 = Active, 2 = Redeemed, 3 = Defaulted.
    ///
    /// Autorité : `asset.authority` (peut être un multisig ou un realm DAO).
    pub fn set_status(ctx: Context<SetStatus>, new_status: u8) -> Result<()> {
        let asset = &mut ctx.accounts.asset;
        let clock = Clock::get()?;

        require_keys_eq!(
            ctx.accounts.authority.key(),
            asset.authority,
            ErrorCode::UnauthorizedAuthority
        );

        let status = RwaStatus::from_u8(new_status)?;
        let old_status = asset.status;
        asset.status = status;
        asset.updated_at = clock.unix_timestamp;

        emit!(StatusChanged {
            asset: asset.key(),
            authority: ctx.accounts.authority.key(),
            old_status,
            new_status: status,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Enregistre un “volume investi” (proxy simple en USDC, en u64).
    ///
    /// Ici on ne transfère pas réellement de tokens : le but est d’avoir un
    /// exemple lisible pour l'evolution du projet et les milestones (“≥ 20k USDC investis”).
    pub fn record_investment(
        ctx: Context<RecordInvestment>,
        amount_usdc: u64,
    ) -> Result<()> {
        let asset = &mut ctx.accounts.asset;
        let clock = Clock::get()?;

        require_keys_eq!(
            ctx.accounts.authority.key(),
            asset.authority,
            ErrorCode::UnauthorizedAuthority
        );

        asset.invested_usdc = asset
            .invested_usdc
            .saturating_add(amount_usdc);
        asset.updated_at = clock.unix_timestamp;

        emit!(InvestmentRecorded {
            asset: asset.key(),
            authority: ctx.accounts.authority.key(),
            investor: ctx.accounts.investor.key(),
            amount_usdc,
            new_total_invested: asset.invested_usdc,
            ts: clock.unix_timestamp,
        });

        Ok(())
    }
}

// --------------------------- Accounts ---------------------------

#[derive(Accounts)]
pub struct InitializeAsset<'info> {
    /// PDA qui représente l’asset RWA.
    #[account(
        init,
        payer = payer,
        space = RWA_ASSET_SIZE,
        seeds = [b"rwa-asset", asset_mint.key().as_ref()],
        bump
    )]
    pub asset: Account<'info, RwaAsset>,

    /// Payer de l’init (sera l’authority par défaut).
    #[account(mut)]
    pub payer: Signer<'info>,

        /// CHECK: Mint SPL associé à l’asset (ex : mint d’un token de position).
    /// Nous ne le modifions jamais dans le programme, il sert uniquement
    /// d’identifiant stable dans le PDA `asset` via les seeds.
    /// La sécurité est assurée par les seeds + bump, donc pas besoin
    /// de vérifications supplémentaires via un type Anchor plus strict.
    pub asset_mint: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PushOracleUpdate<'info> {
    #[account(
        mut,
        seeds = [b"rwa-asset", asset.asset_mint.as_ref()],
        bump = asset.bump
    )]
    pub asset: Account<'info, RwaAsset>,

    /// Signer autorisé à pousser les updates de points.
    pub oracle: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetStatus<'info> {
    #[account(
        mut,
        seeds = [b"rwa-asset", asset.asset_mint.as_ref()],
        bump = asset.bump
    )]
    pub asset: Account<'info, RwaAsset>,

    /// Autorité de gestion de l’asset (peut être remplacée plus tard par un realm/DAO).
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct RecordInvestment<'info> {
    #[account(
        mut,
        seeds = [b"rwa-asset", asset.asset_mint.as_ref()],
        bump = asset.bump
    )]
    pub asset: Account<'info, RwaAsset>,

    /// Autorité qui enregistre les investissements (project / DAO).
    pub authority: Signer<'info>,

    /// CHECK: simple compte “investor” utilisé uniquement pour les logs / events.
    /// Nous ne lisons ni n’écrivons dans cet account, nous loggons seulement sa clé
    /// publique dans l’event `InvestmentRecorded`. Aucun accès mémoire critique.
    pub investor: UncheckedAccount<'info>,
}

// --------------------------- State ---------------------------

#[account]
pub struct RwaAsset {
    /// Admin / owner de l’asset (DAO, multisig, Wakama, etc.).
    pub authority: Pubkey,

    /// Oracle autorisé à pousser les `points` (stack Wakama Oracle).
    pub oracle_authority: Pubkey,

    /// Mint associé à l’asset (peut être un token de position ou un NFT).
    pub asset_mint: Pubkey,

    /// Bump du PDA.
    pub bump: u8,

    /// Statut lifecycle de l’actif.
    pub status: RwaStatus,

    /// Total de points agrégés depuis l’oracle (soil, NDVI, batches coop, etc.).
    pub total_points: u64,

    /// Nombre total de “batches” agrégés.
    pub total_batches: u64,

    /// Volume “investi” agrégé (en USDC, format u64 pour l’exemple).
    pub invested_usdc: u64,

    /// Dernier slot où l’oracle a mis à jour cet actif.
    pub last_oracle_update_slot: u64,

    /// Timestamp de création (unix).
    pub created_at: i64,

    /// Timestamp de dernière mise à jour (unix).
    pub updated_at: i64,
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
)]
#[repr(u8)]
pub enum RwaStatus {
    Pending = 0,
    Active = 1,
    Redeemed = 2,
    Defaulted = 3,
}

impl RwaStatus {
    pub fn from_u8(v: u8) -> Result<Self> {
        Ok(match v {
            0 => RwaStatus::Pending,
            1 => RwaStatus::Active,
            2 => RwaStatus::Redeemed,
            3 => RwaStatus::Defaulted,
            _ => return Err(ErrorCode::InvalidStatus.into()),
        })
    }
}

// --------------------------- Events ---------------------------

#[event]
pub struct AssetInitialized {
    pub asset: Pubkey,
    pub authority: Pubkey,
    pub oracle_authority: Pubkey,
    pub asset_mint: Pubkey,
    pub created_at: i64,
}

#[event]
pub struct OracleUpdate {
    pub asset: Pubkey,
    pub oracle: Pubkey,
    pub points_delta: u64,
    pub batch_count: u64,
    pub new_total_points: u64,
    pub new_total_batches: u64,
    pub slot: u64,
    pub ts: i64,
}

#[event]
pub struct StatusChanged {
    pub asset: Pubkey,
    pub authority: Pubkey,
    pub old_status: RwaStatus,
    pub new_status: RwaStatus,
    pub ts: i64,
}

#[event]
pub struct InvestmentRecorded {
    pub asset: Pubkey,
    pub authority: Pubkey,
    pub investor: Pubkey,
    pub amount_usdc: u64,
    pub new_total_invested: u64,
    pub ts: i64,
}

// --------------------------- Errors ---------------------------

#[error_code]
pub enum ErrorCode {
    #[msg("Caller is not the configured oracle authority for this asset.")]
    UnauthorizedOracle,

    #[msg("Caller is not the configured authority for this asset.")]
    UnauthorizedAuthority,

    #[msg("Invalid RWA status value.")]
    InvalidStatus,
}
