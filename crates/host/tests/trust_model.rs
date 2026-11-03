//! Model-based property test for the Trust lifecycle (docs/VERIFICACAO-FORMAL.md, candidate 1).
//! A tiny reference model of "open workspace + stored decision per path + in-memory trusted flag"
//! is driven with seeded random operation sequences against the real `Host`; after every step the
//! host must agree with the model on the invariants listed there. No new dependency: a fixed-seed
//! generator keeps failures reproducible (the seed and the sequence are in the panic message).

use openridr_host::{Host, HostError};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const PATHS: usize = 2;
const WALKS: u64 = 60;
const STEPS: usize = 14;

#[derive(Clone, Copy, Debug)]
enum Op {
    Open(usize),
    Close,
    Grant { remember: bool },
    Revoke,
    Set(usize, bool),
}

/// Reference model: what the stored decision and the in-memory flag must be after each operation.
#[derive(Default)]
struct Model {
    open: Option<usize>,
    trusted: bool,
    /// `Some(true)` trusted, `Some(false)` restricted, `None` never decided.
    store: [Option<bool>; PATHS],
}

impl Model {
    /// `None` = the host must refuse; `Some(())` = it must accept.
    fn apply(&mut self, op: Op) -> Option<()> {
        match op {
            Op::Open(i) => {
                self.open = Some(i);
                self.trusted = self.store[i] == Some(true);
            }
            Op::Close => {
                self.open = None;
                self.trusted = false;
            }
            Op::Grant { remember } => {
                let i = self.open?;
                if remember {
                    self.store[i] = Some(true);
                }
                self.trusted = true;
            }
            Op::Revoke => {
                let i = self.open?;
                self.store[i] = Some(false);
                self.trusted = false;
            }
            Op::Set(j, trust) if self.open == Some(j) => {
                return self.apply(if trust { Op::Grant { remember: true } } else { Op::Revoke });
            }
            Op::Set(j, trust) => self.store[j] = Some(trust),
        }
        Some(())
    }
}

fn next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed >> 33
}

fn random_op(seed: &mut u64) -> Op {
    let path = (next(seed) % PATHS as u64) as usize;
    match next(seed) % 6 {
        0 | 1 => Op::Open(path),
        2 => Op::Close,
        3 => Op::Grant { remember: next(seed) % 2 == 0 },
        4 => Op::Revoke,
        _ => Op::Set(path, next(seed) % 2 == 0),
    }
}

fn call(host: &Host, op: Op, paths: &[PathBuf]) -> Result<Value, HostError> {
    match op {
        Op::Open(i) => host.dispatch("workspace.open", json!({ "path": paths[i] })),
        Op::Close => host.dispatch("workspace.close", json!({})),
        Op::Grant { remember } => host.dispatch("trust.grant", json!({ "remember": remember })),
        Op::Revoke => host.dispatch("trust.revoke", json!({})),
        Op::Set(j, trust) => host.dispatch(
            "trust.set",
            json!({ "path": paths[j], "decision": if trust { "trusted" } else { "restricted" } }),
        ),
    }
}

fn label(decision: Option<bool>) -> Value {
    match decision {
        Some(true) => json!("trusted"),
        Some(false) => json!("restricted"),
        None => Value::Null,
    }
}

fn project(dir: &Path, name: &str) -> PathBuf {
    let root = dir.join(name);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("App.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
    )
    .unwrap();
    fs::write(root.join("App.cs"), "class C {}\n").unwrap();
    // The decision is keyed by the resolved workspace path, which for a project is the .csproj.
    fs::canonicalize(root.join("App.csproj")).unwrap()
}

/// Invariants I1-I5 of docs/VERIFICACAO-FORMAL.md, checked against the real host after each step.
fn check(host: &Host, model: &Model, paths: &[PathBuf], home: &Path, context: &str) {
    let state = host.dispatch("workspace.state", json!({})).unwrap();
    let granted = state["trust"]["granted"] == true;
    // I1: the host flag is exactly the model flag, and a trusted flag needs an open workspace.
    assert_eq!(granted, model.trusted, "I1 flag: {context}");
    assert!(!granted || model.open.is_some(), "I1 trusted without workspace: {context}");
    // I2: mode and the stored decision shown for the open workspace follow the model.
    match model.open {
        Some(i) => {
            assert_eq!(state["workspace"]["path"], json!(paths[i]), "workspace: {context}");
            assert_eq!(
                state["workspace"]["mode"],
                if model.trusted { "trusted" } else { "restricted" },
                "I2 mode: {context}"
            );
            assert_eq!(state["trust"]["decision"], label(model.store[i]), "I2 decision: {context}");
        }
        None => assert!(state["workspace"].is_null(), "no workspace: {context}"),
    }
    // I3: the gate. An executable method is refused with trust_required exactly when not trusted.
    let gate = host.dispatch("terminal.resize", json!({ "cols": 80, "rows": 24 }));
    let refused = matches!(&gate, Err(error) if error.code == "trust_required");
    assert_eq!(refused, !model.trusted, "I3 gate (trusted={}): {context}", model.trusted);
    // I4: the decision stored per path (read back through the host) is the model's, for every path.
    for (i, path) in paths.iter().enumerate() {
        let got = host.dispatch("trust.get", json!({ "path": path })).unwrap();
        assert_eq!(got["decision"], label(model.store[i]), "I4 trust.get {i}: {context}");
    }
    // I5: durability. The file on disk holds exactly the decisions the model made.
    let on_disk: Value = fs::read_to_string(home.join("trusted.json"))
        .map(|text| serde_json::from_str(&text).unwrap())
        .unwrap_or_else(|_| json!({}));
    for (i, path) in paths.iter().enumerate() {
        assert_eq!(
            on_disk.get(path.to_string_lossy().as_ref()).cloned().unwrap_or(Value::Null),
            label(model.store[i]),
            "I5 trusted.json {i}: {context}"
        );
    }
}

#[test]
fn trust_lifecycle_matches_the_reference_model_on_random_sequences() {
    let dir = tempfile::tempdir().unwrap();
    let paths: Vec<PathBuf> = (0..PATHS).map(|i| project(dir.path(), &format!("p{i}"))).collect();
    std::env::remove_var("OPENRIDR_TRUST_STORE");

    for walk in 0..WALKS {
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("OPENRIDR_HOME", home.path());
        let host = Host::new();
        let mut model = Model::default();
        let mut seed = walk + 1;
        let mut trace: Vec<Op> = Vec::new();
        for _ in 0..STEPS {
            let op = random_op(&mut seed);
            trace.push(op);
            let context = format!("walk {walk} trace {trace:?}");
            let expected = model.apply(op);
            let result = call(&host, op, &paths);
            assert_eq!(result.is_ok(), expected.is_some(), "accept/refuse of {op:?}: {context}");
            check(&host, &model, &paths, home.path(), &context);
            // A restart (new Host, same home) keeps only what was persisted: reopening the open path
            // is trusted exactly when the stored decision says trusted.
            if let (Op::Open(i), true) = (op, walk % 3 == 0) {
                let restarted = Host::new();
                restarted.dispatch("workspace.open", json!({ "path": paths[i] })).unwrap();
                let granted = restarted.dispatch("workspace.state", json!({})).unwrap()["trust"]["granted"] == true;
                assert_eq!(granted, model.store[i] == Some(true), "I6 restart: {context}");
            }
        }
    }
    std::env::remove_var("OPENRIDR_HOME");
}
