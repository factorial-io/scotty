use std::sync::Once;

static INIT: Once = Once::new();

/// Install a process-wide rustls [`CryptoProvider`] if none is set yet.
///
/// reqwest, tokio-tungstenite and bollard all share a single rustls instance.
/// Depending on how Cargo resolves features, rustls can end up compiled with
/// both the `ring` and `aws-lc-rs` providers (or, in some builds, with neither
/// being auto-selectable). In that case rustls refuses to guess and panics on
/// the first TLS handshake with:
///
/// > Could not automatically determine the process-level CryptoProvider from
/// > Rustls crate features.
///
/// Installing an explicit default at startup makes the choice deterministic and
/// independent of dependency-resolution drift. We pin `ring` to match the
/// `rustls-tls-native-roots` stack used by the websocket client.
///
/// Safe to call multiple times and from any entry point; only the first call
/// has any effect.
pub fn ensure_crypto_provider() {
    INIT.call_once(|| {
        // Ignore the result: an `Err` only means a provider was already
        // installed (e.g. by another entry point), which is exactly what we want.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
