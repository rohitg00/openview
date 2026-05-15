use openview_worker::{
    git_worktree_worker_manifest, runtime_manifest_json, terminal_pty_worker_manifest,
};

#[test]
fn worker_crate_exposes_terminal_pty_manifest_for_cli_use() {
    let manifest = terminal_pty_worker_manifest();
    let json = runtime_manifest_json(&manifest).expect("manifest serializes");

    assert_eq!(manifest.name, "terminal.pty");
    assert!(json.contains("terminal.pty::spawn"));
    assert!(json.contains("terminal.pty::read"));
    assert!(json.contains("terminal.pty::resize"));
}

#[test]
fn worker_crate_exposes_git_worktree_manifest_for_cli_use() {
    let manifest = git_worktree_worker_manifest();
    let json = runtime_manifest_json(&manifest).expect("manifest serializes");

    assert_eq!(manifest.name, "git.worktree");
    assert!(json.contains("git.worktree::list"));
    assert!(json.contains("git.worktree::create"));
}
