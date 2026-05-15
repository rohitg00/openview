use std::path::Path;
use std::process::Command;

#[test]
fn live_iii_runtime_e2e_script_runs_when_enabled() {
    if std::env::var_os("OPENVIEW_RUN_LIVE_III_E2E").is_none() {
        eprintln!("skipping live iii runtime E2E; set OPENVIEW_RUN_LIVE_III_E2E=1 to run it");
        return;
    }

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = repo_root.join("scripts/e2e_iii_runtime.sh");

    let status = Command::new("bash")
        .arg(&script)
        .status()
        .unwrap_or_else(|err| panic!("failed to launch {}: {err}", script.display()));

    assert!(status.success(), "{} failed", script.display());
}
