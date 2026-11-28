use std::fs;
use std::path::PathBuf;

use modern_chess_sandbox::{Error, Limits, Sandbox};
use modern_chess_sandbox_protocol::{answer, Request, Response, Seeing, Step};

fn specs() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/specs");
    let mut found: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "json"))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                fs::read_to_string(entry.path()).unwrap(),
            )
        })
        .collect();
    found.sort();
    found
}

fn sandbox() -> Sandbox {
    Sandbox::new(Limits::default()).unwrap()
}

fn chess() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/specs/chess.json"))
        .unwrap()
}

/// The sandbox is the engine behind a wall, not another engine: the same question gets the same
/// answer in it as in this process, for every game of the catalogue.
#[test]
fn every_catalogue_game_answers_in_the_sandbox_as_it_does_in_process() {
    let sandbox = sandbox();
    let ask = |request: Request| {
        let (inside, _) = sandbox.call_blocking(&request).unwrap();
        assert_eq!(inside, answer(request.clone()), "{request:?}");
        inside
    };
    for (file, rules) in specs() {
        let checked = ask(Request::Check {
            rules: rules.clone(),
        });
        assert!(
            matches!(checked, Response::Checked { .. }),
            "{file}: {checked:?}"
        );
        ask(Request::Picture {
            rules: rules.clone(),
        });
        let Response::Opened { state, seats } = ask(Request::Open {
            rules: rules.clone(),
            seed: 7,
        }) else {
            panic!("{file} did not open");
        };
        assert!(!seats.is_empty(), "{file}");
        ask(Request::View {
            rules: rules.clone(),
            state: state.clone(),
            revision: 0,
            seeing: Seeing::Everyone,
        });
        ask(Request::View {
            rules,
            state,
            revision: 0,
            seeing: Seeing::Player {
                seat: seats[0].clone(),
            },
        });
    }
}

#[test]
fn a_move_is_the_same_in_the_sandbox_and_the_rules_it_refuses_are_refused() {
    let sandbox = sandbox();
    let rules = chess();
    let (Response::Opened { state, seats }, _) = sandbox
        .call_blocking(&Request::Open {
            rules: rules.clone(),
            seed: 1,
        })
        .unwrap()
    else {
        panic!("chess did not open");
    };
    let white = seats[0].clone();
    let black = seats[1].clone();
    let choose = |place: &str| Step::Choose {
        place: Some(place.into()),
        code: None,
        held: None,
        action: None,
    };
    let play = |state: &serde_json::Value, revision: u64, seat: &str, step: Step| Request::Play {
        rules: rules.clone(),
        state: state.clone(),
        revision,
        seat: seat.into(),
        step,
    };
    let ask = |request: Request| {
        let (inside, _) = sandbox.call_blocking(&request).unwrap();
        assert_eq!(inside, answer(request), "in the sandbox as out of it");
        inside
    };

    let Response::Played { state: picked, .. } = ask(play(&state, 0, &white, choose("4,1"))) else {
        panic!("4,1 was not accepted");
    };
    let Response::Played {
        state: moved,
        standing,
        ..
    } = ask(play(&picked, 1, &white, choose("4,3")))
    else {
        panic!("4,3 was not accepted");
    };
    assert_eq!(standing, modern_chess_sandbox_protocol::Standing::Playing);

    assert_eq!(
        ask(play(&moved, 2, &white, choose("4,4"))),
        Response::NotYourTurn
    );
    assert!(matches!(
        ask(play(&moved, 2, &black, choose("z9"))),
        Response::Rejected { .. }
    ));
    assert!(matches!(
        ask(play(
            &moved,
            2,
            &black,
            Step::Choose {
                place: None,
                code: None,
                held: None,
                action: None
            }
        )),
        Response::Rejected { .. }
    ));
}

#[test]
fn rules_the_engine_will_not_read_are_an_answer_not_a_fault() {
    let (response, _) = sandbox()
        .call_blocking(&Request::Check {
            rules: "{\n \"name\": ".into(),
        })
        .unwrap();
    let Response::Unreadable(problem) = response else {
        panic!("{response:?}");
    };
    assert_eq!(problem.line, Some(2));
}

#[test]
fn a_request_that_is_not_one_is_a_failed_answer() {
    // Straight to the module's own entry: the sandbox hands back what the engine says.
    assert!(modern_chess_sandbox_protocol::handle("{\"op\":\"nonsense\"}").contains("\"failed\""));
}

