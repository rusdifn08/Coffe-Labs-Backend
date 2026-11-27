/// 16-bit strict category bitmask flags matching Android/iOS StrictRadarCategory engine.
pub const FLAG_ROASTERY: u16 = 1 << 0;
pub const FLAG_BEAN_SELLER: u16 = 1 << 1;
pub const FLAG_SPECIALTY_CAFE: u16 = 1 << 2;
pub const FLAG_COFFEE_SHOP: u16 = 1 << 3;
pub const FLAG_CAFE_RESTO: u16 = 1 << 4;
pub const FLAG_SLOW_BAR_V60: u16 = 1 << 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictRadarScope {
    All,
    BeansRadar,
    CafeRadar,
}

impl StrictRadarScope {
    pub fn parse_opt(raw: Option<&str>, sells_beans_legacy: Option<bool>) -> Self {
        match raw.map(|s| s.trim().to_ascii_uppercase()).as_deref() {
            Some("BEANS_RADAR") | Some("BEANS") | Some("BEAN_SELLER") => Self::BeansRadar,
            Some("CAFE_RADAR") | Some("CAFE") => Self::CafeRadar,
            _ if sells_beans_legacy == Some(true) => Self::BeansRadar,
            _ => Self::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrictCategoryFilter {
    All,
    Roastery,
    BeanSeller,
    Cafe,
    SpecialtyCafe,
    Resto,
    SlowBar,
}

impl StrictCategoryFilter {
    pub fn parse_opt(raw: Option<&str>) -> Self {
        match raw.map(|s| s.trim().to_ascii_uppercase()).as_deref() {
            Some("ROASTERY") | Some("ROASTERY_AND_CAFE") => Self::Roastery,
            Some("BEAN_SELLER") | Some("JUAL_BIJI") => Self::BeanSeller,
            Some("CAFE") | Some("COFFEE_SHOP") => Self::Cafe,
            Some("SPECIALTY_CAFE") | Some("SPECIALTY") => Self::SpecialtyCafe,
            Some("CAFE_RESTO") | Some("RESTO") | Some("RESTO_KOPI") => Self::Resto,
            Some("SLOW_BAR") | Some("SLOW_BAR_V60") | Some("V60") => Self::SlowBar,
            _ => Self::All,
        }
    }
}

pub fn compute_category_flags(
    category: &str,
    sells_roast_beans: bool,
    has_manual_brew_v60: bool,
) -> u16 {
    let mut flags: u16 = match category.trim().to_ascii_uppercase().as_str() {
        "ROASTERY" => FLAG_ROASTERY | FLAG_BEAN_SELLER,
        "ROASTERY_AND_CAFE" => FLAG_ROASTERY | FLAG_BEAN_SELLER | FLAG_SPECIALTY_CAFE,
        "BEAN_SELLER" => FLAG_BEAN_SELLER,
        "SPECIALTY_CAFE" => FLAG_SPECIALTY_CAFE,
        "CAFE_RESTO" | "RESTO" => FLAG_CAFE_RESTO,
        _ => FLAG_COFFEE_SHOP,
    };
    if sells_roast_beans {
        flags |= FLAG_BEAN_SELLER;
    }
    if has_manual_brew_v60 {
        flags |= FLAG_SLOW_BAR_V60;
    }
    flags
}

pub fn matches_strict_filter(
    flags: u16,
    scope: StrictRadarScope,
    category: StrictCategoryFilter,
) -> bool {
    let scope_ok = match scope {
        StrictRadarScope::All => true,
        StrictRadarScope::BeansRadar => (flags & (FLAG_ROASTERY | FLAG_BEAN_SELLER)) != 0,
        StrictRadarScope::CafeRadar => {
            (flags & (FLAG_SPECIALTY_CAFE | FLAG_COFFEE_SHOP | FLAG_CAFE_RESTO | FLAG_ROASTERY)) != 0
        }
    };
    if !scope_ok {
        return false;
    }
    match category {
        StrictCategoryFilter::All => true,
        StrictCategoryFilter::Roastery => (flags & FLAG_ROASTERY) != 0,
        StrictCategoryFilter::BeanSeller => (flags & FLAG_BEAN_SELLER) != 0,
        // Cafe strict mode: ONLY cafes/coffee shops, NEVER Resto Kopi unless pure specialty cafe!
        StrictCategoryFilter::Cafe => {
            (flags & (FLAG_SPECIALTY_CAFE | FLAG_COFFEE_SHOP)) != 0
                && (flags & FLAG_CAFE_RESTO) == 0
        }
        StrictCategoryFilter::SpecialtyCafe => (flags & FLAG_SPECIALTY_CAFE) != 0,
        StrictCategoryFilter::Resto => (flags & FLAG_CAFE_RESTO) != 0,
        StrictCategoryFilter::SlowBar => (flags & FLAG_SLOW_BAR_V60) != 0,
    }
}
