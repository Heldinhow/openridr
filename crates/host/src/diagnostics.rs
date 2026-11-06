//! Diagnostics snapshots by origin, analysis session, document and revision.
//!
//! csharp-ls never sends a document version, so a publication can only be attributed by
//! protocol order: the client revision at reception. An older revision is never presented as
//! current; it stays visible as `awaiting` (same session, newer buffer) or `stale` (other
//! session or unavailable analysis). Origins are kept apart so a future Build origin can be
//! added without merging or erasing the LSP one.

use crate::lsp::Publication;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub const LSP_ORIGIN: &str = "lsp";
pub const LSP_LABEL: &str = "LSP · csharp-ls";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    pub id: String,
    pub label: String,
}

impl Origin {
    pub fn lsp() -> Self {
        Self {
            id: LSP_ORIGIN.to_string(),
            label: LSP_LABEL.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Compiler,
    Analyzer,
    Unknown,
}

/// Only an explicit marker in the source may select Compiler/Analyzer. A code prefix such as
/// `CSxxxx` is not an analyzer/compiler marker and never changes the category.
pub fn category_of(source: Option<&str>) -> Category {
    match source.map(str::to_ascii_lowercase).as_deref() {
        Some(source) if source.contains("analyzer") => Category::Analyzer,
        Some(source) if source.contains("compiler") => Category::Compiler,
        _ => Category::Unknown,
    }
}

/// LSP severity values: 1 error, 2 warning, 3 information, 4 hint. A missing/unknown value is
/// reported as unknown instead of being guessed.
pub fn severity_label(raw: Option<u64>) -> &'static str {
    match raw {
        Some(1) => "error",
        Some(2) => "warning",
        Some(3) => "information",
        Some(4) => "hint",
        _ => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

fn position_of(range: &Value) -> Option<Position> {
    let start = range.get("start")?;
    Some(Position {
        line: start.get("line")?.as_u64()?,
        character: start.get("character")?.as_u64()?,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemItem {
    pub id: String,
    pub severity: Option<u64>,
    pub severity_label: &'static str,
    pub severity_raw: Option<u64>,
    pub code: Value,
    pub source: Option<String>,
    pub category: Category,
    pub message: String,
    /// Start position when the source published a usable range; navigation never guesses one.
    pub position: Option<Position>,
    pub range: Value,
    pub code_description: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSnapshot {
    pub id: String,
    pub origin: Origin,
    pub session: u64,
    pub workspace_id: u64,
    pub context_id: u64,
    pub uri: String,
    pub path: Option<String>,
    pub received_at: u64,
    /// Client revision attributed at reception. csharp-ls sends no version.
    pub client_revision: Option<i64>,
    /// Current client revision of the document in the current session, when known.
    pub current_revision: Option<i64>,
    pub revision_known: bool,
    pub server_version: Value,
    /// `current`, `awaiting` (newer buffer) or `stale` (other session/unavailable analysis).
    pub state: &'static str,
    pub stale_reason: Option<String>,
    pub items: Vec<ProblemItem>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub error: u64,
    pub warning: u64,
    pub information: u64,
    pub hint: u64,
    pub unknown: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AwaitingDocument {
    pub origin: Origin,
    pub session: u64,
    pub path: String,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginSummary {
    pub origin: Origin,
    pub session: u64,
    pub current_items: usize,
    pub stale_items: usize,
    pub unavailable: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsView {
    pub session: u64,
    pub origin: Origin,
    pub snapshots: Vec<DocumentSnapshot>,
    pub awaiting: Vec<AwaitingDocument>,
    pub counts: Counts,
    pub summaries: Vec<OriginSummary>,
    pub current_items: usize,
    pub stale_items: usize,
    pub coverage: &'static str,
    pub coverage_note: Option<String>,
    /// Analyzer execution is not verified by either source in this slice.
    pub analyzer_coverage: &'static str,
    pub analysis_unavailable: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentRevision {
    pub uri: String,
    pub path: String,
    pub revision: i64,
}

#[derive(Debug, Clone)]
struct CurrentOrigin {
    label: String,
    session: u64,
    workspace_id: u64,
    context_id: u64,
}

#[derive(Default)]
pub struct Store {
    current: BTreeMap<String, CurrentOrigin>,
    unavailable: BTreeMap<String, String>,
    snapshots: BTreeMap<String, DocumentSnapshot>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.current.clear();
        self.unavailable.clear();
        self.snapshots.clear();
    }

    /// A new analysis session. Older sessions stay in the store, but only this one can receive
    /// publications or be presented as current.
    pub fn begin_session(
        &mut self,
        origin: &Origin,
        session: u64,
        workspace_id: u64,
        context_id: u64,
    ) {
        self.unavailable.remove(&origin.id);
        self.current.insert(
            origin.id.clone(),
            CurrentOrigin {
                label: origin.label.clone(),
                session,
                workspace_id,
                context_id,
            },
        );
    }

    pub fn mark_unavailable(&mut self, origin_id: &str, reason: impl Into<String>) {
        // The first reason after a session starts is the specific one (revoke, stop, failure);
        // later snapshot polling must not replace it with a generic message.
        self.unavailable
            .entry(origin_id.to_string())
            .or_insert_with(|| reason.into());
    }

    /// Stores a publication only for the current origin/session/context. A late publication
    /// from an older session is ignored instead of clearing or overwriting current diagnostics.
    pub fn ingest(&mut self, origin: &Origin, publication: &Publication) -> bool {
        let Some(current) = self.current.get(&origin.id) else {
            return false;
        };
        if current.session != publication.session
            || current.workspace_id != publication.workspace_id
            || current.context_id != publication.context_id
        {
            return false;
        }
        if self.unavailable.contains_key(&origin.id) {
            return false;
        }
        let id = format!("{}:{}:{}", origin.id, publication.session, publication.uri);
        let items = publication
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| ProblemItem {
                id: format!("{id}:{index}"),
                severity: item.severity,
                severity_label: severity_label(item.severity),
                severity_raw: item.severity,
                code: item.code.clone(),
                source: item.source.clone(),
                category: category_of(item.source.as_deref()),
                message: item.message.clone(),
                position: position_of(&item.range),
                range: item.range.clone(),
                code_description: item.code_description.clone(),
                stale: false,
            })
            .collect();
        self.snapshots.insert(
            id.clone(),
            DocumentSnapshot {
                id,
                origin: origin.clone(),
                session: publication.session,
                workspace_id: publication.workspace_id,
                context_id: publication.context_id,
                uri: publication.uri.clone(),
                path: publication.path.clone(),
                received_at: publication.received_at,
                client_revision: publication.client_revision,
                current_revision: publication.client_revision,
                revision_known: publication.client_revision.is_some(),
                server_version: publication.server_version.clone(),
                state: "current",
                stale_reason: None,
                items,
            },
        );
        true
    }

    pub fn view(
        &self,
        documents: &[DocumentRevision],
        coverage: &'static str,
        coverage_note: Option<String>,
    ) -> DiagnosticsView {
        let lsp = Origin::lsp();
        let session = self
            .current
            .get(LSP_ORIGIN)
            .map(|current| current.session)
            .unwrap_or(0);
        let analysis_unavailable = self.unavailable.get(LSP_ORIGIN).cloned();
        let mut snapshots: Vec<DocumentSnapshot> = self.snapshots.values().cloned().collect();
        for snapshot in &mut snapshots {
            update_state(snapshot, &self.current, &self.unavailable, documents);
        }
        snapshots.sort_by(|left, right| {
            right
                .received_at
                .cmp(&left.received_at)
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut counts = Counts::default();
        let mut current_items = 0;
        let mut stale_items = 0;
        let mut summaries: BTreeMap<String, OriginSummary> = BTreeMap::new();
        for snapshot in &snapshots {
            let summary = summaries
                .entry(snapshot.origin.id.clone())
                .or_insert_with(|| OriginSummary {
                    origin: snapshot.origin.clone(),
                    session: self
                        .current
                        .get(&snapshot.origin.id)
                        .map(|current| current.session)
                        .unwrap_or(0),
                    current_items: 0,
                    stale_items: 0,
                    unavailable: self.unavailable.get(&snapshot.origin.id).cloned(),
                });
            if snapshot.state == "current" {
                for item in &snapshot.items {
                    current_items += 1;
                    match item.severity_label {
                        "error" => counts.error += 1,
                        "warning" => counts.warning += 1,
                        "information" => counts.information += 1,
                        "hint" => counts.hint += 1,
                        _ => counts.unknown += 1,
                    }
                    summary.current_items += 1;
                }
            } else {
                stale_items += snapshot.items.len();
                summary.stale_items += snapshot.items.len();
            }
        }
        // An origin with no publication still needs a summary line for honest coverage.
        for (origin_id, current) in &self.current {
            summaries
                .entry(origin_id.clone())
                .or_insert_with(|| OriginSummary {
                    origin: Origin {
                        id: origin_id.clone(),
                        label: current.label.clone(),
                    },
                    session: current.session,
                    current_items: 0,
                    stale_items: 0,
                    unavailable: self.unavailable.get(origin_id).cloned(),
                });
        }

        let awaiting = documents
            .iter()
            .filter(|document| document.path.ends_with(".cs"))
            .filter(|document| {
                let origin_current = self.current.get(LSP_ORIGIN);
                let Some(current) = origin_current else {
                    return false;
                };
                let covered = self.snapshots.values().any(|snapshot| {
                    snapshot.origin.id == LSP_ORIGIN
                        && snapshot.session == current.session
                        && snapshot.uri == document.uri
                        && snapshot
                            .client_revision
                            .is_some_and(|revision| revision >= document.revision)
                });
                !covered
            })
            .map(|document| AwaitingDocument {
                origin: Origin::lsp(),
                session: session,
                path: document.path.clone(),
                revision: document.revision,
            })
            .collect();

        DiagnosticsView {
            session,
            origin: lsp,
            snapshots,
            awaiting,
            counts,
            summaries: summaries.into_values().collect(),
            current_items,
            stale_items,
            coverage,
            coverage_note,
            analyzer_coverage: "not_verified",
            analysis_unavailable,
        }
    }
}

fn update_state(
    snapshot: &mut DocumentSnapshot,
    current: &BTreeMap<String, CurrentOrigin>,
    unavailable: &BTreeMap<String, String>,
    documents: &[DocumentRevision],
) {
    snapshot.current_revision = current
        .get(&snapshot.origin.id)
        .filter(|origin| origin.session == snapshot.session)
        .and_then(|_| {
            if snapshot.origin.id != LSP_ORIGIN {
                // Another origin (for example Build) has its own run/revision identity; client
                // document revisions belong to the LSP session only.
                return None;
            }
            documents
                .iter()
                .find(|document| document.uri == snapshot.uri)
                .map(|document| document.revision)
        });
    let (state, reason) = if let Some(reason) = unavailable.get(&snapshot.origin.id) {
        ("stale", Some(reason.clone()))
    } else if let Some(origin) = current.get(&snapshot.origin.id) {
        if origin.session != snapshot.session {
            (
                "stale",
                Some(format!(
                    "From {} session {}; the current analysis session is {}. This publication is not current.",
                    snapshot.origin.label, snapshot.session, origin.session
                )),
            )
        } else {
            match (snapshot.client_revision, snapshot.current_revision) {
                (Some(covered), Some(current_revision)) if covered < current_revision => (
                    "awaiting",
                    Some(format!(
                        "{} was edited after this publication (revision {} → {}); awaiting diagnostics.",
                        snapshot.path.as_deref().unwrap_or(&snapshot.uri),
                        covered,
                        current_revision
                    )),
                ),
                (None, Some(current_revision)) => (
                    "awaiting",
                    Some(format!(
                        "This publication has no document revision; awaiting diagnostics for revision {}.",
                        current_revision
                    )),
                ),
                _ => ("current", None),
            }
        }
    } else {
        (
            "stale",
            Some(format!(
                "No current {} session; this publication is not current.",
                snapshot.origin.label
            )),
        )
    };
    snapshot.state = state;
    snapshot.stale_reason = reason;
    for item in &mut snapshot.items {
        item.stale = state != "current";
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::{DiagnosticItem, Publication};
    use serde_json::json;

    fn item(code: &str, source: Option<&str>, range: Value) -> DiagnosticItem {
        DiagnosticItem {
            range,
            severity: Some(1),
            code: json!(code),
            source: source.map(str::to_string),
            message: format!("message for {code}"),
            code_description: None,
        }
    }

    fn publication(
        session: u64,
        uri: &str,
        revision: i64,
        items: Vec<DiagnosticItem>,
    ) -> Publication {
        Publication {
            uri: uri.to_string(),
            path: Some(uri.trim_start_matches("file://").to_string()),
            received_at: 100,
            client_revision: Some(revision),
            server_version: Value::Null,
            session,
            workspace_id: 1,
            context_id: 1,
            items,
        }
    }

    fn range(line: u64, character: u64) -> Value {
        json!({"start":{"line":line,"character":character},"end":{"line":line,"character":character+1}})
    }

    fn documents(revision: i64) -> Vec<DocumentRevision> {
        vec![DocumentRevision {
            uri: "file:///Program.cs".to_string(),
            path: "src/Program.cs".to_string(),
            revision,
        }]
    }

    fn store_with(session: u64) -> Store {
        let mut store = Store::new();
        store.begin_session(&Origin::lsp(), session, 1, 1);
        store
    }

    #[test]
    fn empty_publication_is_scoped_to_one_document() {
        let mut store = store_with(1);
        assert!(store.ingest(
            &Origin::lsp(),
            &publication(1, "file:///A.cs", 1, Vec::new())
        ));
        let view = store.view(&[], "full", None);
        assert_eq!(view.snapshots.len(), 1);
        assert_eq!(view.snapshots[0].path.as_deref(), Some("/A.cs"));
        assert_eq!(view.snapshots[0].state, "current");
        assert_eq!(view.current_items, 0);
        // An empty publication for A says nothing about B.
        assert!(!store.snapshots.contains_key("lsp:1:file:///B.cs"));
    }

    #[test]
    fn edit_marks_awaiting_and_keeps_previous_items() {
        let mut store = store_with(1);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![item("CS0246", Some("lsp"), range(4, 17))],
            ),
        );
        let view = store.view(&documents(2), "full", None);
        assert_eq!(view.snapshots[0].state, "awaiting");
        assert!(view.snapshots[0]
            .stale_reason
            .as_deref()
            .unwrap()
            .contains("awaiting"));
        assert_eq!(view.snapshots[0].items.len(), 1, "items must be retained");
        assert!(view.snapshots[0].items[0].stale);
        assert_eq!(view.current_items, 0);
        assert_eq!(view.stale_items, 1);
        assert_eq!(view.awaiting.len(), 1);
    }

    #[test]
    fn republication_for_the_current_revision_replaces_the_previous_one() {
        let mut store = store_with(1);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![item("CS0246", Some("lsp"), range(4, 17))],
            ),
        );
        assert_eq!(
            store.view(&documents(2), "full", None).snapshots[0].state,
            "awaiting"
        );
        store.ingest(
            &Origin::lsp(),
            &publication(1, "file:///Program.cs", 2, Vec::new()),
        );
        let view = store.view(&documents(2), "full", None);
        assert_eq!(view.snapshots.len(), 1);
        assert_eq!(view.snapshots[0].state, "current");
        assert!(view.snapshots[0].items.is_empty());
        assert_eq!(view.current_items, 0);
    }

    #[test]
    fn late_publication_from_an_older_session_is_ignored() {
        let mut store = store_with(1);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![item("CS0246", Some("lsp"), range(4, 17))],
            ),
        );
        store.begin_session(&Origin::lsp(), 2, 1, 2);
        assert!(!store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                2,
                vec![item("CS0103", Some("lsp"), range(1, 1))],
            )
        ));
        let view = store.view(&documents(2), "full", None);
        assert_eq!(view.session, 2);
        assert_eq!(view.snapshots[0].session, 1);
        assert_eq!(view.snapshots[0].state, "stale");
        assert_eq!(view.snapshots[0].items[0].code, "CS0246");
        assert_eq!(view.current_items, 0, "the old session is not current");
    }

    #[test]
    fn an_old_context_publication_is_ignored() {
        let mut store = store_with(1);
        let mut late = publication(1, "file:///Program.cs", 1, Vec::new());
        late.context_id = 9;
        assert!(!store.ingest(&Origin::lsp(), &late));
        assert!(store.snapshots.is_empty());
    }

    #[test]
    fn category_is_unknown_for_lsp_source_even_with_a_cs_code() {
        let mut store = store_with(1);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![
                    item("CS0246", Some("lsp"), range(4, 17)),
                    item("CA1822", Some("Roslyn Analyzer"), range(5, 1)),
                    item("CS1002", Some("C# Compiler"), range(6, 1)),
                    item("X1", None, range(7, 1)),
                ],
            ),
        );
        let items = &store.view(&documents(1), "full", None).snapshots[0].items;
        assert_eq!(items[0].category, Category::Unknown);
        assert_eq!(items[1].category, Category::Analyzer);
        assert_eq!(items[2].category, Category::Compiler);
        assert_eq!(items[3].category, Category::Unknown);
    }

    #[test]
    fn severity_comes_from_the_source_values() {
        let mut store = store_with(1);
        let mut first = item("E", Some("lsp"), range(1, 1));
        first.severity = Some(1);
        let mut second = item("W", Some("lsp"), range(2, 1));
        second.severity = Some(2);
        let mut third = item("I", Some("lsp"), range(3, 1));
        third.severity = Some(3);
        let mut fourth = item("H", Some("lsp"), range(4, 1));
        fourth.severity = Some(4);
        let mut fifth = item("U", Some("lsp"), range(5, 1));
        fifth.severity = Some(9);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![first, second, third, fourth, fifth],
            ),
        );
        let view = store.view(&documents(1), "full", None);
        let severities: Vec<&str> = view.snapshots[0]
            .items
            .iter()
            .map(|item| item.severity_label)
            .collect();
        assert_eq!(
            severities,
            ["error", "warning", "information", "hint", "unknown"]
        );
        assert_eq!(view.counts.error, 1);
        assert_eq!(view.counts.warning, 1);
        assert_eq!(view.counts.information, 1);
        assert_eq!(view.counts.hint, 1);
        assert_eq!(view.counts.unknown, 1);
    }

