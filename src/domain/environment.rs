//! Environment labels inferred from dotenv file names.
//! Labels are visual hints and do not imply extra protection.

/// A visual hint about a dotenv file's intended environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvironmentLabel {
    /// Matches `development`, `dev`, or `local`.
    Development,
    /// Matches `staging` or `stage`.
    Staging,
    /// Matches `production` or `prod`.
    Production,
    /// Has no matching token or has tokens for multiple environments.
    Other,
}

/// Infers a label from whole tokens in a bare file name.
///
/// Every non-ASCII-alphanumeric character separates tokens, including path
/// separators. Matching ignores ASCII case. Repeated matches for the same
/// label are allowed.
///
/// # Examples
///
/// ```
/// use envy::domain::environment::{EnvironmentLabel, infer_from_file_name};
///
/// assert_eq!(infer_from_file_name(".env.production"), EnvironmentLabel::Production);
/// assert_eq!(infer_from_file_name(".env.prod.local"), EnvironmentLabel::Other);
/// ```
pub fn infer_from_file_name(file_name: &str) -> EnvironmentLabel {
    use EnvironmentLabel::{Development, Other, Production, Staging};

    let mut inferred = Other;
    for token in file_name.split(|character: char| !character.is_ascii_alphanumeric()) {
        let label =
            if token.eq_ignore_ascii_case("production") || token.eq_ignore_ascii_case("prod") {
                Production
            } else if token.eq_ignore_ascii_case("staging") || token.eq_ignore_ascii_case("stage") {
                Staging
            } else if token.eq_ignore_ascii_case("development")
                || token.eq_ignore_ascii_case("dev")
                || token.eq_ignore_ascii_case("local")
            {
                Development
            } else {
                continue;
            };

        if inferred != Other && inferred != label {
            return Other;
        }
        inferred = label;
    }

    inferred
}

#[cfg(test)]
mod tests {
    use super::EnvironmentLabel::{Development, Other, Production, Staging};
    use super::infer_from_file_name;

    #[test]
    fn infers_environment_from_whole_file_name_tokens() {
        for (file_name, expected) in [
            (".env.production", Production),
            ("prod.env", Production),
            (".env.local", Development),
            (".env.example", Other),
            (".env", Other),
            (".env.prod.local", Other),
            (".env.staging", Staging),
            (".env.stage", Staging),
            (".env.development", Development),
            (".env.dev", Development),
            (".env.dev.local", Development),
            (".ENV.PROD", Production),
            (".env.preprod", Other),
            (".env.devops", Other),
            ("", Other),
            (".env-staging", Staging),
            (".env_dev", Development),
            ("staging.prod.env", Other),
            (".env.production2", Other),
            ("prod/.env", Production),
            (".envédev", Development),
        ] {
            assert_eq!(infer_from_file_name(file_name), expected, "{file_name:?}");
        }
    }
}
