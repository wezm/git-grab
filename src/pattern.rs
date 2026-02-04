#[derive(PartialEq, Clone, Copy, Debug)]
pub enum GrabPatternPlaceholder {
    Home,
    Host,
    Path,
    Owner,
    Repo,
}

#[derive(PartialEq, Clone, Debug)]
pub enum GrabPatternComponent {
    Placeholder {
        placeholder: GrabPatternPlaceholder,
        leading_slash: bool,
        trailing_slash: bool,
    },
    Literal(String),
}

#[derive(PartialEq, Clone, Debug)]
pub struct GrabPattern(pub Vec<GrabPatternComponent>);

#[derive(PartialEq, Clone, Debug)]
pub enum GrabPatternParseError {
    EmptyPlaceholder,
    UnknownPlaceholder(String),
    UnclosedPlaceholder(String),
    BlankPattern,
}

impl GrabPattern {
    pub fn try_parse(input: &str) -> Result<Self, GrabPatternParseError> {
        let mut chars: std::str::Chars<'_> = input.chars();

        #[derive(PartialEq)]
        enum ParsingMode {
            Literal,
            Placeholder,
            Escape,
        }

        let mut result = Vec::new();

        let mut mode = ParsingMode::Literal;
        let mut current = String::new();

        while let Some(c) = chars.next() {
            if c == '{' && mode == ParsingMode::Literal {
                mode = ParsingMode::Placeholder;
                if !current.is_empty() {
                    result.push(GrabPatternComponent::Literal(current));
                    current = String::new();
                }
                continue;
            }

            if c == '{' && mode == ParsingMode::Placeholder {
                current.push('{');
                mode = ParsingMode::Escape;
                continue;
            }

            if c == '{' && mode == ParsingMode::Literal {
                mode = ParsingMode::Placeholder;
                if !current.is_empty() {
                    result.push(GrabPatternComponent::Literal(current));
                    current = String::new();
                }
                continue;
            }

            if c == '}' && mode == ParsingMode::Escape {
                if let Some('}') = chars.next() {
                    current.push('}');
                    mode = ParsingMode::Literal;
                    continue;
                }
                current.push('}');
                continue;
            }

            if c == '}' && mode == ParsingMode::Placeholder {
                mode = ParsingMode::Literal;

                if current.is_empty() {
                    return Err(GrabPatternParseError::EmptyPlaceholder);
                }

                let placeholder_str = current.as_str();
                let mut start = 0;
                let mut end = placeholder_str.len();
                let mut leading_slash = false;
                let mut trailing_slash = false;

                if placeholder_str.starts_with('/') {
                    leading_slash = true;
                    start += 1;
                }

                if placeholder_str.ends_with('/') {
                    trailing_slash = true;
                    end -= 1;
                }

                let placeholder = match &placeholder_str[start..end] {
                    "home" => GrabPatternPlaceholder::Home,
                    "host" => GrabPatternPlaceholder::Host,
                    "path" => GrabPatternPlaceholder::Path,
                    "owner" => GrabPatternPlaceholder::Owner,
                    "repo" => GrabPatternPlaceholder::Repo,
                    "" => return Err(GrabPatternParseError::EmptyPlaceholder),
                    _ => return Err(GrabPatternParseError::UnknownPlaceholder(current)),
                };

                result.push(GrabPatternComponent::Placeholder {
                    placeholder,
                    leading_slash,
                    trailing_slash,
                });

                current = String::new();
                continue;
            }

            current.push(c);
        }

        if mode == ParsingMode::Placeholder {
            return Err(GrabPatternParseError::UnclosedPlaceholder(current));
        }

        if !current.is_empty() {
            result.push(GrabPatternComponent::Literal(current));
        }

        if result.is_empty()
            || result.iter().all(|comp| {
                matches!(comp,
                    GrabPatternComponent::Literal(s) if s.trim().is_empty()
                )
            })
        {
            return Err(GrabPatternParseError::BlankPattern);
        }

        Ok(Self(result))
    }
}

