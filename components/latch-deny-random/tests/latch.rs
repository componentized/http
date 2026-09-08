use test_harness::{Harness, HttpErrorCode, Level, LogEntry, TestSubject};

const LATCH: &str = "latch-deny-random";

/// A gate with the latch, configured with the entries.
async fn gate(config: &[(&str, &str)]) -> wasmtime::Result<TestSubject> {
    let mut harness = Harness::new("gate-client").latch(LATCH);
    for (key, value) in config {
        harness = harness.config(key, value);
    }
    harness.build().await
}

/// Send requests, whether each was denied.
async fn sends(gate: &mut TestSubject, count: usize) -> wasmtime::Result<Vec<bool>> {
    let mut denied = vec![];
    for _ in 0..count {
        let result = gate.send(http::Method::GET, "https://example.com/").await?;
        denied.push(result.is_err());
    }
    Ok(denied)
}

/// The messages the latch logged.
fn latch_logs(gate: &TestSubject) -> Vec<LogEntry> {
    gate.recorder()
        .logs()
        .into_iter()
        .filter(|entry| entry.context == "componentized-latch")
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn always_denies() -> wasmtime::Result<()> {
    let mut gate = gate(&[("probability", "1"), ("seed", "7")]).await?;
    assert_eq!(sends(&mut gate, 3).await?, vec![true, true, true]);
    assert_eq!(gate.recorder().requests(), vec![]);
    assert_eq!(
        latch_logs(&gate),
        vec![LogEntry::warn(
            "componentized-latch",
            "Randomly denying wasi:http requests PROBABILITY=1 SEED=7"
        )]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn denies_with_the_configured_reason() -> wasmtime::Result<()> {
    let mut gate = gate(&[("probability", "1"), ("reason", "connection-refused")]).await?;
    let result = gate.send(http::Method::GET, "https://example.com/").await?;
    assert!(
        matches!(result, Err(HttpErrorCode::ConnectionRefused)),
        "{result:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn never_denies() -> wasmtime::Result<()> {
    let mut gate = gate(&[("probability", "0")]).await?;
    assert_eq!(sends(&mut gate, 3).await?, vec![false, false, false]);
    assert_eq!(gate.recorder().requests().len(), 3);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn same_seed_denies_the_same_requests() -> wasmtime::Result<()> {
    let config = [("probability", "0.5"), ("seed", "1234")];
    let first = sends(&mut gate(&config).await?, 32).await?;
    let again = sends(&mut gate(&config).await?, 32).await?;
    assert_eq!(first, again);
    // some of each, a coin flip for 32 requests
    assert!(first.contains(&true) && first.contains(&false), "{first:?}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn random_seed_is_logged() -> wasmtime::Result<()> {
    let mut logged = vec![];
    for _ in 0..2 {
        let mut gate = gate(&[]).await?;
        sends(&mut gate, 1).await?;
        let logs = latch_logs(&gate);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, Level::Warn);
        let seed = logs[0]
            .message
            .strip_prefix("Randomly denying wasi:http requests PROBABILITY=0.1 SEED=")
            .expect("seed is logged")
            .parse::<u64>()?;
        logged.push(seed);
    }
    // each instance draws its own seed
    assert_ne!(logged[0], logged[1]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_config_fails_requests() -> wasmtime::Result<()> {
    let mut gate = gate(&[("probability", "often")]).await?;
    let result = gate.send(http::Method::GET, "https://example.com/").await?;
    assert!(
        matches!(
            &result,
            Err(HttpErrorCode::InternalError(Some(message)))
                if message == "latch-error: invalid-config<latch-deny-random>"
        ),
        "{result:?}"
    );
    assert_eq!(
        latch_logs(&gate),
        vec![LogEntry::critical(
            "componentized-latch",
            "Invalid config LATCH=latch-deny-random KEY=probability VALUE=often ERROR=expected a number from 0 to 1"
        )]
    );
    Ok(())
}
