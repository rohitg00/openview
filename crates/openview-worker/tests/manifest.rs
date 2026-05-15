use openview_worker::{git_worktree_worker_manifest, runtime_manifest_json};

#[test]
fn worker_crate_exposes_git_worktree_manifest_for_cli_use() {
    let manifest = git_worktree_worker_manifest();
    let json = runtime_manifest_json(&manifest).expect("manifest serializes");

    assert_eq!(manifest.name, "git.worktree");
    assert!(json.contains("git.worktree::list"));
    assert!(json.contains("git.worktree::create"));
}
