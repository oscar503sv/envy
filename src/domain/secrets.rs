//! Name-based secret heuristic (PRD §8.3).
//!
//! This is a heuristic, not a guarantee: it only looks at the key name, never
//! at the value, and it can miss secrets (`DATABASE_URL` with embedded
//! credentials) or flag harmless names (`PUBLIC_KEY`). It only decides the
//! default masking of a row; the user can always override it per row.
//!
//! The key is split into tokens on any non-alphanumeric character (`_`, `-`,
//! `.`), on lower-to-upper camelCase boundaries (`apiKey`), before the last
//! capital of an acronym (`APIKey`) and between letters and digits (`KEY2`).
//! Tokens are compared case-insensitively against [`SECRET_TOKENS`] as whole
//! tokens, never as substrings.
//!
//! No exclusion list is applied: PRD §8.3 defines no public-name exceptions,
//! and a false positive only hides a value, whereas a false negative leaks it.
//!
//! | Key | Secret | Why |
//! |---|---|---|
//! | `API_KEY`, `SECRET_KEY_BASE`, `AWS_KEYID` | yes | `KEY` / `KEYID` / `SECRET` token |
//! | `DB_PASSWORD`, `ADMIN_PASSWD`, `DB_PWD`, `DB_PW`, `ROOT_PASS` | yes | password tokens |
//! | `GITHUB_TOKEN`, `AUTH_TOKEN`, `AUTH` | yes | `TOKEN` / `AUTH` token |
//! | `APIKEY`, `apiKey`, `stripeApiKey`, `APIKey` | yes | token split |
//! | `api-key`, `aws.secret`, `KEY2`, `S3_KEY` | yes | other separators and digits |
//! | `PRIVATE_KEY`, `PUBLIC_KEY`, `PUBLISHABLE_KEY` | yes | `KEY`; no exclusions |
//! | `CREDENTIALS`, `SENTRY_DSN`, `PASSWORD_SALT`, `WEBHOOK_SIGNATURE`, `TLS_CERT` | yes | listed tokens |
//! | `KEYBOARD_LAYOUT` | no | `KEY` is only a substring |
//! | `PASSENGER_COUNT`, `COMPASS_DIR` | no | `PASS` is only a substring |
//! | `MONKEY`, `AUTHOR`, `AUTHORITY`, `SECRETARY` | no | substring only |
//! | `PORT`, `DATABASE_URL`, `NODE_ENV`, `` (empty) | no | no listed token |

/// Tokens (upper case) that mark a key as secret.
const SECRET_TOKENS: &[&str] = &[
    "SECRET",
    "TOKEN",
    "PASSWORD",
    "PASSWD",
    "PWD",
    "PW",
    "PASS",
    "PASSPHRASE",
    "KEY",
    "KEYID",
    "APIKEY",
    "PRIVATE",
    "CREDENTIAL",
    "CREDENTIALS",
    "AUTH",
    "SALT",
    "DSN",
    "SIGNATURE",
    "CERT",
];

/// Returns whether the key name suggests its value is a secret.
///
/// Takes only the key on purpose: values must never reach this classifier.
pub fn is_secret_key(key: &str) -> bool {
    tokenize(key).iter().any(|token| {
        SECRET_TOKENS
            .iter()
            .any(|secret| token.eq_ignore_ascii_case(secret))
    })
}

fn tokenize(key: &str) -> Vec<String> {
    let chars: Vec<char> = key.chars().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut previous: Option<char> = None;

    for (index, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            flush(&mut tokens, &mut current);
            previous = None;
            continue;
        }
        let next = chars.get(index + 1).copied();
        let is_boundary = previous.is_some_and(|prev| {
            (prev.is_lowercase() && c.is_uppercase())
                || (prev.is_ascii_digit() != c.is_ascii_digit())
                || (prev.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase))
        });
        if is_boundary {
            flush(&mut tokens, &mut current);
        }
        current.push(c);
        previous = Some(c);
    }
    flush(&mut tokens, &mut current);
    tokens
}

fn flush(tokens: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSITIVE: &[&str] = &[
        "DB_PW",
        "AWS_KEYID",
        "API_KEY",
        "SECRET_KEY_BASE",
        "DB_PASSWORD",
        "ADMIN_PASSWD",
        "DB_PWD",
        "ROOT_PASS",
        "GITHUB_TOKEN",
        "AUTH_TOKEN",
        "AUTH",
        "APIKEY",
        "apiKey",
        "stripeApiKey",
        "APIKey",
        "api-key",
        "aws.secret",
        "KEY2",
        "S3_KEY",
        "PRIVATE_KEY",
        "PUBLIC_KEY",
        "PUBLISHABLE_KEY",
        "CREDENTIALS",
        "CREDENTIAL_FILE",
        "SENTRY_DSN",
        "PASSWORD_SALT",
        "WEBHOOK_SIGNATURE",
        "TLS_CERT",
        "SSH_PASSPHRASE",
        "password",
        "Token",
    ];

    const NEGATIVE: &[&str] = &[
        "KEYBOARD_LAYOUT",
        "PASSENGER_COUNT",
        "COMPASS_DIR",
        "MONKEY",
        "AUTHOR",
        "AUTHORITY",
        "SECRETARY",
        "CERTIFIED",
        "PORT",
        "DATABASE_URL",
        "NODE_ENV",
        "keyboardLayout",
        "",
        "___",
    ];

    #[test]
    fn documented_positive_keys_are_secret() {
        for (index, key) in POSITIVE.iter().enumerate() {
            assert!(is_secret_key(key), "positive case {index} should be secret");
        }
    }

    #[test]
    fn documented_negative_keys_are_not_secret() {
        for (index, key) in NEGATIVE.iter().enumerate() {
            assert!(
                !is_secret_key(key),
                "negative case {index} should not be secret"
            );
        }
    }

    #[test]
    fn tokenizer_splits_separators_camel_case_acronyms_and_digits() {
        assert!(tokenize("stripeApiKey") == ["stripe", "Api", "Key"]);
        assert!(tokenize("APIKey") == ["API", "Key"]);
        assert!(tokenize("DB_PASS2") == ["DB", "PASS", "2"]);
        assert!(tokenize("a--b..c") == ["a", "b", "c"]);
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn secret_token_list_is_upper_case() {
        assert!(SECRET_TOKENS.iter().all(|t| *t == t.to_ascii_uppercase()));
    }
}
