#[allow(unused_imports)]
pub use crate::core::extractors::{
    RequireTier, TierBusiness, TierPro, TierRequirement, TierTrial,
};
use crate::modules::rbac::models::{AccountTier, PermissionScope};

pub fn permissions_for_tier(tier: AccountTier) -> Vec<PermissionScope> {
    match tier {
        AccountTier::Trial => vec![PermissionScope::BasicBrewTimer],
        AccountTier::Pro => vec![
            PermissionScope::BasicBrewTimer,
            PermissionScope::UnlimitedBrews,
            PermissionScope::FullChampionRecipes,
            PermissionScope::ZeroAds,
            PermissionScope::AiDialInLab,
            PermissionScope::GrinderMatrix,
            PermissionScope::WaterAndTdsLab,
        ],
        AccountTier::Business => vec![
            PermissionScope::BasicBrewTimer,
            PermissionScope::UnlimitedBrews,
            PermissionScope::FullChampionRecipes,
            PermissionScope::ZeroAds,
            PermissionScope::AiDialInLab,
            PermissionScope::GrinderMatrix,
            PermissionScope::WaterAndTdsLab,
            PermissionScope::MultiBaristaSync,
            PermissionScope::ThermalReceiptPrint,
            PermissionScope::SensoryExportPdfCsv,
            PermissionScope::BeanRadarPriority,
        ],
    }
}
