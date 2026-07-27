use std::cmp::Ordering;

const VERSION_LIMIT: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Version {
    core: [Numeric; 3],
    prerelease: Option<Vec<Identifier>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Numeric(String);

#[derive(Clone, Debug, Eq, PartialEq)]
enum Identifier {
    Numeric(Numeric),
    Text(String),
}

impl Version {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        if value.is_empty()
            || value.len() > VERSION_LIMIT
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        {
            return None;
        }
        let mut plus = value.split('+');
        let core_prerelease = plus.next()?;
        if let Some(build) = plus.next() {
            parse_identifiers(build, false)?;
        }
        if plus.next().is_some() {
            return None;
        }
        let (core, prerelease) = core_prerelease
            .split_once('-')
            .map_or((core_prerelease, None), |(core, value)| (core, Some(value)));
        let mut core_parts = core.split('.');
        let core = [
            Numeric::parse(core_parts.next()?)?,
            Numeric::parse(core_parts.next()?)?,
            Numeric::parse(core_parts.next()?)?,
        ];
        if core_parts.next().is_some() {
            return None;
        }
        let prerelease = match prerelease {
            Some(value) => Some(parse_identifiers(value, true)?),
            None => None,
        };
        Some(Self { core, prerelease })
    }

    pub(crate) fn precedence_cmp(&self, other: &Self) -> Ordering {
        for (left, right) in self.core.iter().zip(&other.core) {
            let order = left.cmp_value(right);
            if order != Ordering::Equal {
                return order;
            }
        }
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(left), Some(right)) => compare_prerelease(left, right),
        }
    }
}

impl Numeric {
    fn parse(value: &str) -> Option<Self> {
        (!value.is_empty()
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && (value == "0" || !value.starts_with('0')))
        .then(|| Self(value.to_owned()))
    }

    fn cmp_value(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.cmp(&other.0))
    }
}

fn parse_identifiers(value: &str, strict_numeric: bool) -> Option<Vec<Identifier>> {
    value
        .split('.')
        .map(|part| {
            if part.is_empty()
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            {
                return None;
            }
            if strict_numeric && part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Numeric::parse(part).map(Identifier::Numeric);
            }
            Some(Identifier::Text(part.to_owned()))
        })
        .collect()
}

fn compare_prerelease(left: &[Identifier], right: &[Identifier]) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        let order = match (left, right) {
            (Identifier::Numeric(left), Identifier::Numeric(right)) => left.cmp_value(right),
            (Identifier::Numeric(_), Identifier::Text(_)) => Ordering::Less,
            (Identifier::Text(_), Identifier::Numeric(_)) => Ordering::Greater,
            (Identifier::Text(left), Identifier::Text(right)) => left.cmp(right),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

#[cfg(test)]
mod tests {
    use super::Version;
    use std::cmp::Ordering;

    #[test]
    fn strict_semver_grammar_rejects_ambiguous_forms() {
        for valid in ["0.0.0", "1.2.3-alpha.1", "1.2.3-beta.2+codex.local-01"] {
            assert!(Version::parse(valid).is_some(), "{valid}");
        }
        for invalid in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.2.3-01",
            "1.2.3-",
            "1.2.3+",
            "1.2.3+a..b",
            "1.2.3+a+b",
            "1.2.3-α",
            "1.2.3\n",
        ] {
            assert!(Version::parse(invalid).is_none(), "{invalid:?}");
        }
    }

    #[test]
    fn precedence_is_semver_exact_without_integer_parsing() {
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in ordered.windows(2) {
            assert_eq!(
                Version::parse(pair[0])
                    .unwrap()
                    .precedence_cmp(&Version::parse(pair[1]).unwrap()),
                Ordering::Less
            );
        }
        let huge = format!("{}.0.0", "9".repeat(100));
        assert_eq!(
            Version::parse(&huge)
                .unwrap()
                .precedence_cmp(&Version::parse("999.0.0").unwrap()),
            Ordering::Greater
        );
        assert_eq!(
            Version::parse("1.2.3+one")
                .unwrap()
                .precedence_cmp(&Version::parse("1.2.3+two").unwrap()),
            Ordering::Equal
        );
    }
}
