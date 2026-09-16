use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], input: &str, no_color: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rustdown"));
    command
        .args(args)
        .env("TERM", "xterm-256color")
        .env_remove("RUSTDOWN_PRESERVE_FENCES")
        .env_remove("NO_COLOR");
    if no_color {
        command.env("NO_COLOR", "1");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn redirected_output_is_plain_unless_color_is_explicitly_requested() {
    for (args, no_color, colored) in [
        (vec![], false, false),
        (vec!["--color", "always"], false, true),
        (vec!["--color", "never"], false, false),
        (vec!["--color", "always"], true, true),
        (vec!["--color", "auto"], true, false),
    ] {
        let output = run(&args, "# Heading\n", no_color);
        assert!(output.status.success());
        assert_eq!(output.stdout.contains(&0x1b), colored);
    }
}

#[test]
fn width_and_ascii_options_work_through_stdin() {
    let output = run(
        &["--width", "20", "--ascii"],
        "- A long sentence that wraps onto several lines.\n\n---\n",
        false,
    );
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.starts_with("- A long sentence\n  that wraps onto\n"),
        "{text}"
    );
    assert!(text.lines().all(|line| line.len() <= 20));
    assert!(text.ends_with("--------------------\n"));
}

#[test]
fn options_work_before_and_after_html_subcommand() {
    for args in [
        vec!["--width", "30", "--color", "never", "html"],
        vec!["html", "--width", "30", "--color", "never"],
    ] {
        let output = run(&args, "<h1>Heading</h1><p>Hello <b>world</b></p>", false);
        assert!(output.status.success(), "{:?}", output.stderr);
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "Heading\n=======\n\nHello world\n"
        );
    }
}

#[test]
fn starting_html_does_not_reinterpret_later_markdown() {
    let output = run(&[], "<div>Intro</div>\n\n# Heading\n\n**bold**\n", false);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Intro\n\nHeading\n=======\n\nbold\n"
    );
}

#[test]
fn invalid_width_has_a_helpful_cli_error() {
    for width in ["0", "10001", "oops"] {
        let output = run(&["--width", width], "", false);
        assert!(!output.status.success());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("--width")
        );
    }
}

#[test]
fn inline_input_and_fence_options_remain_available() {
    let output = run(
        &["--preserve-fences", "~~~~rust title=test\nlet x = 1;\n~~~~"],
        "",
        false,
    );
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "~~~~rust title=test\nlet x = 1;\n~~~~\n"
    );
}

#[test]
fn wrapping_and_link_modes_are_available_from_cli() {
    let output = run(
        &["--wrap-code", "--width", "10"],
        "```\n0123456789abcdef\n```\n",
        false,
    );
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .all(|s| s.chars().count() <= 10)
    );
    let output = run(
        &["--links", "reference"],
        "[docs](https://example.com)\n",
        false,
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("docs[1]\n\nLinks\n\n[1]: https://example.com")
    );
    let output = run(
        &["--links", "hyperlink"],
        "[docs](https://example.com)\n",
        false,
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "docs (https://example.com)\n"
    );
    let output = run(
        &["--links", "hyperlink", "--color", "always"],
        "[docs](https://example.com)\n",
        false,
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("\x1b]8;;https://example.com\x1b\\docs")
    );
}
