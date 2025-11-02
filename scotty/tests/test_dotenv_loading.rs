use std::env;
use std::fs;
use tempfile::TempDir;

/// Test that .env files are loaded in the correct order with proper precedence
#[test]
fn test_dotenv_loading_order() {
    // Create a temporary directory for test files
    let temp_dir = TempDir::new().unwrap();

    // Clean up any existing env vars from previous tests
    env::remove_var("TEST_VAR_1");
    env::remove_var("TEST_VAR_2");
    env::remove_var("TEST_VAR_3");

    // Create .env file
    let env_path = temp_dir.path().join(".env");
    fs::write(
        &env_path,
        "TEST_VAR_1=from_env\nTEST_VAR_2=from_env\nTEST_VAR_3=from_env\n",
    )
    .unwrap();

    // Create .env.local file (should override .env)
    let env_local_path = temp_dir.path().join(".env.local");
    fs::write(&env_local_path, "TEST_VAR_2=from_env_local\n").unwrap();

    // Set an environment variable (should override both .env and .env.local)
    env::set_var("TEST_VAR_3", "from_environment");

    // Load .env files in the same order as main.rs using from_path
    // Load .env.local first, then .env (dotenvy doesn't override existing vars)
    dotenvy::from_path(&env_local_path).ok();
    dotenvy::from_path(&env_path).ok();

    // Verify the precedence:
    // TEST_VAR_1: only in .env
    assert_eq!(env::var("TEST_VAR_1").unwrap(), "from_env");

    // TEST_VAR_2: in .env and .env.local (local should win)
    assert_eq!(env::var("TEST_VAR_2").unwrap(), "from_env_local");

    // TEST_VAR_3: in .env, .env.local, and environment (environment should win)
    assert_eq!(env::var("TEST_VAR_3").unwrap(), "from_environment");

    // Clean up
    env::remove_var("TEST_VAR_1");
    env::remove_var("TEST_VAR_2");
    env::remove_var("TEST_VAR_3");
}

/// Test that missing .env files don't cause failures
#[test]
fn test_missing_dotenv_files() {
    let temp_dir = TempDir::new().unwrap();

    // Try to load non-existent files - should not panic
    dotenvy::from_path(temp_dir.path().join(".env")).ok();
    dotenvy::from_path(temp_dir.path().join(".env.local")).ok();

    // Test passes if we get here without panicking
}

/// Test that .env.local is optional
#[test]
fn test_only_dotenv_present() {
    let temp_dir = TempDir::new().unwrap();
    env::remove_var("TEST_VAR_ONLY_ENV");

    // Create only .env file
    let env_path = temp_dir.path().join(".env");
    fs::write(&env_path, "TEST_VAR_ONLY_ENV=from_env_only\n").unwrap();

    // Load .env files
    dotenvy::from_path(&env_path).ok();
    dotenvy::from_path(temp_dir.path().join(".env.local")).ok(); // Should not fail even though it doesn't exist

    // Verify the value from .env is loaded
    assert_eq!(env::var("TEST_VAR_ONLY_ENV").unwrap(), "from_env_only");

    // Clean up
    env::remove_var("TEST_VAR_ONLY_ENV");
}

/// Test that .env is optional
#[test]
fn test_only_dotenv_local_present() {
    let temp_dir = TempDir::new().unwrap();
    env::remove_var("TEST_VAR_ONLY_LOCAL");

    // Create only .env.local file
    let env_local_path = temp_dir.path().join(".env.local");
    fs::write(&env_local_path, "TEST_VAR_ONLY_LOCAL=from_local_only\n").unwrap();

    // Load .env files
    dotenvy::from_path(temp_dir.path().join(".env")).ok(); // Should not fail even though it doesn't exist
    dotenvy::from_path(&env_local_path).ok();

    // Verify the value from .env.local is loaded
    assert_eq!(env::var("TEST_VAR_ONLY_LOCAL").unwrap(), "from_local_only");

    // Clean up
    env::remove_var("TEST_VAR_ONLY_LOCAL");
}

/// Test with SCOTTY-prefixed environment variables (the actual use case)
#[test]
fn test_scotty_env_vars() {
    let temp_dir = TempDir::new().unwrap();
    env::remove_var("SCOTTY__API__AUTH_MODE");
    env::remove_var("SCOTTY__DOCKER__REGISTRIES__TEST__PASSWORD");

    // Create .env file with SCOTTY vars
    let env_path = temp_dir.path().join(".env");
    fs::write(
        &env_path,
        "SCOTTY__API__AUTH_MODE=dev\nSCOTTY__DOCKER__REGISTRIES__TEST__PASSWORD=secret123\n",
    )
    .unwrap();

    // Create .env.local that overrides one var
    let env_local_path = temp_dir.path().join(".env.local");
    fs::write(&env_local_path, "SCOTTY__API__AUTH_MODE=oauth\n").unwrap();

    // Load .env files - .env.local first, then .env
    dotenvy::from_path(&env_local_path).ok();
    dotenvy::from_path(&env_path).ok();

    // Verify the precedence
    assert_eq!(env::var("SCOTTY__API__AUTH_MODE").unwrap(), "oauth"); // .env.local wins
    assert_eq!(
        env::var("SCOTTY__DOCKER__REGISTRIES__TEST__PASSWORD").unwrap(),
        "secret123"
    ); // only in .env

    // Clean up
    env::remove_var("SCOTTY__API__AUTH_MODE");
    env::remove_var("SCOTTY__DOCKER__REGISTRIES__TEST__PASSWORD");
}
