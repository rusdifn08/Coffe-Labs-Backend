/// Leksikon moderasi kata kasar multibahasa (15+ bahasa + anti-spam judi online).
/// Dipisahkan menjadi:
/// 1) `SEVERE_SUBSTRING_TERMS`: kata kasar/umpatan tegas yang diblokir sebagai token, substring,
///    maupun variasi spasi/simbol tersembunyi.
/// 2) `EXACT_TOKEN_TERMS`: kata pendek/ambigu yang HANYA diblokir bila berdiri sebagai kata utuh.
/// 3) `SAFE_COFFEE_WHITELIST`: kosakata kopi & umum yang dilindungi agar 0% false positive.

pub const SEVERE_SUBSTRING_TERMS: &[&str] = &[
    // Indonesia & bahasa daerah (Jawa, Sunda, Batak, Melayu, Minang, Makassar, Betawi)
    "bangsat", "bangsad", "kontol", "kntol", "kntl", "memek", "mmek", "ngentot", "ngntot",
    "entot", "jancok", "jancuk", "dancok", "dancuk", "jnck", "jncok", "anjing", "anjink",
    "anjir", "anjrit", "bajingan", "keparat", "brengsek", "brengsex", "pepek", "jembut",
    "perek", "lonte", "bencong", "pukimak", "pukima", "pantek", "panteq", "kimak", "lancau",
    "lanjiao", "tempik", "turuk", "pelacur", "pecun", "sundal", "cibai", "chibai", "jablay",
    "tetek", "ngocok", "colmek", "bokep", "porno", "sange", "ngewe", "ewean", "goblog",
    "goblok", "tolol", "bego", "idiot", "laknat", "biadab", "kampret", "bangke", "taek",
    "telek", "matamu", "ndasmu", "raimu", "cangkemmu", "jahanam", "bodat", "sundala",
    "telaso", "bacot", "bacod", "bacotan", "cocot", "geblek", "somplak", "sinting",
    // English
    "fuck", "fucker", "fucking", "motherfucker", "mofo", "fuxk", "shit", "bullshit",
    "shitty", "bitch", "bitches", "cunt", "dickhead", "cocksucker", "asshole", "arsehole",
    "bastard", "whore", "slut", "wanker", "twat", "nigger", "nigga", "faggot", "retard",
    "blowjob", "handjob", "pornhub", "xvideos", "xnxx",
    // Filipino / Tagalog
    "putangina", "tangina", "gago", "ulol", "tarantado", "pakyu", "kupal", "punyeta",
    "hinayupak", "kantot",
    // Spanish
    "hijodeputa", "mierda", "pendejo", "cabron", "chinga", "chingada", "chingar", "verga",
    "pinche", "joder", "cojones", "maricon", "culero", "carajo", "gilipollas", "malparido",
    "gonorrea",
    // Portuguese
    "porra", "caralho", "fodase", "buceta", "arrombado", "desgracado", "viado", "cuzao",
    "merda",
    // French
    "merde", "putain", "salope", "connard", "connasse", "encule", "enculer", "niquer",
    "bordel", "batard", "couille",
    // German
    "scheisse", "arschloch", "hurensohn", "ficken", "fotze", "wichser", "schlampe",
    "missgeburt", "verpiss", "drecksau",
    // Italian
    "stronzo", "vaffanculo", "fanculo", "cazzo", "coglione", "puttana", "troia", "bastardo",
    // Russian (Latin + Cyrillic)
    "blyat", "blyad", "cyka", "pizdec", "pizda", "nahui", "ebat", "yebat", "mudak",
    "pidor", "pidoras", "блядь", "блять", "сука", "пиздец", "пизда", "хуй", "ебать",
    "мудак", "пидор",
    // Korean (Romanized + Hangul)
    "shibal", "ssibal", "sibal", "gaesaekki", "saekki", "byeongsin", "byungsin", "jiral",
    "씨발", "시발", "개새끼", "새끼", "병신", "지랄", "미친놈", "미친년", "좆",
    // Japanese (Romanized + Kana/Kanji)
    "kusoyaro", "chikusho", "bakayaro", "chinko", "manko", "yariman", "くそ", "クソ",
    "糞", "死ね", "ちんこ", "まんこ", "ばかやろう", "バカヤロウ", "クソ野郎",
    // Chinese / Mandarin / Cantonese / Hokkien
    "caonima", "shabi", "nimabi", "tamade", "gundan", "diulei", "pokgai", "kanina",
    "knnbccb", "操你妈", "草泥马", "傻逼", "煞笔", "妈逼", "他妈的", "滚蛋", "屌你", "扑街",
    // Arabic (Romanized + Arabic script)
    "sharmouta", "sharmuta", "kusomak", "kosomak", "manyouk", "شرموطة", "كس امك",
    "كسمك", "خرا", "ابن الكلب", "منيوك", "قحبة",
    // Hindi / Urdu (Romanized + Devanagari)
    "madarchod", "bhenchod", "behenchod", "chutiya", "chutiye", "bhosdike", "bhosdi",
    "harami", "haramzada", "gandu", "randi", "मादरचोद", "बहनचोद", "चूतिया", "भोसड़ीके",
    "हरामी", "गांडू", "रंडी",
    // Turkish, Polish, Dutch
    "orospu", "siktir", "amcik", "yarrak", "pezevenk", "kurwa", "pierdol", "spierdalaj",
    "klootzak",
    // Gambling / scam spam Indonesia
    "slot gacor", "judi online", "togel online", "situs gacor", "maxwin88", "zeus88",
    "slot88", "jp maxwin", "depo slot",
];

