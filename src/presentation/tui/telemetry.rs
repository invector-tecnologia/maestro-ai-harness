use super::*;

pub(super) struct TelemetryStore;

impl TelemetryStore {
    pub(super) fn record(event: &str, detail: Option<&str>) -> Result<()> {
        if !telemetry_enabled() {
            return Ok(());
        }

        let path = telemetry_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let sanitized = detail.unwrap_or("").replace('"', "'");
        writeln!(
            file,
            "{{\"ts\":{},\"event\":\"{}\",\"detail\":\"{}\"}}",
            ts, event, sanitized
        )?;
        Ok(())
    }
}

fn telemetry_enabled() -> bool {
    matches!(
        std::env::var("MAESTRO_TELEMETRY").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE")
    )
}

fn telemetry_file_path() -> Result<PathBuf> {
    Ok(workspace_maestro_dir()?.join("telemetry_onboarding.jsonl"))
}

fn workspace_maestro_dir() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join("maestro"))
}
