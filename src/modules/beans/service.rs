use sqlx::PgPool;

use crate::{
    core::error::AppResult,
    modules::beans::{
        dto::{BeanCatalogQuery, DialInCalculationRequest, DialInRecommendationResponse},
        models::{CuratedBeanRow, DialInPourStep},
        repository,
    },
};

pub async fn get_catalog(pool: &PgPool, query: BeanCatalogQuery) -> AppResult<Vec<CuratedBeanRow>> {
    repository::query_beans(
        pool,
        query.q.as_deref(),
        query.roastery.as_deref(),
        query.city.as_deref(),
        query.process.as_deref(),
    )
    .await
}

pub fn calculate_dial_in(req: DialInCalculationRequest) -> DialInRecommendationResponse {
    let dose = req.coffee_grams.unwrap_or(15.0).clamp(8.0, 50.0);
    let process = req
        .process
        .unwrap_or_else(|| "Washed".to_string())
        .to_lowercase();
    let roast = req
        .roast_level
        .unwrap_or_else(|| "Light-Medium".to_string())
        .to_lowercase();
    let dripper = req.dripper.unwrap_or_else(|| "Hario V60 02".to_string());
    let goal = req.flavor_goal.unwrap_or_else(|| "Sweetness & Clarity".to_string());

    let is_anaerobic_or_natural = process.contains("natural")
        || process.contains("anaerobic")
        || process.contains("wine")
        || process.contains("carbonic");

    let mut temp = if roast.contains("dark") {
        87
    } else if roast.contains("medium") {
        90
    } else {
        92
    };
    if is_anaerobic_or_natural {
        temp -= 2;
    }

    let ratio_mult = if goal.to_lowercase().contains("body") {
        14.5
    } else if goal.to_lowercase().contains("clarity") {
        16.0
    } else {
        15.0
    };
    let total_water = (dose * ratio_mult).round();

    let (microns, clicks, label) = if is_anaerobic_or_natural {
        (720, 24, "Medium-Coarse (24 Klik Comandante / 720µm)")
    } else {
        (660, 22, "Medium (22 Klik Comandante / 660µm)")
    };

    let bloom = (dose * 3.0).round();
    let remaining = (total_water - bloom).max(0.0);
    let p2 = (remaining * 0.40).round();
    let p3 = (remaining * 0.35).round();
    let p4 = remaining - p2 - p3;

    let pour_schedule = vec![
        DialInPourStep {
            step_order: 1,
            title: "Degassing Bloom".to_string(),
            water_added_grams: bloom,
            cumulative_water_grams: bloom,
            duration_seconds: 40,
            instruction: format!("Tuang {:.0}g air melingkar merata, swirl lembut.", bloom),
        },
        DialInPourStep {
            step_order: 2,
            title: "Acidity & Brightness Pour".to_string(),
            water_added_grams: p2,
            cumulative_water_grams: bloom + p2,
            duration_seconds: 35,
            instruction: "Tuang spiral dari tengah ke luar dengan flow stabil.".to_string(),
        },
        DialInPourStep {
            step_order: 3,
            title: "Sweetness Matrix Pour".to_string(),
            water_added_grams: p3,
            cumulative_water_grams: bloom + p2 + p3,
            duration_seconds: 35,
            instruction: "Tuang lebih lambat untuk membangun sweetness.".to_string(),
        },
        DialInPourStep {
            step_order: 4,
            title: "Center Clean Finish".to_string(),
            water_added_grams: p4,
            cumulative_water_grams: total_water,
            duration_seconds: 40,
            instruction: "Tuang titik tengah hingga target air tercapai.".to_string(),
        },
    ];

    DialInRecommendationResponse {
        bean_name: req.bean_name,
        dripper,
        flavor_goal: goal,
        coffee_grams: dose,
        total_water_grams: total_water,
        ratio: format!("1:{:.1}", ratio_mult),
        water_temp_celsius: temp,
        grind_size_label: label.to_string(),
        grind_microns: microns,
        comandante_clicks: clicks,
        target_tds_percent: 1.38,
        target_extraction_yield_percent: 20.4,
        total_time_seconds: 150,
        barista_note: if is_anaerobic_or_natural {
            "Suhu sedikit diturunkan & gilingan sedikit dilonggarkan untuk mencegah over-fermentasi dan mengangkat karakter buah tropis.".to_string()
        } else {
            "Suhu optimal & turbulensi spiral bertahap untuk memaksimalkan kejernihan rasa (clarity) serta manis alami.".to_string()
        },
        pour_schedule,
    }
}