    #[test]
    fn a_diagnostic_without_a_range_has_no_position() {
        let mut store = store_with(1);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![item("NO_RANGE", Some("lsp"), Value::Null)],
            ),
        );
        let item = &store.view(&documents(1), "full", None).snapshots[0].items[0];
        assert!(item.position.is_none());
    }

    #[test]
    fn origins_are_not_merged_and_keep_their_own_counts() {
        let mut store = store_with(1);
        let build = Origin {
            id: "build".to_string(),
            label: "Build · dotnet".to_string(),
        };
        store.begin_session(&build, 7, 1, 1);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![item("CS0246", Some("lsp"), range(4, 17))],
            ),
        );
        let mut build_publication = publication(
            7,
            "file:///Program.cs",
            1,
            vec![item("CS0246", Some("compiler"), range(4, 17))],
        );
        build_publication.client_revision = None;
        store.ingest(&build, &build_publication);
        let view = store.view(&documents(1), "full", None);
        assert_eq!(view.snapshots.len(), 2);
        assert_eq!(view.current_items, 2);
        assert!(view
            .snapshots
            .iter()
            .any(|snapshot| snapshot.origin.label == "LSP · csharp-ls"));
        assert!(view
            .snapshots
            .iter()
            .any(|snapshot| snapshot.origin.label == "Build · dotnet"));
    }

    #[test]
    fn revoke_keeps_items_but_marks_them_stale() {
        let mut store = store_with(1);
        store.ingest(
            &Origin::lsp(),
            &publication(
                1,
                "file:///Program.cs",
                1,
                vec![item("CS0246", Some("lsp"), range(4, 17))],
            ),
        );
        store.mark_unavailable(LSP_ORIGIN, "Trust was revoked; analysis is unavailable.");
        let view = store.view(&documents(1), "full", None);
        assert_eq!(view.snapshots.len(), 1);
        assert_eq!(view.snapshots[0].state, "stale");
        assert!(view.snapshots[0].items[0].stale);
        assert_eq!(view.current_items, 0);
        assert_eq!(
            view.analysis_unavailable.as_deref(),
            Some("Trust was revoked; analysis is unavailable.")
        );
    }

    #[test]
    fn analyzer_coverage_is_never_claimed() {
        let store = store_with(1);
        let view = store.view(&[], "partial", Some("restore incomplete".to_string()));
        assert_eq!(view.analyzer_coverage, "not_verified");
        assert_eq!(view.coverage, "partial");
        assert_eq!(view.coverage_note.as_deref(), Some("restore incomplete"));
    }
}
