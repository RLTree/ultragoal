use std::path::Path;

const REQUIRED_IMAGES: &[&str] = &[
    "victoriametrics/victoria-metrics:v1.146.0",
    "victoriametrics/victoria-logs:v1.51.0",
    "victoriametrics/victoria-traces:v0.8.0",
    "otel/opentelemetry-collector-contrib:0.130.0",
    "timberio/vector:0.47.0-debian",
    "grafana/grafana:12.0.2",
];

pub(super) fn check(root: &Path, out: &mut Vec<String>) {
    let text =
        std::fs::read_to_string(root.join("dev/observability/compose.yml")).unwrap_or_default();
    for image in REQUIRED_IMAGES {
        if !text.contains(image) {
            out.push(format!("observability_compose_missing_image:{image}"));
        }
    }
    if text.contains(":latest") {
        out.push("observability_compose_latest_image_forbidden".to_string());
    }
    if text.contains("\"0.0.0.0:") || text.contains("'0.0.0.0:") {
        out.push("observability_compose_public_port_binding".to_string());
    }
    check_loopback(&text, out);
    check_health(&text, out);
    if !text.contains("-retentionPeriod=2d") {
        out.push("observability_compose_missing_bounded_retention".to_string());
    }
}

fn check_loopback(text: &str, out: &mut Vec<String>) {
    for needle in [
        "127.0.0.1:8428",
        "127.0.0.1:9428",
        "127.0.0.1:10428",
        "127.0.0.1:3009",
    ] {
        if !text.contains(needle) {
            out.push(format!(
                "observability_compose_missing_loopback_binding:{needle}"
            ));
        }
    }
}

fn check_health(text: &str, out: &mut Vec<String>) {
    for service in [
        "victoriametrics",
        "victorialogs",
        "victoriatraces",
        "otel-collector",
        "vector",
        "grafana",
    ] {
        if !text.contains(&format!("  {service}:")) || !text.contains("healthcheck:") {
            out.push(format!(
                "observability_compose_missing_healthcheck:{service}"
            ));
        }
    }
}
