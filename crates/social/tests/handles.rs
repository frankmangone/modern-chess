//! What a person may choose as a username, in words a person can read.

use modern_chess_social::handles::{choose, HandleProblem};

fn problem(raw: &str) -> HandleProblem {
    choose(raw).expect_err(raw)
}

#[test]
fn an_ordinary_name_is_accepted_as_written() {
    for name in [
        "ada",
        "Ada_Lovelace",
        "k9-rook",
        "A1b",
        "x".repeat(24).as_str(),
    ] {
        assert_eq!(choose(name).unwrap().as_str(), name);
    }
}

#[test]
fn a_name_must_be_3_to_24_characters() {
    assert_eq!(problem(""), HandleProblem::TooShort);
    assert_eq!(problem("ab"), HandleProblem::TooShort);
    assert_eq!(problem(&"x".repeat(25)), HandleProblem::TooLong);
}

#[test]
fn only_letters_numbers_dash_and_underscore_are_allowed() {
    for name in [
        "ada lovelace",
        "ada!",
        "ada.l",
        "añda",
        "日本語名前",
        " ada",
        "ada ",
        "ada\n",
    ] {
        assert_eq!(problem(name), HandleProblem::BadCharacters, "{name:?}");
    }
}

#[test]
fn a_name_starts_with_a_letter_or_a_number() {
    for name in ["-ada", "_ada", "---", "___"] {
        assert_eq!(problem(name), HandleProblem::BadStart, "{name}");
    }
}

#[test]
fn names_we_keep_for_ourselves_are_refused_in_any_letter_case() {
    for name in [
        "admin",
        "Admin",
        "HUBBABLE",
        "support",
        "moderator",
        "staff",
        "system",
    ] {
        assert_eq!(problem(name), HandleProblem::Reserved, "{name}");
    }
    // Containing a reserved word is fine; being one is not.
    assert!(choose("admiral").is_ok());
    assert!(choose("hubbable_fan").is_ok());
}

#[test]
fn the_names_we_make_are_not_for_choosing() {
    for name in [
        "Guest-1234",
        "guest-1",
        "PLAYER-99",
        "player-abc",
        "deleted-0a1b2c",
        "Deleted-x",
    ] {
        assert_eq!(problem(name), HandleProblem::Generated, "{name}");
    }
    // Only the dash form is ours.
    assert!(choose("guest_1").is_ok());
    assert!(choose("playerone").is_ok());
}

#[test]
fn every_problem_has_a_plain_message_and_a_code() {
    let all = [
        HandleProblem::TooShort,
        HandleProblem::TooLong,
        HandleProblem::BadCharacters,
        HandleProblem::BadStart,
        HandleProblem::Reserved,
        HandleProblem::Generated,
    ];
    let codes: std::collections::HashSet<_> = all.iter().map(|p| p.code()).collect();
    assert_eq!(codes.len(), all.len());
    for p in all {
        assert!(p.message().ends_with('.'), "{}", p.message());
    }
}
