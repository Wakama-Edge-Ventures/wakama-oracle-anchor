use anchor_lang::prelude::*;

declare_id!("93eL55wjf62Pw8UPsKS8V7b9efk28UyG8C74Vif2gMNR");

#[program]
pub mod wakama_oracle_anchor {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
