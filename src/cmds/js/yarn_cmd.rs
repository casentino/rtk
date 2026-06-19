//! Filters yarn output and auto-injects the "run" subcommand when appropriate.

use crate::core::runner;
use crate::core::utils::resolved_command;
use anyhow::Result;

/// Known yarn subcommands that should NOT get "run" injected.
const YARN_SUBCOMMANDS: &[&str] = &[
    "add",
    "audit",
    "autoclean",
    "bin",
    "cache",
    "check",
    "config",
    "create",
    "dedupe",
    "dlx",
    "exec",
    "explain",
    "help",
    "import",
    "info",
    "init",
    "install",
    "licenses",
    "link",
    "list",
    "login",
    "logout",
    "node",
    "npm",
    "outdated",
    "owner",
    "pack",
    "patch",
    "plugin",
    "publish",
    "remove",
    "run",
    "search",
    "set",
    "tag",
    "team",
    "unlink",
    "unplug",
    "up",
    "upgrade",
    "version",
    "versions",
    "why",
    "workspace",
    "workspaces",
];

pub fn run(args: &[String], verbose: u8, skip_env: bool) -> Result<i32> {
    let mut effective_args: Vec<String> = Vec::with_capacity(args.len() + 1);

    if needs_run_injection(args) {
        effective_args.push("run".to_string());
    }
    effective_args.extend_from_slice(args);

    let mut cmd = resolved_command("yarn");
    for arg in &effective_args {
        cmd.arg(arg);
    }

    if skip_env {
        cmd.env("SKIP_ENV_VALIDATION", "1");
    }

    let args_display = effective_args.join(" ");
    if verbose > 0 {
        eprintln!("Running: yarn {}", args_display);
    }

    runner::run_filtered(
        cmd,
        "yarn",
        &args_display,
        filter_yarn_output,
        runner::RunOptions::default(),
    )
}

fn needs_run_injection(args: &[String]) -> bool {
    let Some(first_arg) = args.first().map(|s| s.as_str()) else {
        return false;
    };

    !YARN_SUBCOMMANDS.contains(&first_arg) && !first_arg.starts_with('-')
}

/// Filter yarn run output: strip yarn wrapper banners, command echoes, and timing noise.
fn filter_yarn_output(output: &str) -> String {
    let mut result = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("yarn run v") {
            continue;
        }
        if trimmed.starts_with('$') {
            continue;
        }
        if trimmed.starts_with("Done in ") {
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        result.push(line.to_string());
    }

    if result.is_empty() {
        "ok".to_string()
    } else {
        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yarn_subcommand_routing() {
        fn needs_run(args: &[&str]) -> bool {
            needs_run_injection(
                &args
                    .iter()
                    .map(|arg| (*arg).to_string())
                    .collect::<Vec<_>>(),
            )
        }

        for subcmd in YARN_SUBCOMMANDS {
            assert!(
                !needs_run(&[subcmd]),
                "'yarn {}' should NOT inject 'run'",
                subcmd
            );
        }

        for script in &["build", "dev", "lint", "typecheck"] {
            assert!(
                needs_run(&[script]),
                "'yarn {}' SHOULD inject 'run'",
                script
            );
        }

        assert!(!needs_run(&[]));
        assert!(!needs_run(&["--version"]));
        assert!(!needs_run(&["-v"]));
        assert!(!needs_run(&["run", "build"]));
    }

    #[test]
    fn test_filter_yarn_output_strips_boilerplate() {
        let output = r#"
yarn run v1.22.22
$ next build
warning package.json: No license field
   Creating an optimized production build...
   Build completed
Done in 2.34s.
"#;

        let result = filter_yarn_output(output);

        assert!(!result.contains("yarn run v"));
        assert!(!result.contains("$ next build"));
        assert!(result.contains("warning package.json"));
        assert!(!result.contains("Done in"));
        assert!(result.contains("Build completed"));
    }

    #[test]
    fn test_filter_yarn_output_preserves_script_warnings_and_errors() {
        let output = r#"
yarn run v1.22.22
$ vite build
warning src/App.tsx: unused export 'debugValue'
ERROR in src/App.tsx: Cannot find name 'missingValue'
Done in 2.34s.
"#;

        let result = filter_yarn_output(output);

        assert!(result.contains("warning src/App.tsx"));
        assert!(result.contains("ERROR in src/App.tsx"));
    }

    #[test]
    fn test_filter_yarn_output_empty() {
        let output = "\nyarn run v1.22.22\n$ echo ok\nDone in 0.01s.\n";
        let result = filter_yarn_output(output);
        assert_eq!(result, "ok");
    }
}
