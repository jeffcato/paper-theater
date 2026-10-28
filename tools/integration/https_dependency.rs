//! The approved transport dependency must reject cleartext before dispatch.
//! Runtime owns its real transport, reservations, cancellation and response caps.
use std::time::Duration;

#[tokio::test]
async fn https_client_rejects_cleartext_without_network_or_credentials() {
    let client = reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(1))
        .read_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let error = client
        .post("http://127.0.0.1:1/")
        .body(b"diagnostic dependency check".to_vec())
        .send()
        .await
        .unwrap_err();
    assert!(
        error.is_builder(),
        "cleartext must fail before connecting: {error}"
    );
}
