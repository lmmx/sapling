#![allow(missing_docs)]
#[cfg(feature = "cli")]
mod cli_integration {
    use assert_cmd::cargo_bin_cmd;
    use predicates::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn cli_applies_literal_target_patch() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("hello.txt");
        fs::write(&source_file, "Hello Louis!").unwrap();

        let patch_file = temp.path().join("patches.json");
        let patch_json = format!(
            r#"[{{
                "file": "{}",
                "snippet": {{
                    "At": {{
                        "target": {{"Literal": "Louis"}},
                        "mode": "Include"
                    }}
                }},
                "replacement": "World"
            }}]"#,
            source_file.display()
        );
        fs::write(&patch_file, patch_json).unwrap();

        cargo_bin_cmd!("sapling")
            .arg(patch_file.to_str().unwrap())
            .assert()
            .success()
            .stderr(predicate::str::contains("Patched:"));

        let result = fs::read_to_string(&source_file).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn cli_applies_line_range_patch() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("lines.txt");
        fs::write(&source_file, "line1\nline2\nline3\nline4\n").unwrap();

        let patch_json = format!(
            r#"[{{
                "file": "{}",
                "snippet": {{
                    "Between": {{
                        "start": {{
                            "target": {{"Line": 1}},
                            "mode": "Include"
                        }},
                        "end": {{
                            "target": {{"Line": 3}},
                            "mode": "Exclude"
                        }}
                    }}
                }},
                "replacement": "replaced\n"
            }}]"#,
            source_file.display()
        );

        cargo_bin_cmd!("sapling")
            .write_stdin(patch_json)
            .assert()
            .success();

        let result = fs::read_to_string(&source_file).unwrap();
        assert_eq!(result, "line1\nreplaced\nline4\n");
    }

    #[test]
    fn cli_applies_between_markers_patch() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("markers.html");
        fs::write(&source_file, "<!-- start -->old content<!-- end -->").unwrap();

        let patch_json = format!(
            r#"[{{
                "file": "{}",
                "snippet": {{
                    "Between": {{
                        "start": {{
                            "target": {{"Literal": "<!-- start -->"}},
                            "mode": "Exclude"
                        }},
                        "end": {{
                            "target": {{"Literal": "<!-- end -->"}},
                            "mode": "Exclude"
                        }}
                    }}
                }},
                "replacement": "new content"
            }}]"#,
            source_file.display()
        );

        cargo_bin_cmd!("sapling")
            .write_stdin(patch_json)
            .assert()
            .success();

        let result = fs::read_to_string(&source_file).unwrap();
        assert_eq!(result, "<!-- start -->new content<!-- end -->");
    }

    #[test]
    fn cli_applies_pattern_patch() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("version.txt");
        fs::write(&source_file, "version=1.2.3").unwrap();

        let patch_json = format!(
            r#"[{{
                "file": "{}",
                "snippet": {{
                    "At": {{
                        "target": {{"Pattern": "\\d+\\.\\d+\\.\\d+"}},
                        "mode": "Include"
                    }}
                }},
                "replacement": "2.0.0"
            }}]"#,
            source_file.display()
        );

        cargo_bin_cmd!("sapling")
            .write_stdin(patch_json)
            .assert()
            .success();

        let result = fs::read_to_string(&source_file).unwrap();
        assert_eq!(result, "version=2.0.0");
    }

    #[test]
    fn cli_applies_deletion_patch() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("delete.txt");
        fs::write(&source_file, "keep this\ndelete this\nkeep this").unwrap();

        let patch_json = format!(
            r#"[{{
                "file": "{}",
                "snippet": {{
                    "At": {{
                        "target": {{"Literal": "delete this\n"}},
                        "mode": "Include"
                    }}
                }},
                "replacement": ""
            }}]"#,
            source_file.display()
        );

        cargo_bin_cmd!("sapling")
            .write_stdin(patch_json)
            .assert()
            .success();

        let result = fs::read_to_string(&source_file).unwrap();
        assert_eq!(result, "keep this\nkeep this");
    }

    #[test]
    fn cli_dry_run_does_not_modify_file() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("original.txt");
        let original_content = "Don't change me!";
        fs::write(&source_file, original_content).unwrap();

        let patch_json = format!(
            r#"[{{
                "file": "{}",
                "snippet": {{
                    "At": {{
                        "target": {{"Literal": "Don't"}},
                        "mode": "Include"
                    }}
                }},
                "replacement": "Please"
            }}]"#,
            source_file.display()
        );

        cargo_bin_cmd!("sapling")
            .arg("--dry-run")
            .write_stdin(patch_json)
            .assert()
            .success()
            .stderr(predicate::str::contains("Would patch:"));

        let result = fs::read_to_string(&source_file).unwrap();
        assert_eq!(result, original_content);
    }

    #[test]
    fn cli_multiple_patches_same_file() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("multi.txt");
        fs::write(&source_file, "foo bar baz").unwrap();

        let patch_json = format!(
            r#"[
                {{
                    "file": "{}",
                    "snippet": {{
                        "At": {{
                            "target": {{"Literal": "foo"}},
                            "mode": "Include"
                        }}
                    }},
                    "replacement": "FOO"
                }},
                {{
                    "file": "{}",
                    "snippet": {{
                        "At": {{
                            "target": {{"Literal": "baz"}},
                            "mode": "Include"
                        }}
                    }},
                    "replacement": "BAZ"
                }}
            ]"#,
            source_file.display(),
            source_file.display()
        );

        cargo_bin_cmd!("sapling")
            .write_stdin(patch_json)
            .assert()
            .success();

        let result = fs::read_to_string(&source_file).unwrap();
        assert_eq!(result, "FOO bar BAZ");
    }

    #[test]
    fn cli_rejects_overlapping_patches() {
        let temp = TempDir::new().unwrap();

        let source_file = temp.path().join("overlap.txt");
        fs::write(&source_file, "abcdef").unwrap();

        let patch_json = format!(
            r#"[
                {{
                    "file": "{}",
                    "snippet": {{
                        "At": {{
                            "target": {{"Literal": "bcd"}},
                            "mode": "Include"
                        }}
                    }},
                    "replacement": "XXX"
                }},
                {{
                    "file": "{}",
                    "snippet": {{
                        "At": {{
                            "target": {{"Literal": "def"}},
                            "mode": "Include"
                        }}
                    }},
                    "replacement": "YYY"
                }}
            ]"#,
            source_file.display(),
            source_file.display()
        );

        cargo_bin_cmd!("sapling")
            .write_stdin(patch_json)
            .assert()
            .failure()
            .stderr(predicate::str::contains("Overlapping"));
    }

    #[test]
    fn cli_fails_on_invalid_json() {
        cargo_bin_cmd!("sapling")
            .write_stdin("not valid json")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Error:"));
    }
}
