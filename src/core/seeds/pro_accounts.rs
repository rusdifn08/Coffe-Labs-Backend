use secrecy::SecretString;
use sqlx::PgPool;
use tracing::info;

use crate::core::{error::AppResult, security::hash_password};

const DEFAULT_PASS: &str = "coffeelabs123";

const PRO_USERS: &[(&str, &str, &str, &str, &str)] = &[
    ("Ropay", "Ahmad Ropai", "Kopi123#", "Ropayahmad@gmail.com", "085365434222"),
    ("Neeyyy04", "Ulul Azmi", "sawd14321", "4rtzemi@gmail.com", "081937295190"),
    ("Fadil05", "Fadil Saifullah", "coffe123456", "fadilsaifullah293@gmail.com", "082122312268"),
    ("@lifeshaaa_", "Nisa Siti Husna", "nisa2703", "nisasitihusna27@gmail.com", "088802029829"),
    ("Beater", "Beater", DEFAULT_PASS, "corpsus115@gmail.com", "088232202407"),
    ("Joko Raharjo", "Joko Raharjo", "sukasuka123", "jokoraharjo93@gmail.com", "082134622895"),
    ("adeabdurahman", "Ade Sufyan A", "abcd1234", "ade.abdurrachman@gmail.com", "081393325322"),
    ("wisnu", "Wisnu Ridho", "Pedesekali", "wisnuridho744@gmail.com", "082357316205"),
    ("Ahha_comeon", "Agni Ainurrofi", "1945", "agniainurrofi47@gmail.com", "089602059157"),
    ("allende46", "Rizki Alam Allende", "1sampesepuluh", "rizkiallende46@gmail.com", "085777290333"),
    ("faizal darma", "Faizal Darma Aji", "faizaldarmacoffee", "faizaldarma331@gmail.com", "089526242289"),
    ("shanks26", "Silmi Muhammad", "arabica26", "dammahumimlis2018@gmail.com", "+967 775 031 204"),
    ("bakoelkopi.jkt", "Miftahul Huda", DEFAULT_PASS, "bakoelkopi.jkt@gmail.com", "085117463365"),
    ("nakhodarizky", "Nakhoda Rizky", "nrps1144", "nakhodarizky1144@gmail.com", "082135844581"),
    ("Iam Zaa", "Khoirul Zar'i", DEFAULT_PASS, "Iam.zaa.88@gmail.com", "+62 851-1321-1962"),
    ("Amin", "Amin", "Amin110100", "aminajah782@gmail.com", "-"),
    ("muharul10", "Muh. Syahrul Kamarullah", "Palopo@01", "muh.arul@gmail.com", "085397837024"),
    ("Fadil Xau", "Fadil", "FadilXau", "madjatifarma@gmail.com", "089612312966"),
    ("ameecoffee", "Hendi Suhaedi", "123456", "ameecoffee@gmail.com", "08112122179"),
    ("riuss", "Riuss R.", "Riuss987", "riussbot@gmail.com", "081991287286"),
    ("@nzrjhr", "Nizar Jauhar", "Coffeelabs20", "niyaryhrd@gmail.com", "089625377755"),
    ("Totak", "Komang Krisnanda Sutapa", "totax123", "omingtotak87@gmail.com", "081238304163"),
    ("JapriLocalCoffee", "Panji Setyawan", "JAPRI_4j4", "japri.donk.yuk@gmail.com", "083866970831"),
    ("fadhilzhi_", "Fadhil Zhahiran Haniah", DEFAULT_PASS, "fadhil.f1512@gmail.com", "083119961804"),
    ("khaivieee", "Khairan Novie", "vR8!qZ#4mT@9Lp2X", "khairannovie03@sma.belajar.id", "087762498848"),
    ("mikejoe08", "Michael Johansen Panggabean", "exceler563", "mikepanggabean8@gmail.com", "081314079718"),
    ("pardiraya", "Pardi Soewarno", "nugoraya", "pardiraya@gmil.com", "081932266000"),
    ("Fadhil Kahfi", "Fadhil Kahfi Raechan", "837456234", "fadhilkahfi14@gmail.com", "082231553873"),
    ("bayu_aris", "Bayu Aris Rifai", DEFAULT_PASS, "bayuarisrifai@gmail.com", "087844624327"),
    ("doeasoedoet", "Wira Anggara Darmawan", "@#$123coffelabs", "soedoetdoea@gmail.com", "087792853286"),
];

pub async fn seed_registered_pro_accounts(pool: &PgPool) -> AppResult<()> {
    let mut created_count = 0_usize;

    for (idx, (username, full_name, password, email, wa)) in PRO_USERS.iter().enumerate() {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
            .bind(username)
            .fetch_one(pool)
            .await?;

        if !exists {
            let pwd = SecretString::from((*password).to_string());
            let password_hash = hash_password(&pwd)?;
            let avatar = format!("assets/images/characters/avatar_{}.png", (idx % 6) + 1);
            let bio = format!("Email: {} | WA: {}", email, wa);

            sqlx::query(
                r#"INSERT INTO users (
                    username, password_hash, full_name, role, status, tier,
                    bio, avatar_url, subscription_plan_id, subscription_payment_method, subscription_activated_at
                ) VALUES ($1, $2, $3, 'user', 'approved', 'pro', $4, $5, 'plan_pro', 'Manual Pro Activation', NOW())"#,
            )
            .bind(username)
            .bind(&password_hash)
            .bind(full_name)
            .bind(&bio)
            .bind(&avatar)
            .execute(pool)
            .await?;
            created_count += 1;
        } else {
            sqlx::query(
                r#"UPDATE users
                   SET status = 'approved',
                       tier = 'pro',
                       subscription_plan_id = 'plan_pro',
                       subscription_activated_at = COALESCE(subscription_activated_at, NOW())
                   WHERE username = $1 AND role <> 'admin'"#,
            )
            .bind(username)
            .execute(pool)
            .await?;
        }
    }

    if created_count > 0 {
        info!("☕ {} akun Pro baru dari data terdaftar berhasil di-seed ke NeonDB", created_count);
    }
    Ok(())
}
