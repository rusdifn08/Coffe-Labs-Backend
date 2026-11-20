use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::modules::live_brew::mirror_dto::{
    BrandedSeatResolutionDto, LiveSessionMirrorSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveBrewMirrorPhase {
    SeatIdleBranded,
    PosOrderQueued,
    BrewerSelectingRecipe,
    BrewerSelectingBean,
    PreBrewCalibrated,
    BrewingActive,
    BrewingPaused,
    ServingReadyPassport,
    Archived,
}

impl LiveBrewMirrorPhase {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::SeatIdleBranded => "SEAT_IDLE_BRANDED",
            Self::PosOrderQueued => "POS_ORDER_QUEUED",
            Self::BrewerSelectingRecipe => "BREWER_SELECTING_RECIPE",
            Self::BrewerSelectingBean => "BREWER_SELECTING_BEAN",
            Self::PreBrewCalibrated => "PRE_BREW_CALIBRATED",
            Self::BrewingActive => "BREWING_ACTIVE",
            Self::BrewingPaused => "BREWING_PAUSED",
            Self::ServingReadyPassport => "SERVING_READY_PASSPORT",
            Self::Archived => "ARCHIVED",
        }
    }

    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "SEAT_IDLE_BRANDED" => Self::SeatIdleBranded,
            "POS_ORDER_QUEUED" => Self::PosOrderQueued,
            "BREWER_SELECTING_BEAN" => Self::BrewerSelectingBean,
            "PRE_BREW_CALIBRATED" => Self::PreBrewCalibrated,
            "BREWING_ACTIVE" => Self::BrewingActive,
            "BREWING_PAUSED" => Self::BrewingPaused,
            "SERVING_READY_PASSPORT" => Self::ServingReadyPassport,
            "ARCHIVED" => Self::Archived,
            _ => Self::BrewerSelectingRecipe,
        }
    }

    pub fn label_id(self) -> &'static str {
        match self {
            Self::SeatIdleBranded => "Siap Menerima Pesanan Kopi",
            Self::PosOrderQueued => "Pesanan Masuk Antrean Brew Bar",
            Self::BrewerSelectingRecipe => "Barista Memilih Metode & Resep",
            Self::BrewerSelectingBean => "Barista Memilih Biji Kopi",
            Self::PreBrewCalibrated => "Resep & Kopi Siap — Menunggu Tuangan",
            Self::BrewingActive => "Sedang Diseduh Live (1:1 Sync)",
            Self::BrewingPaused => "Seduhan Dijeda Sementara",
            Self::ServingReadyPassport => "Siap Disajikan! Digital Cup Passport",
            Self::Archived => "Sesi Seduhan Selesai",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveTimerStatus {
    Idle,
    Running,
    Paused,
    Finished,
}

impl LiveTimerStatus {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Running => "RUNNING",
            Self::Paused => "PAUSED",
            Self::Finished => "FINISHED",
        }
    }

    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "RUNNING" => Self::Running,
            "PAUSED" => Self::Paused,
            "FINISHED" => Self::Finished,
            _ => Self::Idle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CafeSeatKind {
    Meja,
    KursiBar,
    VipRoom,
    Outdoor,
    TakeawayCounter,
}

impl CafeSeatKind {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Meja => "MEJA",
            Self::KursiBar => "KURSI_BAR",
            Self::VipRoom => "VIP_ROOM",
            Self::Outdoor => "OUTDOOR",
            Self::TakeawayCounter => "TAKEAWAY_COUNTER",
        }
    }

    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "KURSI_BAR" => Self::KursiBar,
            "VIP_ROOM" => Self::VipRoom,
            "OUTDOOR" => Self::Outdoor,
            "TAKEAWAY_COUNTER" => Self::TakeawayCounter,
            _ => Self::Meja,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "cmd", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrewerMirrorCommand {
    NtpPing {
        client_send_ms: i64,
    },
    SelectRecipePreview {
        method_id: String,
        method_name: String,
        recipe_id: Option<Uuid>,
        recipe_title: String,
        recipe_snapshot_json: Value,
    },
    SelectBeanPreview {
        bean_id: Option<Uuid>,
        bean_snapshot_json: Value,
    },
    ConfirmReadyToBrew,
    TimerStartOrResume,
    TimerPause,
    TimerReset,
    AdvanceStepOrTelemetry {
        current_step_index: i16,
        weight_g: f32,
        flow_rate_ml_s: f32,
        temp_c: f32,
    },
    CompleteAndServe {
        final_weight_g: f32,
        #[allow(dead_code)]
        accuracy_score_pct: Option<f32>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum MirrorDownstreamFrame {
    NtpPong {
        client_send_ms: i64,
        server_recv_ms: i64,
        server_send_ms: i64,
    },
    MirrorSnapshot(LiveSessionMirrorSnapshot),
    SeatPosUpdate(BrandedSeatResolutionDto),
    CheerBurst {
        cheer_kind: String,
        total_cheers: i32,
    },
    ViewerCountUpdated {
        active_viewers: i32,
    },
}
