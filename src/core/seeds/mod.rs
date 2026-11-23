pub mod accounts;
pub mod curated_content;
pub mod pro_accounts;
pub mod toolkit_content_seed;
pub mod toolkit_lms_seed;

use sqlx::PgPool;
use crate::core::error::AppResult;

pub async fn run_seeds(pool: &PgPool) -> AppResult<()> {
    accounts::seed_admin(pool).await?;
    accounts::seed_business_demo_account(pool).await?;
    pro_accounts::seed_registered_pro_accounts(pool).await?;
    curated_content::seed_version_config(pool).await?;
    curated_content::seed_curated_beans(pool).await?;
    toolkit_lms_seed::seed_lms_courses_and_lessons(pool).await?;
    toolkit_content_seed::seed_ebooks_and_game_chapters(pool).await?;
    Ok(())
}
