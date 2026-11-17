use crate::{
    core::rabbitmq::{
        topology::{RabbitMqEventEnvelope, DEFAULT_EXCHANGE},
        RabbitMqClient,
    },
    modules::barista_toolkit::dto::LmsCompletionEvent,
};
use uuid::Uuid;

/// Menerbitkan event kelulusan modul LMS ke exchange RabbitMQ secara non-blocking
/// agar worker gamifikasi & sertifikat Cloudinary dapat memproses asinkron.
pub fn publish_lms_lesson_completed(
    rabbitmq: &RabbitMqClient,
    user_id: Uuid,
    lesson_id: Uuid,
    course_id: Uuid,
    xp_awarded: i32,
    completion_pct: i16,
    quiz_score_pct: Option<i16>,
) {
    let payload = LmsCompletionEvent {
        user_id,
        lesson_id,
        course_id,
        xp_awarded,
        completion_pct,
        quiz_score_pct,
        certificate_public_id: format!("coffeelabs/certificates/{user_id}_{course_id}.pdf"),
    };
    let envelope = RabbitMqEventEnvelope::new(
        "LMS_LESSON_COMPLETED",
        "barista_toolkit",
        serde_json::json!(payload),
    );
    rabbitmq.publish_json(DEFAULT_EXCHANGE, "events.lms.completed", &envelope);
}
