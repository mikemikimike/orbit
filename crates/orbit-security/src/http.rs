//! HTTP client helpers that honour custom CA certificates.

use std::collections::BTreeSet;
use std::time::Duration;

/// Build a `reqwest::Client` that trusts the built-in roots **and** any CA bundle
/// pointed to by `SSL_CERT_FILE` / `REQUESTS_CA_BUNDLE` / `NODE_EXTRA_CA_CERTS`.
///
/// `reqwest` with `rustls-tls` uses webpki roots and does not read the system trust
/// store, so a private CA mounted by the Helm chart would otherwise be ignored. Bundles
/// may contain multiple certificates (root + intermediates); every certificate in the
/// file is added, not just the first one.
pub fn client_with_custom_ca(timeout: Duration) -> reqwest::Client {
    let mut builder = reqwest::Client::builder().timeout(timeout);
    let mut seen = BTreeSet::new();
    let mut added = 0usize;

    for var in ["SSL_CERT_FILE", "REQUESTS_CA_BUNDLE", "NODE_EXTRA_CA_CERTS"] {
        let Ok(path) = std::env::var(var) else {
            continue;
        };
        let path = path.trim();
        if path.is_empty() || !seen.insert(path.to_string()) {
            continue;
        }
        let pem = match std::fs::read(path) {
            Ok(pem) => pem,
            Err(error) => {
                tracing::warn!(%error, path, variable = var, "failed to read CA bundle");
                continue;
            }
        };

        match reqwest::Certificate::from_pem_bundle(&pem) {
            Ok(certificates) => {
                for certificate in certificates {
                    builder = builder.add_root_certificate(certificate);
                    added += 1;
                }
            }
            Err(bundle_error) => {
                // Some bundles mix certificate blocks with comments or other PEM
                // material. Fall back to adding each certificate block individually so
                // one bad block does not discard the whole bundle.
                let mut any = false;
                for block in certificate_blocks(&pem) {
                    if let Ok(certificate) = reqwest::Certificate::from_pem(&block) {
                        builder = builder.add_root_certificate(certificate);
                        added += 1;
                        any = true;
                    }
                }
                if !any {
                    tracing::warn!(
                        error = %bundle_error,
                        path,
                        variable = var,
                        "CA bundle could not be parsed; no certificates added"
                    );
                }
            }
        }
    }

    if added > 0 {
        tracing::debug!(certificates = added, "loaded custom CA certificates");
    }

    builder.build().unwrap_or_else(|error| {
        tracing::error!(%error, "failed to build HTTP client with custom CAs; using defaults");
        reqwest::Client::new()
    })
}

/// Extract every `-----BEGIN CERTIFICATE-----` … `-----END CERTIFICATE-----` block.
fn certificate_blocks(pem: &[u8]) -> Vec<Vec<u8>> {
    const BEGIN: &[u8] = b"-----BEGIN CERTIFICATE-----";
    const END: &[u8] = b"-----END CERTIFICATE-----";
    let mut blocks = Vec::new();
    let mut cursor = 0usize;
    while let Some(start) = find(&pem[cursor..], BEGIN) {
        let start = cursor + start;
        let Some(end) = find(&pem[start..], END) else {
            break;
        };
        let end = start + end + END.len();
        blocks.push(pem[start..end].to_vec());
        cursor = end;
    }
    blocks
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_multiple_certificate_blocks() {
        let pem =
            b"subject=/CN=one\n-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n\
-----BEGIN CERTIFICATE-----\nBBBB\n-----END CERTIFICATE-----\n";
        let blocks = certificate_blocks(pem);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].ends_with(b"-----END CERTIFICATE-----"));
        assert!(blocks[1].starts_with(b"-----BEGIN CERTIFICATE-----"));
    }

    #[test]
    fn ignores_unterminated_blocks() {
        let pem = b"-----BEGIN CERTIFICATE-----\nAAAA\n";
        assert!(certificate_blocks(pem).is_empty());
    }

    #[test]
    fn builds_a_client_without_a_configured_bundle() {
        // No env overrides in tests: the client must still build and be usable.
        let client = client_with_custom_ca(Duration::from_secs(1));
        assert!(client.get("http://127.0.0.1:1/").build().is_ok());
    }
}