/// The engine bounds what rules may ask for (a board's size, a step that goes nowhere), and the
/// bounds hold in the sandbox: they come back as answers, cheaply, and the sandbox carries on.
#[test]
fn rules_that_ask_too_much_are_an_answer_and_the_sandbox_carries_on() {
    let sandbox = sandbox();
    let mut rules: serde_json::Value = serde_json::from_str(&chess()).unwrap();
    rules["board"] = serde_json::json!({"dimensions": [255, 255, 255]});
    let (response, cost) = sandbox
        .call_blocking(&Request::Picture {
            rules: rules.to_string(),
        })
        .unwrap();
    assert!(matches!(response, Response::Failed { .. }), "{response:?}");
    assert!(cost.fuel < 100_000_000, "{cost:?}");

    let (fine, _) = sandbox
        .call_blocking(&Request::Check { rules: chess() })
        .unwrap();
    assert!(matches!(fine, Response::Checked { .. }));
}

#[test]
fn a_call_that_uses_up_its_fuel_is_told_so() {
    let starved = Sandbox::new(Limits {
        fuel: 10_000,
        ..Limits::default()
    })
    .unwrap();
    let outcome = starved.call_blocking(&Request::Check { rules: chess() });
    assert!(matches!(outcome, Err(Error::OutOfFuel)), "{outcome:?}");
}

#[test]
fn a_call_that_asks_for_more_memory_than_it_may_have_is_stopped() {
    let tight = Sandbox::new(Limits {
        memory: 256 << 10,
        ..Limits::default()
    })
    .unwrap();
    let outcome = tight.call_blocking(&Request::Open {
        rules: chess(),
        seed: 1,
    });
    assert!(matches!(outcome, Err(Error::Trapped(_))), "{outcome:?}");
}

#[tokio::test]
async fn calls_from_many_tasks_all_get_their_answer() {
    let sandbox = sandbox();
    let rules = chess();
    let mut tasks = Vec::new();
    for _ in 0..24 {
        let (sandbox, rules) = (sandbox.clone(), rules.clone());
        tasks.push(tokio::spawn(async move {
            sandbox.call(&Request::Check { rules }).await
        }));
    }
    for task in tasks {
        let (response, _) = task.await.unwrap().unwrap();
        assert!(matches!(response, Response::Checked { .. }));
    }
}

#[tokio::test]
async fn a_call_that_waits_too_long_for_its_turn_is_turned_away() {
    let one_at_a_time = Sandbox::new(Limits {
        concurrency: 1,
        patience: std::time::Duration::from_millis(1),
        ..Limits::default()
    })
    .unwrap();
    let rules = chess();
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let (sandbox, rules) = (one_at_a_time.clone(), rules.clone());
        tasks.push(tokio::spawn(async move {
            sandbox.call(&Request::Open { rules, seed: 1 }).await
        }));
    }
    let mut turned_away = 0;
    for task in tasks {
        if matches!(task.await.unwrap(), Err(Error::Busy)) {
            turned_away += 1;
        }
    }
    assert!(turned_away > 0, "nobody was turned away");
}

/// What the heaviest everyday calls cost, so the budget has a reason: run with `--nocapture` to
/// read the table. The test itself holds every catalogue call to a small part of the budget.
#[test]
fn every_catalogue_call_costs_a_small_part_of_the_budget() {
    let sandbox = sandbox();
    let budget = Limits::default().fuel;
    let mut worst = (0u64, String::new());
    for (file, rules) in specs() {
        let (opened, open_cost) = sandbox
            .call_blocking(&Request::Open {
                rules: rules.clone(),
                seed: 3,
            })
            .unwrap();
        let Response::Opened { state, seats } = opened else {
            panic!("{file}")
        };
        let (_, picture) = sandbox
            .call_blocking(&Request::Picture {
                rules: rules.clone(),
            })
            .unwrap();
        let (_, view) = sandbox
            .call_blocking(&Request::View {
                rules: rules.clone(),
                state,
                revision: 0,
                seeing: Seeing::Player {
                    seat: seats[0].clone(),
                },
            })
            .unwrap();
        let heaviest = open_cost.fuel.max(picture.fuel).max(view.fuel);
        println!(
            "{file:<28} open {:>12}  picture {:>12}  view {:>12}",
            open_cost.fuel, picture.fuel, view.fuel
        );
        if heaviest > worst.0 {
            worst = (heaviest, file);
        }
    }
    println!("heaviest: {} costs {} of {budget}", worst.1, worst.0);
    assert!(
        worst.0 * 20 < budget,
        "{} costs {} of a budget of {budget}",
        worst.1,
        worst.0
    );
}
