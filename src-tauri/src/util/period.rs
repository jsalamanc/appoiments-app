use time::{Duration, OffsetDateTime};

/// Puerto de `getPeriodRange` (src/lib/reports-server.ts): devuelve el
/// timestamp ISO-8601 de inicio del período solicitado, hasta "ahora".
pub fn period_from(period: &str) -> String {
    let now = OffsetDateTime::now_utc();
    let from = match period {
        "day" => now - Duration::hours(24),
        "week" => now - Duration::days(7),
        "month" => now - Duration::days(30),
        _ => now - Duration::hours(24),
    };
    from.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}
