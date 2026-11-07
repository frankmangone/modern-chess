//! The two backend modules must never depend on each other, directly or through anything
//! else. Only the kernel (identifiers and the token contract) may sit between them.

use std::collections::{HashMap, HashSet};
use std::process::Command;

use serde_json::Value;

/// Package name -> names of the packages it depends on directly (workspace and external).
fn dependency_graph() -> HashMap<String, Vec<String>> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--offline"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata should run");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();

    let names_by_id: HashMap<&str, &str> = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["name"].as_str().unwrap()))
        .collect();

    metadata["resolve"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| {
            let name = names_by_id[node["id"].as_str().unwrap()].to_string();
            let deps = node["deps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| names_by_id[d["pkg"].as_str().unwrap()].to_string())
                .collect();
            (name, deps)
        })
        .collect()
}

fn reachable_from(graph: &HashMap<String, Vec<String>>, start: &str) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut stack = vec![start.to_string()];
    while let Some(next) = stack.pop() {
        for dep in graph.get(&next).into_iter().flatten() {
            if seen.insert(dep.clone()) {
                stack.push(dep.clone());
            }
        }
    }
    seen
}

#[test]
fn game_and_social_do_not_depend_on_each_other() {
    let graph = dependency_graph();

    // Sanity: the packages exist, and both really do use the kernel, so the checks
    // below are not vacuously true.
    for package in [
        "modern-chess-game",
        "modern-chess-social",
        "modern-chess-game-api",
    ] {
        assert!(
            graph.contains_key(package),
            "{package} is missing from the workspace"
        );
    }
    assert!(reachable_from(&graph, "modern-chess-game").contains("modern-chess-kernel"));
    assert!(reachable_from(&graph, "modern-chess-social").contains("modern-chess-kernel"));

    let game_side = ["modern-chess-game", "modern-chess-game-api"];
    let social_side = ["modern-chess-social", "modern-chess-social-api"];

    for package in game_side {
        let reach = reachable_from(&graph, package);
        for forbidden in social_side {
            assert!(
                !reach.contains(forbidden),
                "{package} must not depend on {forbidden}"
            );
        }
    }
    for package in social_side.into_iter().filter(|p| graph.contains_key(*p)) {
        let reach = reachable_from(&graph, package);
        for forbidden in game_side {
            assert!(
                !reach.contains(forbidden),
                "{package} must not depend on {forbidden}"
            );
        }
    }
}
