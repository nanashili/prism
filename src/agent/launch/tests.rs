use super::*;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

fn executable(directory: &Path, name: &str, permissions: u32) -> PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, "#!/bin/sh\nexit 47\n").expect("write executable fixture");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(permissions))
        .expect("set fixture permissions");
    path
}

fn entry(id: &str, distribution: Value) -> Entry {
    Entry {
        id: id.into(),
        name: id.into(),
        description: String::new(),
        version: "detection-test".into(),
        icon: String::new(),
        distribution,
    }
}

#[test]
fn local_agents_use_executable_paths_and_keep_their_protocol_arguments() {
    let first = tempfile::tempdir().expect("first executable directory");
    let second = tempfile::tempdir().expect("second executable directory");
    let local = LocalAgents {
        paths: vec![first.path().into(), second.path().into()],
    };
    let platform = super::super::catalog::platform();
    let agents = [
        (
            "copilot",
            entry(
                "github-copilot-cli",
                json!({
                    "npx": {
                        "package": "@github/copilot@1.0.103",
                        "args": ["--acp"],
                        "env": {"AGENT_TEST_MODE": "stdio"}
                    }
                }),
            ),
            "--acp",
        ),
        (
            "cursor-agent",
            entry(
                "cursor",
                json!({
                    "binary": {platform.as_str(): {
                        "cmd": "./dist-package/cursor-agent",
                        "args": ["acp"],
                        "env": {"AGENT_TEST_MODE": "stdio"}
                    }}
                }),
            ),
            "acp",
        ),
    ];

    for (command, agent, argument) in agents {
        assert!(
            !local.installed(&agent),
            "An absent CLI must not be installed"
        );
        executable(first.path(), command, 0o644);
        assert!(
            !local.installed(&agent),
            "A plain file must not be installed"
        );
        let expected = executable(second.path(), command, 0o755);

        assert!(local.installed(&agent));
        let launch = local.resolve(&agent).expect("use the local executable");
        assert_eq!(Path::new(&launch.command), expected);
        assert_eq!(launch.args, [argument]);
        assert_eq!(
            launch.env.get("AGENT_TEST_MODE").map(String::as_str),
            Some("stdio")
        );

        let preferred = executable(first.path(), command, 0o755);
        assert_eq!(
            Path::new(
                &local
                    .resolve(&agent)
                    .expect("resolve the preferred executable")
                    .command
            ),
            preferred
        );
    }

    executable(first.path(), "opencode", 0o755);
    let other_version = entry(
        "opencode-v2",
        json!({
            "binary": {platform: {"cmd": "./opencode", "args": ["acp"]}}
        }),
    );
    assert!(
        !local.installed(&other_version),
        "One CLI must not make a separate registry version appear installed"
    );
    assert!(local.resolve(&other_version).is_none());
}

#[test]
fn a_provider_cli_is_detected_without_mistaking_it_for_its_adapter() {
    let directory = tempfile::tempdir().expect("executable directory");
    let local = LocalAgents {
        paths: vec![directory.path().into()],
    };
    let claude = entry(
        "claude-acp",
        json!({
            "npx": {"package": "@agentclientprotocol/claude-agent-acp"}
        }),
    );
    executable(directory.path(), "claude", 0o755);

    assert!(local.installed(&claude));
    assert!(
        local.resolve(&claude).is_none(),
        "The provider CLI cannot speak ACP by itself"
    );

    let adapter = executable(directory.path(), "claude-agent-acp", 0o755);
    assert_eq!(
        Path::new(
            &local
                .resolve(&claude)
                .expect("resolve the ACP adapter")
                .command
        ),
        adapter
    );

    let custom_path = executable(directory.path(), "my-agent", 0o755);
    let custom = entry(
        "custom",
        json!({
            "command": custom_path,
            "args": ["--stdio", "two words"],
            "env": {"AGENT_TEST_MODE": "custom"}
        }),
    );
    let launch = local
        .resolve(&custom)
        .expect("use an explicit custom executable");
    assert!(local.installed(&custom));
    assert_eq!(launch.args, ["--stdio", "two words"]);
    assert_eq!(launch.env["AGENT_TEST_MODE"], "custom");
}
