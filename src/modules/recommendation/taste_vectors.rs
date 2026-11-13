pub const TASTE_DIM: usize = 16;
pub const TASTE_EMA_ALPHA: f32 = 0.25;

pub const TASTE_DIMENSION_LABELS: [&str; TASTE_DIM] = [
    "Citrus & Bergamot",
    "Berry & Hibiscus",
    "Floral & Jasmine",
    "Chocolate & Caramel",
    "Tropical & Stone Fruit",
    "Washed Clarity",
    "Natural & Anaerobic",
    "Honey & Syrupy",
    "Ethiopia & Kenya",
    "Colombia & Geisha",
    "Nusantara Single Origin",
    "Light Filter Roast",
    "Balanced Omni Roast",
    "V60 & Pour-Over",
    "Espresso & AeroPress",
    "Experimental Brew",
];

const DIMENSION_KEYWORDS: [&[&str]; TASTE_DIM] = [
    &["citrus", "bergamot", "orange", "lemon", "grapefruit", "tangerine", "lime", "jeruk"],
    &["berry", "strawberry", "blueberry", "raspberry", "hibiscus", "fruit", "berrys", "kismis"],
    &["floral", "jasmine", "blossom", "rose", "tea", "chamomile", "bunga", "teh"],
    &["chocolate", "cocoa", "caramel", "nutty", "hazelnut", "brown sugar", "cokelat", "kacang"],
    &["peach", "apricot", "plum", "mango", "tropical", "pineapple", "mangga", "nanas"],
    &["washed", "clean", "clarity", "crisp", "bersih", "full wash"],
    &["natural", "anaerobic", "carbonic", "ferment", "winey", "boozy", "funky", "koji"],
    &["honey", "pulped", "syrupy", "creamy", "madu", "manis"],
    &["ethiopia", "kenya", "yirgacheffe", "guji", "sidamo", "kirinyaga", "africa"],
    &["colombia", "panama", "geisha", "gesha", "pink bourbon", "costa rica", "huila"],
    &["indonesia", "gayo", "toraja", "java", "papua", "bajawa", "kintamani", "temanggung", "garut", "flores"],
    &["light roast", "nordic", "filter", "light"],
    &["medium", "omni", "balanced", "seimbang"],
    &["v60", "pour-over", "pourover", "pour over", "origami", "kalita", "chemex", "konis", "switch"],
    &["espresso", "aeropress", "moka", "crema", "shot", "piccolo", "flat white"],
    &["cold brew", "french press", "immersion", "syphon", "competition", "wbrc", "resep"],
];

pub fn l2_normalize(vec: &mut [f32; TASTE_DIM]) {
    let norm_sq: f32 = vec.iter().map(|v| v * v).sum();
    let norm = norm_sq.sqrt();
    if norm > 1e-6 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}

pub fn neutral_taste_vector() -> [f32; TASTE_DIM] {
    let mut v = [0.25f32; TASTE_DIM];
    l2_normalize(&mut v);
    v
}

pub fn vec_from_slice(slice: &[f32]) -> [f32; TASTE_DIM] {
    let mut out = [0.0f32; TASTE_DIM];
    for (idx, val) in slice.iter().take(TASTE_DIM).enumerate() {
        if val.is_finite() && *val > 0.0 {
            out[idx] = *val;
        }
    }
    if out.iter().map(|v| v * v).sum::<f32>() <= 1e-6 {
        return neutral_taste_vector();
    }
    l2_normalize(&mut out);
    out
}

pub fn extract_16d_vector_from_text(raw: &str) -> [f32; TASTE_DIM] {
    let lower = raw.to_ascii_lowercase();
    let mut vec = [0.0f32; TASTE_DIM];
    let mut matched = false;
    for (idx, keywords) in DIMENSION_KEYWORDS.iter().enumerate() {
        for kw in *keywords {
            if lower.contains(kw) {
                vec[idx] += 1.0;
                matched = true;
            }
        }
    }
    if matched {
        l2_normalize(&mut vec);
    }
    vec
}

pub fn cosine_similarity(a: &[f32; TASTE_DIM], b: &[f32; TASTE_DIM]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|y| y * y).sum::<f32>().sqrt();
    if norm_a <= 1e-6 || norm_b <= 1e-6 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(0.0, 1.0)
}

pub fn update_ema_l2(
    current: &[f32; TASTE_DIM],
    signal: &[f32; TASTE_DIM],
    alpha: f32,
) -> [f32; TASTE_DIM] {
    if signal.iter().map(|x| x * x).sum::<f32>().sqrt() <= 1e-6 {
        return *current;
    }
    let a = alpha.clamp(0.01, 0.9);
    let mut next = [0.0f32; TASTE_DIM];
    for i in 0..TASTE_DIM {
        next[i] = (1.0 - a) * current[i] + a * signal[i];
    }
    l2_normalize(&mut next);
    next
}