impl Default for GrabPattern {
    fn default() -> Self {
        // Default pattern: ~/src/{host/}{path/}
        GrabPattern(vec![
            GrabPatternComponent::Literal("~/src/".into()),
            GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Host,
                leading_slash: false,
                trailing_slash: true,
            },
            GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Path,
                leading_slash: false,
                trailing_slash: true,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_pattern() {
        let parsed = GrabPattern::try_parse("~/src/{host/}{path/}").unwrap();
        assert_eq!(parsed, GrabPattern::default());
    }

    #[test]
    fn test_parse_valid_placeholders() {
        assert_eq!(
            GrabPattern::try_parse("{home}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Home,
                leading_slash: false,
                trailing_slash: false,
            }]))
        );

        assert_eq!(
            GrabPattern::try_parse("{host}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Host,
                leading_slash: false,
                trailing_slash: false,
            }]))
        );

        assert_eq!(
            GrabPattern::try_parse("{path}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Path,
                leading_slash: false,
                trailing_slash: false,
            }]))
        );

        assert_eq!(
            GrabPattern::try_parse("{owner}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Owner,
                leading_slash: false,
                trailing_slash: false,
            }]))
        );

        assert_eq!(
            GrabPattern::try_parse("{repo}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Repo,
                leading_slash: false,
                trailing_slash: false,
            }]))
        );
    }

    #[test]
    fn test_parse_leading_slash() {
        assert_eq!(
            GrabPattern::try_parse("{/home}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Home,
                leading_slash: true,
                trailing_slash: false,
            }]))
        );
    }

    #[test]
    fn test_parse_trailing_slash() {
        assert_eq!(
            GrabPattern::try_parse("{home/}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Placeholder {
                placeholder: GrabPatternPlaceholder::Home,
                leading_slash: false,
                trailing_slash: true,
            }]))
        );
    }

    #[test]
    fn test_parse_invalid_casing() {
        assert_eq!(
            GrabPattern::try_parse("{Home}"),
            Err(GrabPatternParseError::UnknownPlaceholder("Home".into()))
        );
        assert_eq!(
            GrabPattern::try_parse("{HOME}"),
            Err(GrabPatternParseError::UnknownPlaceholder("HOME".into()))
        );
    }

    #[test]
    fn test_parse_escaping() {
        assert_eq!(
            GrabPattern::try_parse("{{home}}"),
            Ok(GrabPattern(vec![GrabPatternComponent::Literal(
                "{home}".into()
            )]))
        );
    }

    #[test]
    fn test_parse_missing_closing_brace() {
        assert_eq!(
            GrabPattern::try_parse("{home"),
            Err(GrabPatternParseError::UnclosedPlaceholder("home".into()))
        );
    }

    #[test]
    fn test_parse_empty_placeholder() {
        assert_eq!(
            GrabPattern::try_parse("{}"),
            Err(GrabPatternParseError::EmptyPlaceholder)
        );
    }

    #[test]
    fn test_parse_unknown_placeholder() {
        assert_eq!(
            GrabPattern::try_parse("{unknown}"),
            Err(GrabPatternParseError::UnknownPlaceholder("unknown".into()))
        );
        assert_eq!(
            GrabPattern::try_parse("{abc}"),
            Err(GrabPatternParseError::UnknownPlaceholder("abc".into()))
        );
        assert_eq!(
            GrabPattern::try_parse("{🐈}"),
            Err(GrabPatternParseError::UnknownPlaceholder("🐈".into()))
        );
    }

    #[test]
    fn test_parse_blank() {
        assert_eq!(
            GrabPattern::try_parse(""),
            Err(GrabPatternParseError::BlankPattern)
        );
        assert_eq!(
            GrabPattern::try_parse(" "),
            Err(GrabPatternParseError::BlankPattern)
        );
        assert_eq!(
            GrabPattern::try_parse("\t"),
            Err(GrabPatternParseError::BlankPattern)
        );
    }
}
