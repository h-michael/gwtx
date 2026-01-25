use crate::common::{
    BASIC_CONFIG, INVALID_CONFIG_ABSOLUTE_PATH, INVALID_CONFIG_DUPLICATE_TARGETS,
    INVALID_CONFIG_PATH_TRAVERSAL, MINIMAL_CONFIG, TestRepo,
};
use predicates::prelude::*;

#[test]
fn test_config_validate_success() {
    let repo = TestRepo::with_config(BASIC_CONFIG);

    // Create source files to pass validation
    repo.create_file_and_commit("local.env", "export FOO=bar\n", "Add local.env");
    repo.create_file_and_commit("config.template", "# Config\n", "Add config");

    repo.gwtx()
        .args(["config", "validate"])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid").or(predicate::str::contains("OK")));
}

#[test]
fn test_config_validate_minimal() {
    let repo = TestRepo::with_config(MINIMAL_CONFIG);

    repo.gwtx().args(["config", "validate"]).assert().success();
}

#[test]
fn test_config_validate_invalid_absolute_path() {
    let repo = TestRepo::with_config(INVALID_CONFIG_ABSOLUTE_PATH);

    repo.gwtx()
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("absolute"));
}

#[test]
fn test_config_validate_invalid_path_traversal() {
    let repo = TestRepo::with_config(INVALID_CONFIG_PATH_TRAVERSAL);

    repo.gwtx()
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("traversal").or(predicate::str::contains("..")));
}

#[test]
fn test_config_validate_duplicate_targets() {
    let repo = TestRepo::with_config(INVALID_CONFIG_DUPLICATE_TARGETS);

    // Create source files
    repo.create_file_and_commit("file1.txt", "content1\n", "Add file1");
    repo.create_file_and_commit("file2.txt", "content2\n", "Add file2");

    repo.gwtx()
        .args(["config", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate"));
}

#[test]
fn test_config_validate_without_source_files() {
    let repo = TestRepo::with_config(BASIC_CONFIG);
    // Source files are NOT created
    // config validate only validates YAML syntax and schema, not file existence

    repo.gwtx().args(["config", "validate"]).assert().success();
}

#[test]
fn test_config_no_subcommand_shows_help() {
    let repo = TestRepo::with_config(MINIMAL_CONFIG);

    // Running `gwtx config` without subcommand should show help or available commands
    repo.gwtx().args(["config"]).assert().success().stdout(
        predicate::str::contains("validate")
            .or(predicate::str::contains("schema"))
            .or(predicate::str::contains("help")),
    );
}

#[test]
fn test_config_schema() {
    let repo = TestRepo::with_config(MINIMAL_CONFIG);

    // Generate JSON schema
    repo.gwtx()
        .args(["config", "schema"])
        .assert()
        .success()
        .stdout(predicate::str::contains("$schema"))
        .stdout(predicate::str::contains("properties"));
}

#[test]
fn test_config_validate_no_config_file() {
    let repo = TestRepo::new();

    // No config file is treated as valid (empty default config)
    repo.gwtx().args(["config", "validate"]).assert().success();
}