pub const EXACT_TOKEN_TERMS: &[&str] = &[
    "asu", "cok", "cuk", "fck", "fuk", "fuq", "fk", "fu", "sht", "btch", "cnt",
    "bgst", "bgsd", "ajg", "anjg", "mmk", "ktl", "ntot", "ngtd", "tai", "tae",
    "babi", "dick", "cock", "pussy", "piss", "ass", "cum", "fag", "hoe", "wtf",
    "stfu", "puta", "puto", "foda", "foder", "kuso", "khara", "hui", "khuy",
    "coli", "ewe", "itil", "porn", "nude", "knn", "chuj", "hoer",
];

pub const SAFE_COFFEE_WHITELIST: &[&str] = &[
    "espresso", "classic", "class", "passion", "assessment", "assistant", "associate",
    "peacock", "hancock", "cockpit", "title", "subtitle", "scunthorpe", "masuk", "kasus",
    "kasur", "asuransi", "basuh", "titik", "titisan", "kocok", "mengocok", "cocok",
    "analisis", "analisa", "memekarkan", "mekar", "bloom", "blooming", "pantai",
    "tangkai", "bintang", "suka", "suku", "sukses", "bitter", "bitterness", "arabica",
    "robusta", "kintamani", "bajawa", "toraja", "gayo", "mandheling", "kerinci",
    "puntang", "ciwidey", "garut", "temanggung", "wamena", "papua", "flores", "halu",
    "shining", "sunshine", "shine", "pic", "pics", "picture", "bite", "bobo", "tangga",
];

/// Pola wildcard sensor manual seperti `f*ck`, `k*nt*l`, `b*ngs*t`, `sh*t`, `m*m*k`.
pub const WILDCARD_PATTERNS: &[(&str, &str)] = &[
    ("f", "ck"),
    ("sh", "t"),
    ("b", "tch"),
    ("k", "ntol"),
    ("knt", "l"),
    ("b", "ngsat"),
    ("m", "mek"),
    ("ng", "ntot"),
    ("j", "ncok"),
    ("j", "ncuk"),
    ("a", "jing"),
    ("anj", "ng"),
];
