use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::live_brew::dto::{CatchupCurvePoint, DigitalCupPassportDto, LiveBrewSnapshotDto};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "live_brew_stage", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LiveBrewStage {
    QueuedAssigned,
    GrindingDose,
    FilterRinsePreheat,
    BloomingDegassing,
    ActivePouring,
    DrawdownSwirl,
    ReadyToServe,
    CompletedArchived,
}

impl LiveBrewStage {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::QueuedAssigned => "QUEUED_ASSIGNED",
            Self::GrindingDose => "GRINDING_DOSE",
            Self::FilterRinsePreheat => "FILTER_RINSE_PREHEAT",
            Self::BloomingDegassing => "BLOOMING_DEGASSING",
            Self::ActivePouring => "ACTIVE_POURING",
            Self::DrawdownSwirl => "DRAWDOWN_SWIRL",
            Self::ReadyToServe => "READY_TO_SERVE",
            Self::CompletedArchived => "COMPLETED_ARCHIVED",
        }
    }

    pub fn stage_label_id(self) -> &'static str {
        match self {
            Self::QueuedAssigned => "Pesanan Diterima di Brew Bar",
            Self::GrindingDose => "Menimbang & Menggiling Biji Kopi",
            Self::FilterRinsePreheat => "Membilas Filter & Pemanasan Server",
            Self::BloomingDegassing => "Tahap 1: Blooming & Pelepasan Gas CO2",
            Self::ActivePouring => "Tahap 2-3: Ekstraksi Aroma & Sweetness",
            Self::DrawdownSwirl => "Tahap Akhir: Gentle Swirl & Drawdown",
            Self::ReadyToServe => "Kopi Anda Siap Disajikan!",
            Self::CompletedArchived => "Sesi Seduhan Tersimpan di Arsip",
        }
    }

    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_uppercase().as_str() {
            "GRINDING_DOSE" | "GRINDING" => Self::GrindingDose,
            "FILTER_RINSE_PREHEAT" | "RINSE" => Self::FilterRinsePreheat,
            "BLOOMING_DEGASSING" | "BLOOMING" => Self::BloomingDegassing,
            "ACTIVE_POURING" | "POURING" => Self::ActivePouring,
            "DRAWDOWN_SWIRL" | "DRAWDOWN" => Self::DrawdownSwirl,
            "READY_TO_SERVE" | "READY" => Self::ReadyToServe,
            "COMPLETED_ARCHIVED" | "ARCHIVED" => Self::CompletedArchived,
            _ => Self::QueuedAssigned,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BaristaUpstreamFrame {
    StageTransition {
        stage: LiveBrewStage,
        step_index: u8,
        message: Option<String>,
        temp_c: f32,
    },
    ScaleTelemetryTick {
        elapsed_ms: u32,
        weight_g: f32,
        flow_rate_ml_s: f32,
        temp_c: f32,
    },
    VideoStreamStatus {
        is_active: bool,
        whep_url: Option<String>,
    },
    VideoKeyframe {
        jpeg_b64: String,
        ts_ms: u32,
    },
    FinishAndServe {
        final_weight_g: f32,
        total_duration_sec: u32,
        accuracy_score_pct: f32,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ViewerDownstreamEvent {
    InitialSessionSnapshot(LiveBrewSnapshotDto),
    StageChanged {
        session_id: Uuid,
        stage: LiveBrewStage,
        step_index: u8,
        stage_label_id: String,
        message: Option<String>,
        elapsed_ms: u32,
        temp_c: f32,
    },
    TelemetryTick(CatchupCurvePoint),
    CustomerCheer {
        cheer_kind: String,
        total_cheers: u32,
    },
    VideoStatusChanged {
        is_active: bool,
        whep_url: Option<String>,
    },
    VideoKeyframeRelayed {
        jpeg_b64: String,
        ts_ms: u32,
    },
    BrewCompletedPassport(DigitalCupPassportDto),
    TableSessionBound {
        table_number: String,
        static_address_code: String,
        snapshot: LiveBrewSnapshotDto,
    },
    TableWaitingForBrew {
        table_number: String,
        static_address_code: String,
        store_name: String,
        livebrew_url: String,
    },
}
