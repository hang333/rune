//! On a disposable Android emulator, run the Android build of this test as
//! the shell user (not root). /data/local/tmp permits file creation and rename
//! but denies hard links under the stock emulator's SELinux policy:
//! `adb push <test-binary> /data/local/tmp/credential_restart`, then
//! `adb shell 'chmod 700 /data/local/tmp/credential_restart && TMPDIR=/data/local/tmp /data/local/tmp/credential_restart'`.

use std::{env, fs, path::PathBuf, process::Command};

use anyhow::{Result, ensure};
use database::credentials::{Credential, CredentialRef, CredentialStore, Secret};

#[tokio::test]
async fn first_credential_survives_process_restart() -> Result<()> {
    const WRITE_DIR: &str = "RUNE_TEST_FRESH_CREDENTIAL_DIR";
    const RECEIPT: &str = "saved-reference";
    if let Some(dir) = env::var_os(WRITE_DIR) {
        let dir = PathBuf::from(dir);
        let reference = CredentialStore::open(&dir)
            .put(Credential {
                username: "fixture".into(),
                secret: Secret::new(b"test password".to_vec()),
            })
            .await?;
        fs::write(dir.join(RECEIPT), serde_json::to_vec(&reference)?)?;
        return Ok(());
    }

    let dir = tempfile::tempdir()?;
    let status = Command::new(env::current_exe()?)
        .args(["--exact", "first_credential_survives_process_restart"])
        .env(WRITE_DIR, dir.path())
        .status()?;
    ensure!(status.success(), "first credential save failed: {status}");

    // Only the opaque reference crosses the process boundary, as it does
    // when a library record is reopened after app startup.
    let reference: CredentialRef = serde_json::from_slice(&fs::read(dir.path().join(RECEIPT))?)?;
    let saved = CredentialStore::open(dir.path()).get(reference).await?;
    assert_eq!(saved.username, "fixture");
    assert!(saved.secret.as_bytes() == b"test password");
    Ok(())
}
