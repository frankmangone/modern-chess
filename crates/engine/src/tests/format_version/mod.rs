//! A spec states the version of the spec format it is written for; this engine reads version 1.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::specs::{parse_game_spec_str, GameSpecError, FORMAT_VERSION};

    const CHATURANGA: &str = include_str!("../../../specs/chaturanga.json");

    fn chaturanga() -> Value {
        serde_json::from_str(CHATURANGA).expect("chaturanga is JSON")
    }

    fn read(spec: &Value) -> Result<crate::specs::GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    #[test]
    fn version_one_is_read() {
        assert_eq!(FORMAT_VERSION, 1);
        let mut spec = chaturanga();
        spec["format_version"] = json!(1);
        assert_eq!(read(&spec).expect("version 1 reads").format_version, 1);
    }

    #[test]
    fn a_spec_with_no_version_is_refused() {
        let mut spec = chaturanga();
        spec.as_object_mut().unwrap().remove("format_version");
        let error = read(&spec).expect_err("a spec with no version is refused");
        assert!(matches!(error, GameSpecError::MissingFormatVersion));
        assert!(error.to_string().contains("format_version"), "{error}");
    }

    #[test]
    fn any_other_version_is_refused_with_the_value_named() {
        for (written, shown) in [
            (json!(2), "2"),
            (json!(0), "0"),
            (json!(-1), "-1"),
            (json!(1.5), "1.5"),
            (json!("1"), "\"1\""),
            (json!(null), "null"),
            (json!(true), "true"),
        ] {
            let mut spec = chaturanga();
            spec["format_version"] = written;
            let error = read(&spec)
                .err()
                .unwrap_or_else(|| panic!("{shown} is refused"));
            assert!(
                matches!(&error, GameSpecError::UnsupportedFormatVersion(found) if found == shown),
                "{shown}: {error:?}"
            );
            assert!(error.to_string().contains(shown), "{error}");
        }
    }

    /// A spec for a newer format may have keys this one lacks: it is told about the version,
    /// not about the key.
    #[test]
    fn a_newer_spec_gets_the_version_error_before_any_other() {
        let mut spec = chaturanga();
        spec["format_version"] = json!(2);
        spec["a_block_version_2_has"] = json!([]);
        spec["board"] = json!("not even a board");
        let error = read(&spec).expect_err("refused");
        assert!(
            matches!(error, GameSpecError::UnsupportedFormatVersion(_)),
            "{error:?}"
        );
    }

    #[test]
    fn text_that_is_not_an_object_keeps_its_own_error() {
        assert!(matches!(
            parse_game_spec_str("{").err(),
            Some(GameSpecError::JsonError(_))
        ));
        assert!(matches!(
            parse_game_spec_str("[1]").err(),
            Some(GameSpecError::JsonError(_))
        ));
    }

    #[test]
    fn a_spec_written_back_keeps_its_version() {
        let mut spec = chaturanga();
        spec["format_version"] = json!(1);
        let game = read(&spec).expect("reads");
        let written: Value = serde_json::to_value(&game).expect("writes");
        assert_eq!(written["format_version"], json!(1));
        assert!(read(&written).is_ok(), "what is written back reads again");
    }

    /// A `GameSpec` built in Rust is judged too, not only one read from text.
    #[test]
    fn a_spec_built_with_another_version_is_refused_by_validation() {
        let mut spec = chaturanga();
        spec["format_version"] = json!(1);
        let mut game = read(&spec).expect("reads");
        game.format_version = 2;
        assert!(matches!(
            game.validate_specs().err(),
            Some(GameSpecError::UnsupportedFormatVersion(found)) if found == "2"
        ));
    }
}
