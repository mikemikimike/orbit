//! HTTP client helpers that honour custom CA certificates.

use std::time::Duration;

/// Build a `reqwest::Client` that trusts the system root store **and** any CA
/// bundle pointed to by `SSL_CERT_FILE` / `REQUESTS_CA_BUNDLE` / `NODE_EXTRA_CA_CERTS`.
/// This lets customers with private CAs (corporate proxies, internal OIDC providers)
/// configure trust via the Helm chart's `caCertificates` section.
pub fn client_with_custom_ca(timeout: Duration) -> reqwest::Client {
    let mut builder = reqwest::Client::builder().timeout(timeout);

    for var in ["SSL_CERT_FILE", "REQUESTS_CA_BUNDLE", "NODE_EXTRA_CA_CERTS"] {
        if let Ok(path) = std::env::var(var) {
            if !path.is_empty() {
                if let Ok(pem) = std::fs::read(&path) {
                    // reqwest::Certificate::from_pem accepts a PEM bundle.
                    if let Ok(cert) = reqwest::Certificate::from_pem(&pem) {
                        builder = builder.add_root_certificate(cert);
                    }
                }
            }
        }
    }

    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}
