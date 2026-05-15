use openview_core::SandboxProfile;

#[test]
fn filesystem_policy_allows_only_declared_read_and_write_roots() {
    let sandbox = SandboxProfile::locked_down()
        .allow_workspace_read("/workspace")
        .allow_workspace_write("/workspace/out");

    assert!(sandbox.can_read_path("/workspace"));
    assert!(sandbox.can_read_path("/workspace/src/lib.rs"));
    assert!(!sandbox.can_read_path("/workspace2/src/lib.rs"));
    assert!(!sandbox.can_read_path("/workspace/../secrets.txt"));

    assert!(sandbox.can_write_path("/workspace/out/report.txt"));
    assert!(!sandbox.can_write_path("/workspace/src/lib.rs"));
    assert!(!sandbox.can_write_path("/workspace/out/../src/lib.rs"));
}

#[test]
fn filesystem_deny_roots_override_less_specific_allow_roots() {
    let mut sandbox = SandboxProfile::locked_down()
        .allow_workspace_read("/workspace")
        .allow_workspace_write("/workspace");

    sandbox
        .filesystem
        .deny_roots
        .insert("/workspace/private".to_string());

    assert!(sandbox.can_read_path("/workspace/public/notes.md"));
    assert!(sandbox.can_write_path("/workspace/public/notes.md"));
    assert!(!sandbox.can_read_path("/workspace/private/token.txt"));
    assert!(!sandbox.can_write_path("/workspace/private/token.txt"));
}

#[test]
fn process_policy_requires_exact_allowed_command_identity() {
    let sandbox = SandboxProfile::locked_down().allow_command("git");

    assert!(sandbox.can_execute_command("git"));
    assert!(!sandbox.can_execute_command("git status"));
    assert!(!sandbox.can_execute_command("/usr/bin/git"));
    assert!(!sandbox.can_execute_command("bash"));
}

#[test]
fn network_policy_requires_allowed_host_when_network_is_enabled() {
    let sandbox = SandboxProfile::locked_down().allow_network_host("API.EXAMPLE.COM");

    assert!(sandbox.can_access_network_host("api.example.com"));
    assert!(sandbox.can_access_network_host("api.example.com."));
    assert!(!sandbox.can_access_network_host("sub.api.example.com"));
    assert!(!sandbox.can_access_network_host("api.example.com:443"));
    assert!(!sandbox
        .deny_network()
        .can_access_network_host("api.example.com"));
}

#[test]
fn secrets_are_referenced_by_allowed_name_only() {
    let sandbox = SandboxProfile::locked_down().allow_secret_name("OPENAI_API_KEY");

    assert!(sandbox.can_reference_secret_name("OPENAI_API_KEY"));
    assert!(!sandbox.can_reference_secret_name("OPENAI_API_KEY=sk-test"));
    assert!(!sandbox.can_reference_secret_name("OPENAI API KEY"));
    assert!(!sandbox.can_reference_secret_name("sk-test"));
}
