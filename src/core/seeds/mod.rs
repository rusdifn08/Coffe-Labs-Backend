pub mod accounts;
pub mod curated_content;

use sqlx::PgPool;
use crate::core::error::AppResult;

pub async fn run_seeds(pool: &PgPool) -> AppResult<()> {
    accounts::seed_admin(pool).await?;
    accounts::seed_business_demo_account(pool).await?;
    curated_content::seed_version_config(pool).await?;
    curated_content::seed_social_posts(pool).await?;
    curated_content::seed_curated_beans(pool).await?;
    Ok(())
}
