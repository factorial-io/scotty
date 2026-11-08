//! The egress policy for Client ID Metadata Documents (SSRF guard).
//!
//! grantry fetches the document an HTTPS `client_id` names. Anyone can make
//! Scotty fetch any URL that way, so [`EgressFetcher`] fetches only `https`
//! URLs whose host resolves to public addresses only, then connects to the
//! vetted address (pinned, so a second DNS answer cannot swap it), follows no
//! redirects, uses no proxy and stops at grantry's size limit.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use async_trait::async_trait;
use grantry::client::{FetchError, Fetched, MetadataFetcher};
use url::Url;

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, Default)]
pub struct EgressFetcher;

#[async_trait]
impl MetadataFetcher for EgressFetcher {
    async fn fetch(&self, url: &Url, limit: usize) -> Result<Fetched, FetchError> {
        let refused = |why: String| Err(FetchError(why));
        if url.scheme() != "https" {
            return refused("only https document URLs are fetched".into());
        }
        let Some(host) = url.host_str() else {
            return refused("no host".into());
        };
        let port = url.port_or_known_default().unwrap_or(443);
        let addr = vetted_address(host, port).await?;

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(TIMEOUT)
            .connect_timeout(TIMEOUT)
            // Connect to the vetted address only; SNI and the certificate
            // check still use the host name.
            .resolve(host, addr)
            .build()
            .map_err(|e| FetchError(format!("client: {e}")))?;
        let mut response = client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| FetchError(format!("fetch failed: {e}")))?;
        if !response.status().is_success() {
            return refused(format!("status {}", response.status()));
        }
        let cache_control = response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| FetchError(format!("read failed: {e}")))?
        {
            if body.len() + chunk.len() > limit {
                return refused("document too large".into());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Fetched {
            body: body.into(),
            cache_control,
        })
    }
}

/// Resolve `host` and return an address to connect to, refusing the host if
/// any of its addresses is not public (a mixed answer is not trusted).
async fn vetted_address(host: &str, port: u16) -> Result<SocketAddr, FetchError> {
    // `Url::host_str` keeps IPv6 literals in brackets.
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let addrs: Vec<SocketAddr> = match bare.parse::<IpAddr>() {
        Ok(ip) => vec![SocketAddr::new(ip, port)],
        Err(_) => tokio::net::lookup_host((bare, port))
            .await
            .map_err(|e| FetchError(format!("resolve failed: {e}")))?
            .collect(),
    };
    match addrs.first() {
        None => Err(FetchError("host has no address".into())),
        Some(_) if addrs.iter().any(|a| !is_public(a.ip())) => {
            Err(FetchError("host resolves to a non-public address".into()))
        }
        Some(first) => Ok(*first),
    }
}

/// Whether `ip` is a public unicast address Scotty may connect to.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    let refused = a == 0 // "this network", unspecified
        || a == 10 // RFC 1918
        || a == 127 // loopback
        || (a == 100 && (64..128).contains(&b)) // CGNAT 100.64/10
        || (a == 169 && b == 254) // link-local
        || (a == 172 && (16..32).contains(&b)) // RFC 1918
        || (a == 192 && b == 168) // RFC 1918
        || (a == 192 && b == 0 && c == 0) // IETF protocol assignments
        || (a == 192 && b == 0 && c == 2) // TEST-NET-1
        || (a == 198 && (b == 18 || b == 19)) // benchmarking
        || (a == 198 && b == 51 && c == 100) // TEST-NET-2
        || (a == 203 && b == 0 && c == 113) // TEST-NET-3
        || a >= 224; // multicast, reserved, broadcast
    !refused
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    let s = ip.segments();
    // Only global unicast (2000::/3), less the ranges inside it that are not
    // a plain public host: documentation (2001:db8::/32, 3fff::/20), Teredo
    // (2001::/32) and 6to4 (2002::/16), which tunnel to arbitrary IPv4.
    // ULA fc00::/7, link-local fe80::/10, multicast ff00::/8, loopback,
    // unspecified and NAT64 64:ff9b::/96 all fall outside 2000::/3.
    (s[0] & 0xe000) == 0x2000
        && !(s[0] == 0x2001 && (s[1] == 0x0db8 || s[1] == 0))
        && s[0] != 0x2002
        && (s[0] & 0xfff0) != 0x3ff0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public(ip: &str) -> bool {
        is_public(ip.parse().unwrap())
    }

    #[test]
    fn refuses_internal_and_special_addresses() {
        for ip in [
            "127.0.0.1",
            "127.8.9.10",
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "169.254.169.254", // cloud metadata
            "100.64.0.1",
            "100.127.255.255",
            "0.0.0.0",
            "224.0.0.1",
            "239.255.255.250",
            "255.255.255.255",
            "240.0.0.1",
            "192.0.2.1",
            "198.51.100.1",
            "203.0.113.1",
            "198.18.0.1",
            "192.0.0.8",
            "::1",
            "::",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "ff02::1",
            "2001:db8::1",
            "3fff::1",
            "2001::1",
            "2002:7f00:1::1",
            "64:ff9b::7f00:1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
        ] {
            assert!(!public(ip), "{ip} must be refused");
        }
    }

    #[test]
    fn allows_public_addresses() {
        for ip in [
            "1.1.1.1",
            "8.8.8.8",
            "100.63.255.255",
            "100.128.0.1",
            "172.15.0.1",
            "172.32.0.1",
            "160.79.104.10",
            "2606:4700:4700::1111",
            "2a00:1450:4001::1",
            "::ffff:8.8.8.8",
        ] {
            assert!(public(ip), "{ip} must be allowed");
        }
    }

    #[tokio::test]
    async fn refuses_non_https_and_internal_hosts() {
        let fetch = |u: &str| {
            let url = Url::parse(u).unwrap();
            async move { EgressFetcher.fetch(&url, 1024).await }
        };
        assert!(fetch("http://example.com/client.json").await.is_err());
        let err = fetch("https://127.0.0.1/client.json").await.unwrap_err();
        assert!(err.0.contains("non-public"), "{err}");
        let err = fetch("https://[::1]/client.json").await.unwrap_err();
        assert!(err.0.contains("non-public"), "{err}");
        let err = fetch("https://localhost/client.json").await.unwrap_err();
        assert!(err.0.contains("non-public"), "{err}");
    }
}
