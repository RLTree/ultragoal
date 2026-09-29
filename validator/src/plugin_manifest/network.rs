use super::validation::bounded_text;
use std::net::IpAddr;

const RESERVED_AUTHORITIES: [&str; 14] = [
    "example.com",
    "example.net",
    "example.org",
    "home.arpa",
    "local",
    "localhost",
    "internal",
    "test",
    "invalid",
    "example",
    "onion",
    "home",
    "lan",
    "corp",
];

pub(crate) fn email(value: &str) -> bool {
    let mut parts = value.split('@');
    parts.next().is_some_and(|part| {
        bounded_text(part)
            && part == part.trim()
            && !part.starts_with('.')
            && !part.ends_with('.')
            && !part.contains("..")
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_{|}~-".contains(&byte))
    }) && parts.next().is_some_and(public_dns_host)
        && parts.next().is_none()
        && value.len() <= 320
}

pub(crate) fn https(value: &str) -> bool {
    if !bounded_text(value)
        || value.len() > 2048
        || value.contains(char::is_whitespace)
        || value.contains('\\')
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let end = rest
        .find(|character| ['/', '?', '#'].contains(&character))
        .unwrap_or(rest.len());
    let authority = &rest[..end];
    if authority.is_empty() || authority.contains('@') || authority.starts_with('[') {
        return false;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => (host, Some(port)),
        Some(_) => return false,
        None => (authority, None),
    };
    public_dns_host(host)
        && port.is_none_or(|value| {
            !value.is_empty()
                && value.bytes().all(|byte| byte.is_ascii_digit())
                && value.parse::<u16>().is_ok_and(|value| value != 0)
        })
}

fn numeric_component(label: &str) -> bool {
    label
        .strip_prefix("0x")
        .or_else(|| label.strip_prefix("0X"))
        .map_or_else(
            || !label.is_empty() && label.bytes().all(|byte| byte.is_ascii_digit()),
            |digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
        )
}

fn ambiguous_numeric_host(host: &str, labels: &[&str]) -> bool {
    host.parse::<IpAddr>().is_ok()
        || (labels.len() <= 4 && labels.iter().all(|label| numeric_component(label)))
}

fn at_or_below(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

fn public_dns_host(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    let labels = host.split('.').collect::<Vec<_>>();
    !host.is_empty()
        && host.len() <= 253
        && host.is_ascii()
        && labels.len() >= 2
        && !ambiguous_numeric_host(host, &labels)
        && labels
            .last()
            .is_some_and(|label| label.bytes().any(|byte| byte.is_ascii_alphabetic()))
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
        && !RESERVED_AUTHORITIES
            .iter()
            .any(|domain| at_or_below(&lower, domain))
}

#[cfg(test)]
mod tests {
    use super::{email, https};

    #[test]
    fn supported_https_requires_public_dns_authority() {
        for valid in [
            "https://terrynoblin.dev",
            "https://dead.beef",
            "https://home.arpa.example.dev",
            "https://github.com/terrynoblin/harness-ultragoal",
            "https://mcp.terrynoblin.dev:443/v1?mode=safe#anchor",
        ] {
            assert!(https(valid), "expected supported URL: {valid}");
        }
        for invalid in [
            "http://mcp.terrynoblin.dev",
            "https://",
            "https://localhost",
            "https://localhost.local",
            "https://mcp.internal",
            "https://mcp.example.com",
            "https://example.com",
            "https://example.net",
            "https://example.org",
            "https://sub.example.net",
            "https://sub.example.org",
            "https://home.arpa",
            "https://sub.home.arpa",
            "https://HOME.ARPA:443",
            "https://127.0.0.1",
            "https://127.1",
            "https://0177.1",
            "https://0x7f.1",
            "https://0X7F.0x0.0x0.0x1",
            "https://2130706433",
            "https://10.0.0.1:443",
            "https://user@mcp.terrynoblin.dev",
            "https://[::1]",
            "https://[::ffff:127.0.0.1]",
            "https://mcp.terrynoblin.dev:0",
            "https://mcp.terrynoblin.dev:bogus",
            "https://mcp.terrynoblin.dev\\redirect",
            "https://mcp.terrynoblin.dev bad",
        ] {
            assert!(!https(invalid), "expected rejected URL: {invalid}");
        }
    }

    #[test]
    fn author_email_requires_one_bounded_public_dns_address() {
        for valid in [
            "tree@terrynoblin.dev",
            "proof+plugin@sub.terrynoblin.dev",
            "tree@home.arpa.example.dev",
        ] {
            assert!(email(valid), "expected supported email: {valid}");
        }
        for invalid in [
            "tree",
            "tree@@terrynoblin.dev",
            ".tree@terrynoblin.dev",
            "tree..proof@terrynoblin.dev",
            "tree@example.com",
            "tree@example.net",
            "tree@example.org",
            "tree@sub.example.net",
            "tree@sub.example.org",
            "tree@home.arpa",
            "tree@sub.home.arpa",
            "tree@HOME.ARPA",
            "tree@127.0.0.1",
            "tree@127.1",
            "tree@0177.1",
            "tree@0x7f.1",
            "tree@mcp.internal",
        ] {
            assert!(!email(invalid), "expected rejected email: {invalid}");
        }
    }
}
