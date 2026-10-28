//! Minimal stdio LSP client for csharp-ls.
//!
//! One process per session. Frames are read on a dedicated thread (notifications may arrive
//! before the initialize response), stderr is drained into the same bounded log, and every
//! write is a Content-Length frame with UTF-8 byte lengths. Late messages from an older
//! session are dropped because a restart replaces the `Arc<Lsp>` the reader threads hold.

use crate::diagnostics::DocumentRevision;
use crate::logbuf::{self, LogBuffer};
use crate::process::{kill_group, process_alive, Termination};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{ChildStderr, ChildStdout, ChildStdin};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SHUTDOWN_WAIT: Duration = Duration::from_millis(3_000);
const KILL_GRACE: Duration = Duration::from_secs(2);

/// csharp-ls answers completion/definition well within this bound; a paused or wedged server
/// returns `timeout` instead of blocking the host forever.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
pub const COMPLETION_LIMIT: usize = 200;
/// More actions than this are not listed; the response says it was truncated.
pub const CODE_ACTION_LIMIT: usize = 50;
/// A rename preview is rejected (never partially applied) above these bounds.
pub const RENAME_DOCUMENT_LIMIT: usize = 50;
pub const RENAME_EDIT_LIMIT: usize = 2_000;
pub const RENAME_CONTENT_LIMIT: usize = 1_048_576;
/// More workspace symbols than this are not listed; the response says it was truncated.
pub const WORKSPACE_SYMBOL_LIMIT: usize = 500;
/// A single displayed line block is shortened; counts and applied text stay complete.
const RENAME_DISPLAY_CHARS: usize = 2_000;

#[derive(Debug)]
pub enum Frame {
    Message(Value),
    Invalid(String),
}

#[derive(Debug)]
pub enum RequestError {
    Unavailable(String),
    Timeout(Duration),
    Server(String),
}

/// Reads one `Content-Length` frame. The length is bytes, not characters; headers may use
/// CRLF or LF. Unparsable bodies are returned as `Invalid` instead of killing the reader.
pub fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Frame>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = Vec::new();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 {
            return Ok(None);
        }
        let text = String::from_utf8_lossy(&line);
        let text = text.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            if content_length.is_none() {
                continue;
            }
            break;
        }
        if let Some((name, value)) = text.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse::<usize>().ok();
            }
        }
    }
    let Some(length) = content_length else {
        return Ok(Some(Frame::Invalid(
            "frame without a valid Content-Length header".to_string(),
        )));
    };
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    match String::from_utf8(body) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(value) => Ok(Some(Frame::Message(value))),
            Err(error) => Ok(Some(Frame::Invalid(format!(
                "unparsable JSON frame ({error}): {}",
                text.chars().take(200).collect::<String>()
            )))),
        },
        Err(error) => Ok(Some(Frame::Invalid(format!(
            "non-UTF-8 frame ({} bytes): {error}",
            length
        )))),
    }
}

pub fn write_message(writer: &mut impl Write, value: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub completion: bool,
    pub definition: bool,
    pub rename: bool,
    pub prepare_rename: bool,
    pub code_actions: bool,
    pub workspace_symbol: bool,
    pub push_diagnostics: bool,
    pub pull_diagnostics: bool,
    pub semantic_tokens: bool,
    pub synchronization: Option<Value>,
    pub summary: Vec<String>,
}

fn capabilities_of(capabilities: &Value) -> Capabilities {
    let has = |name: &str| capabilities.get(name).is_some_and(|value| !value.is_null());
    let synchronization = capabilities.get("textDocumentSync").cloned();
    let rename_provider = capabilities.get("renameProvider");
    let prepare_rename = rename_provider
        .and_then(|value| value.get("prepareProvider"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut summary = Vec::new();
    if has("completionProvider") {
        summary.push("completion".to_string());
    }
    if has("definitionProvider") {
        summary.push("definition".to_string());
    }
    if has("renameProvider") {
        summary.push(if prepare_rename {
            "rename (prepare)".to_string()
        } else {
            "rename".to_string()
        });
    }
    if has("codeActionProvider") {
        summary.push("code actions".to_string());
    }
    if has("workspaceSymbolProvider") {
        summary.push("workspace symbols".to_string());
    }
    if has("diagnosticProvider") {
        summary.push("diagnostics (pull advertised)".to_string());
    }
    if has("semanticTokensProvider") {
        summary.push("semantic tokens".to_string());
    }
    Capabilities {
        completion: has("completionProvider"),
        definition: has("definitionProvider"),
        rename: has("renameProvider"),
        prepare_rename,
        code_actions: has("codeActionProvider"),
        workspace_symbol: has("workspaceSymbolProvider"),
        push_diagnostics: true,
        pull_diagnostics: has("diagnosticProvider"),
        semantic_tokens: has("semanticTokensProvider"),
        synchronization,
        summary,
    }
}

impl Capabilities {
    /// Whether the server advertised the request this client will send.
    pub fn supports(&self, method: &str) -> bool {
        match method {
            "textDocument/completion" => self.completion,
            "textDocument/definition" => self.definition,
            "textDocument/rename" => self.rename,
            "textDocument/codeAction" => self.code_actions,
            "workspace/symbol" => self.workspace_symbol,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItemView {
    pub label: String,
    pub detail: Option<String>,
    pub kind: Option<String>,
    pub kind_raw: Option<u64>,
    pub insert_text: Option<String>,
    /// Normalized to `{ range, newText }`; `range` is the replacement range the server sent.
    pub text_edit: Option<Value>,
    pub documentation: Option<String>,
    pub sort_text: Option<String>,
}

pub struct CompletionView {
    pub items: Vec<CompletionItemView>,
    pub total: usize,
    pub is_incomplete: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionTarget {
    pub uri: String,
    pub range: Value,
    pub selection_range: Option<Value>,
}

pub struct DefinitionView {
    pub targets: Vec<DefinitionTarget>,
    pub rejected: usize,
}

/// Completion responses may be an array or a `CompletionList`. `None` means the shape was not
/// understood, so the caller reports a failure instead of inventing an empty list.
pub fn parse_completion(result: &Value, limit: usize) -> Option<CompletionView> {
    let (items, is_incomplete) = match result {
        Value::Null => (&[][..], false),
        Value::Array(items) => (items.as_slice(), false),
        Value::Object(map) => (
            map.get("items")?.as_array()?.as_slice(),
            map.get("isIncomplete")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        _ => return None,
    };
    let total = items.len();
    let items = items
        .iter()
        .take(limit)
        .filter_map(completion_item)
        .collect::<Vec<_>>();
    Some(CompletionView {
        items,
        total,
        is_incomplete,
        truncated: total > limit,
    })
}

fn completion_item(item: &Value) -> Option<CompletionItemView> {
    let label = item.get("label")?.as_str()?.to_string();
    let kind_raw = item.get("kind").and_then(Value::as_u64);
    Some(CompletionItemView {
        label,
        detail: item.get("detail").and_then(Value::as_str).map(str::to_string),
        kind: kind_raw.map(completion_kind).map(str::to_string),
        kind_raw,
        insert_text: item
            .get("insertText")
            .and_then(Value::as_str)
            .map(str::to_string),
        text_edit: item.get("textEdit").and_then(text_edit),
        documentation: item.get("documentation").and_then(documentation),
        sort_text: item.get("sortText").and_then(Value::as_str).map(str::to_string),
    })
}

fn text_edit(value: &Value) -> Option<Value> {
    let new_text = value.get("newText")?.as_str()?.to_string();
    let range = value
        .get("range")
        .or_else(|| value.get("replace"))
        .or_else(|| value.get("insert"))?;
    Some(json!({
        "range": {
            "start": position(range.get("start")?)?,
            "end": position(range.get("end")?)?,
        },
        "newText": new_text,
    }))
}

fn position(value: &Value) -> Option<Value> {
    Some(json!({
        "line": value.get("line")?.as_u64()?,
        "character": value.get("character")?.as_u64()?,
    }))
}

fn documentation(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map.get("value").and_then(Value::as_str).map(str::to_string),
        _ => None,
    }
}

fn completion_kind(kind: u64) -> &'static str {
    match kind {
        2 => "method",
        3 => "function",
        4 => "constructor",
        5 => "field",
        6 => "variable",
        7 => "class",
        8 => "interface",
        9 => "module",
        10 => "property",
        11 => "unit",
        12 => "value",
        13 => "enum",
        14 => "keyword",
        15 => "snippet",
        16 => "color",
        17 => "file",
        18 => "reference",
        19 => "folder",
        20 => "enummember",
        21 => "constant",
        22 => "struct",
        23 => "event",
        24 => "operator",
        25 => "type",
        _ => "text",
    }
}

/// The range the client should replace with the chosen item's text. csharp-ls sends one
/// textEdit range for the list; when it sends none, the client inserts at the cursor.
pub fn completion_replace_range(items: &[CompletionItemView]) -> Value {
    items
        .iter()
        .find_map(|item| item.text_edit.as_ref())
        .and_then(|edit| edit.get("range").cloned())
        .unwrap_or(Value::Null)
}

/// Definitions are `null`, a single Location/LocationLink or an array of them. A malformed
/// entry is counted as rejected instead of becoming a destination.
pub fn parse_definition(result: &Value) -> Option<DefinitionView> {
    let values: Vec<&Value> = match result {
        Value::Null => Vec::new(),
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![result],
        _ => return None,
    };
    let mut targets = Vec::new();
    let mut rejected = 0;
    for value in values {
        match definition_target(value) {
            Some(target) => targets.push(target),
            None => rejected += 1,
        }
    }
    Some(DefinitionView { targets, rejected })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionCommand {
    pub title: String,
    pub command: String,
}

/// One action as returned by the server. `edit` stays raw so the host validates it with the
/// same WorkspaceEdit parser used by rename; `command` is recorded but never executed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeActionView {
    pub title: String,
    pub kind: Option<String>,
    pub is_preferred: bool,
    pub disabled: Option<String>,
    pub edit: Option<Value>,
    pub command: Option<CodeActionCommand>,
    pub command_present: bool,
    pub has_data: bool,
}

pub struct CodeActionList {
    pub actions: Vec<CodeActionView>,
    pub total: usize,
    pub rejected: usize,
    pub truncated: bool,
}

/// `textDocument/codeAction` answers with an array (or null). A malformed entry is counted as
/// rejected, never turned into an action; a non-array result is not understood at all.
pub fn parse_code_actions(result: &Value, limit: usize) -> Option<CodeActionList> {
    let items = match result {
        Value::Null => &[][..],
        Value::Array(items) => items.as_slice(),
        _ => return None,
    };
    let total = items.len();
    let mut actions = Vec::new();
    let mut rejected = 0;
    for item in items.iter().take(limit) {
        match code_action(item) {
            Some(action) => actions.push(action),
            None => rejected += 1,
        }
    }
    Some(CodeActionList {
        actions,
        total,
        rejected,
        truncated: total > limit,
    })
}

fn code_action(value: &Value) -> Option<CodeActionView> {
    let title = value.get("title")?.as_str()?.to_string();
    let disabled = match value.get("disabled") {
        Some(Value::Bool(true)) => Some("the server marked this action disabled".to_string()),
        Some(Value::Object(map)) => Some(
            map.get("reason")
                .and_then(Value::as_str)
                .unwrap_or("the server marked this action disabled without a reason")
                .to_string(),
        ),
        _ => None,
    };
    let command_present = value.get("command").is_some_and(|command| !command.is_null());
    let command = value.get("command").and_then(Value::as_object).and_then(|command| {
        Some(CodeActionCommand {
            title: command
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            command: command.get("command")?.as_str()?.to_string(),
        })
    });
    Some(CodeActionView {
        title,
        kind: value.get("kind").and_then(Value::as_str).map(str::to_string),
        is_preferred: value
            .get("isPreferred")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        disabled,
        edit: value
            .get("edit")
            .filter(|edit| !edit.is_null())
            .cloned(),
        command,
        command_present,
        has_data: value.get("data").is_some_and(|data| !data.is_null()),
    })
}

fn definition_target(value: &Value) -> Option<DefinitionTarget> {
    let uri = value
        .get("uri")
        .or_else(|| value.get("targetUri"))
        .and_then(Value::as_str)?
        .to_string();
    let range = value
        .get("range")
        .or_else(|| value.get("targetRange"))?
        .clone();
    let selection_range = value.get("targetSelectionRange").cloned();
    let chosen = selection_range.as_ref().unwrap_or(&range);
    position(chosen.get("start")?)?;
    position(chosen.get("end")?)?;
    Some(DefinitionTarget {
        uri,
        range,
        selection_range,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSymbolItem {
    pub name: String,
    pub kind: u64,
    pub container_name: String,
    pub uri: String,
    pub range: Value,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSymbolList {
    pub items: Vec<WorkspaceSymbolItem>,
    pub total: usize,
    pub truncated: bool,
}

/// Accepts both shapes LSP allows for `workspace/symbol`: `WorkspaceSymbol` with a
/// `location` of uri/range (or target uri/range) and `SymbolInformation`. Entries without a
/// usable name, kind or start position are skipped, never invented.
pub fn parse_workspace_symbols(result: &Value, limit: usize) -> Option<WorkspaceSymbolList> {
    let entries = match result {
        Value::Null => return Some(WorkspaceSymbolList {
            items: Vec::new(),
            total: 0,
            truncated: false,
        }),
        Value::Array(entries) => entries,
        _ => return None,
    };
    let mut items = Vec::new();
    let mut total = 0;
    for entry in entries {
        let Some(name) = entry.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(kind) = entry.get("kind").and_then(Value::as_u64) else {
            continue;
        };
        let Some(location) = entry.get("location") else {
            continue;
        };
        let Some(uri) = location
            .get("uri")
            .or_else(|| location.get("targetUri"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(range) = location
            .get("range")
            .or_else(|| location.get("targetSelectionRange"))
            .or_else(|| location.get("targetRange"))
        else {
            continue;
        };
        let Some(start) = range.get("start") else {
            continue;
        };
        if start.get("line").and_then(Value::as_u64).is_none()
            || start.get("character").and_then(Value::as_u64).is_none()
        {
            continue;
        }
        total += 1;
        if items.len() >= limit {
            continue;
        }
        items.push(WorkspaceSymbolItem {
            name: name.to_string(),
            kind,
            container_name: entry
                .get("containerName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            uri: uri.to_string(),
            range: range.clone(),
        });
    }
    Some(WorkspaceSymbolList {
        total,
        truncated: total > items.len(),
        items,
    })
}

#[derive(Debug, Clone)]
pub struct RenameTextEdit {
    pub start: Position,
    pub end: Position,
    pub new_text: String,
}

#[derive(Debug, Clone)]
pub struct RenameDocumentEdits {
    pub uri: String,
    pub version: Option<i64>,
    pub edits: Vec<RenameTextEdit>,
}

#[derive(Debug, Clone)]
pub struct RenameChange {
    /// 1-based line of the first line covered by the edit.
    pub line: u64,
    pub before: String,
    pub after: String,
    pub display_truncated: bool,
}

pub struct AppliedRename {
    pub content: String,
    pub changes: Vec<RenameChange>,
}

/// A `WorkspaceEdit` may carry `changes` (uri → TextEdit[]) and/or `documentChanges`
/// (TextDocumentEdit[]). Resource operations and malformed entries are refused instead of
/// being interpreted as text edits.
pub fn parse_workspace_edit(result: &Value) -> Result<Vec<RenameDocumentEdits>, String> {
    let mut documents: Vec<RenameDocumentEdits> = Vec::new();
    if result.is_null() {
        return Ok(documents);
    }
    let Some(map) = result.as_object() else {
        return Err("the result is not a WorkspaceEdit object".to_string());
    };
    if let Some(changes) = map.get("changes") {
        let Some(entries) = changes.as_object() else {
            return Err("`changes` is not an object".to_string());
        };
        for (uri, edits) in entries {
            let Some(edits) = edits.as_array() else {
                return Err(format!("`changes` for {uri} is not an array"));
            };
            merge_document_edits(&mut documents, uri, None, edits)?;
        }
    }
    if let Some(changes) = map.get("documentChanges") {
        let Some(entries) = changes.as_array() else {
            return Err("`documentChanges` is not an array".to_string());
        };
        for entry in entries {
            if let Some(kind) = entry.get("kind").and_then(Value::as_str) {
                return Err(format!(
                    "the server requested the file operation `{kind}`, which this client never applies"
                ));
            }
            let uri = entry
                .get("textDocument")
                .and_then(|document| document.get("uri"))
                .and_then(Value::as_str)
                .ok_or_else(|| "a `documentChanges` entry has no textDocument.uri".to_string())?;
            let Some(edits) = entry.get("edits").and_then(Value::as_array) else {
                return Err(format!("`documentChanges` for {uri} has no edits array"));
            };
            let version = entry
                .get("textDocument")
                .and_then(|document| document.get("version"))
                .and_then(Value::as_i64);
            merge_document_edits(&mut documents, uri, version, edits)?;
        }
    }
    if documents.is_empty() && map.is_empty() {
        return Ok(documents);
    }
    if map.get("changes").is_none() && map.get("documentChanges").is_none() {
        return Err("the result has neither `changes` nor `documentChanges`".to_string());
    }
    Ok(documents)
}

fn merge_document_edits(
    documents: &mut Vec<RenameDocumentEdits>,
    uri: &str,
    version: Option<i64>,
    edits: &[Value],
) -> Result<(), String> {
    let mut parsed = Vec::with_capacity(edits.len());
    for edit in edits {
        parsed.push(rename_text_edit(edit).map_err(|error| format!("{uri}: {error}"))?);
    }
    match documents.iter_mut().find(|document| document.uri == uri) {
        Some(document) => {
            if let (Some(existing), Some(incoming)) = (document.version, version) {
                if existing != incoming {
                    return Err(format!("{uri} is listed with two different versions"));
                }
            }
            if document.version.is_none() {
                document.version = version;
            }
            document.edits.extend(parsed);
        }
        None => documents.push(RenameDocumentEdits {
            uri: uri.to_string(),
            version,
            edits: parsed,
        }),
    }
    Ok(())
}

fn rename_text_edit(value: &Value) -> Result<RenameTextEdit, String> {
    let new_text = value
        .get("newText")
        .and_then(Value::as_str)
        .ok_or_else(|| "an edit has no newText".to_string())?
        .to_string();
    let range = value
        .get("range")
        .ok_or_else(|| "an edit has no range".to_string())?;
    let start = parse_position(range.get("start"))
        .ok_or_else(|| "an edit has no valid range.start".to_string())?;
    let end = parse_position(range.get("end"))
        .ok_or_else(|| "an edit has no valid range.end".to_string())?;
    Ok(RenameTextEdit {
        start,
        end,
        new_text,
    })
}

fn parse_position(value: Option<&Value>) -> Option<Position> {
    let value = value?;
    Some(Position {
        line: value.get("line")?.as_u64()?,
        character: value.get("character")?.as_u64()?,
    })
}

/// LSP positions count lines by `\n` and characters in UTF-16 code units, so astral
/// characters occupy two units and a CRLF line keeps its `\r` as part of the line text.
pub fn utf16_offset(content: &str, position: Position) -> Option<usize> {
    let mut offset = 0usize;
    for _ in 0..position.line {
        let next = content[offset..].find('\n')?;
        offset += next + 1;
    }
    let end = content[offset..]
        .find('\n')
        .map(|next| offset + next)
        .unwrap_or(content.len());
    let mut units = 0u64;
    for (index, character) in content[offset..end].char_indices() {
        if units == position.character {
            return Some(offset + index);
        }
        units += character.len_utf16() as u64;
    }
    (units == position.character).then_some(end)
}

/// Applies every edit against the base text, computes the new text and a per-edit line
/// block for the preview. Overlapping or out-of-range edits reject the whole document.
pub fn apply_rename_edits(base: &str, edits: &[RenameTextEdit]) -> Result<AppliedRename, String> {
    let mut resolved = Vec::with_capacity(edits.len());
    for edit in edits {
        let start = utf16_offset(base, edit.start).ok_or_else(|| {
            format!(
                "the edit start {}:{} is outside the document",
                edit.start.line, edit.start.character
            )
        })?;
        let end = utf16_offset(base, edit.end).ok_or_else(|| {
            format!(
                "the edit end {}:{} is outside the document",
                edit.end.line, edit.end.character
            )
        })?;
        if start > end {
            return Err(format!(
                "the edit {}:{}-{}:{} is reversed",
                edit.start.line, edit.start.character, edit.end.line, edit.end.character
            ));
        }
        resolved.push((start, end, edit));
    }
    let mut ordered: Vec<usize> = (0..resolved.len()).collect();
    ordered.sort_by_key(|index| resolved[*index].0);
    for pair in ordered.windows(2) {
        let (previous_start, previous_end, _) = resolved[pair[0]];
        let (start, _, _) = resolved[pair[1]];
        if start <= previous_start || start < previous_end {
            return Err(format!(
                "edits overlap at byte {start} (previous edit ends at {previous_end})"
            ));
        }
    }

    let mut changes: Vec<RenameChange> = Vec::with_capacity(resolved.len());
    for (start, end, edit) in &resolved {
        let line_start = base[..*start]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let line_end = base[*end..]
            .find('\n')
            .map(|index| end + index)
            .unwrap_or(base.len());
        let before = base[line_start..line_end].to_string();
        let after = format!(
            "{}{}{}",
            &base[line_start..*start],
            edit.new_text,
            &base[*end..line_end]
        );
        let line = base[..line_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count() as u64
            + 1;
        let display_truncated = before.chars().count() > RENAME_DISPLAY_CHARS
            || after.chars().count() > RENAME_DISPLAY_CHARS;
        changes.push(RenameChange {
            line,
            before: truncate_chars(&before, RENAME_DISPLAY_CHARS),
            after: truncate_chars(&after, RENAME_DISPLAY_CHARS),
            display_truncated,
        });
    }
    changes.sort_by_key(|change| change.line);

    let mut content = base.to_string();
    let mut descending = resolved.clone();
    descending.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    for (start, end, edit) in descending {
        content.replace_range(start..end, &edit.new_text);
    }
    Ok(AppliedRename { content, changes })
}

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut shortened: String = text.chars().take(limit).collect();
    shortened.push('…');
    shortened
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticItem {
    pub range: Value,
    pub severity: Option<u64>,
    pub code: Value,
    pub source: Option<String>,
    pub message: String,
    pub code_description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub uri: String,
    pub path: Option<String>,
    pub received_at: u64,
    /// Client document revision at reception. The server's publishDiagnostics has no version.
    pub client_revision: Option<i64>,
    pub server_version: Value,
    pub session: u64,
    pub workspace_id: u64,
    pub context_id: u64,
    pub items: Vec<DiagnosticItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspView {
    pub state: &'static str,
    pub session: u64,
    pub log_level: &'static str,
    pub log: String,
    pub log_truncated: bool,
    pub error: Option<String>,
    pub capabilities: Option<Capabilities>,
    pub server_info: Option<Value>,
    pub host_root: String,
    pub host_root_rebuilt: bool,
    pub host_root_forced: bool,
    pub host_root_check: String,
    pub solution: Option<String>,
    pub command: Vec<String>,
    pub cwd: String,
    pub process_id: Option<u64>,
    pub pid: Option<u32>,
    pub exit: Option<i32>,
    pub termination: Option<Termination>,
    pub coverage: &'static str,
    pub coverage_note: Option<String>,
    pub publication_count: usize,
    pub documents: usize,
    pub awaiting_documents: Vec<String>,
    pub diagnostic_items: usize,
    pub output_closed: bool,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub stopped_reason: Option<String>,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
}

struct Document {
    path: String,
    text: String,
    version: i64,
    last_change_at: u64,
}

pub struct OpenDocument {
    pub path: String,
    pub text: String,
    pub version: i64,
}

struct Data {
    session: u64,
    state: &'static str,
    verbose: bool,
    log: LogBuffer,
    error: Option<String>,
    capabilities: Option<Capabilities>,
    server_info: Option<Value>,
    host_root: String,
    host_root_rebuilt: bool,
    host_root_forced: bool,
    host_root_check: String,
    solution: Option<String>,
    command: Vec<String>,
    cwd: String,
    process_id: Option<u64>,
    pid: Option<u32>,
    exit: Option<i32>,
    termination: Option<Termination>,
    coverage: &'static str,
    coverage_note: Option<String>,
    workspace_id: u64,
    context_id: u64,
    documents: BTreeMap<String, Document>,
    publications: BTreeMap<String, Publication>,
    initialized: bool,
    solution_loaded: bool,
    output_closed: bool,
    shutdown_requested: bool,
    stopped_reason: Option<String>,
    sdk_lines: Vec<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    writer: Option<Arc<Mutex<ChildStdin>>>,
    pending: HashMap<i64, Sender<Value>>,
    next_id: i64,
    initialize_id: Option<i64>,
    kill_tx: Option<Sender<()>>,
}

pub struct Lsp {
    data: Mutex<Data>,
}

impl Lsp {
    #[allow(clippy::too_many_arguments)]
    pub fn begin(
        session: u64,
        workspace_id: u64,
        context_id: u64,
        host_root: &str,
        host_root_rebuilt: bool,
        host_root_forced: bool,
        host_root_check: &str,
        solution: Option<String>,
        command: Vec<String>,
        cwd: String,
        verbose: bool,
        coverage: &'static str,
        coverage_note: Option<String>,
    ) -> Arc<Self> {
        let mut log = LogBuffer::default();
        log.push(format!(
            "LSP session {session} · {} · {}",
            if verbose { "log level debug" } else { "log level info" },
            command.join(" ")
        ));
        log.push(format!("cwd {cwd}"));
        log.push(format!("DOTNET_ROOT {host_root}"));
        log.push(host_root_check.to_string());
        Arc::new(Self {
            data: Mutex::new(Data {
                session,
                state: "starting",
                verbose,
                log,
                error: None,
                capabilities: None,
                server_info: None,
                host_root: host_root.to_string(),
                host_root_rebuilt,
                host_root_forced,
                host_root_check: host_root_check.to_string(),
                solution,
                command,
                cwd,
                process_id: None,
                pid: None,
                exit: None,
                termination: None,
                coverage,
                coverage_note,
                workspace_id,
                context_id,
                documents: BTreeMap::new(),
                publications: BTreeMap::new(),
                initialized: false,
                solution_loaded: false,
                output_closed: false,
                shutdown_requested: false,
                stopped_reason: None,
                sdk_lines: Vec::new(),
                started_at: Some(now_ms()),
                finished_at: None,
                writer: None,
                pending: HashMap::new(),
                next_id: 0,
                initialize_id: None,
                kill_tx: None,
            }),
        })
    }

    pub fn attach(
        &self,
        process_id: u64,
        pid: u32,
        stdin: Option<ChildStdin>,
        kill_tx: Sender<()>,
    ) {
        let mut data = self.data.lock().unwrap();
        data.process_id = Some(process_id);
        data.pid = Some(pid);
        data.writer = stdin.map(|stdin| Arc::new(Mutex::new(stdin)));
        data.kill_tx = Some(kill_tx);
    }

    pub fn session(&self) -> u64 {
        self.data.lock().unwrap().session
    }

    pub fn is_active(&self) -> bool {
        let data = self.data.lock().unwrap();
        !matches!(data.state, "unavailable" | "stopped" | "failed")
    }

    /// Whether the initialize response has been accepted, so document notifications are valid.
    pub fn accepts_documents(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.initialized && matches!(data.state, "loading" | "ready" | "partial" | "awaiting_diagnostics")
    }

    pub fn process_id(&self) -> Option<u64> {
        self.data.lock().unwrap().process_id
    }

    pub fn state(&self) -> &'static str {
        self.data.lock().unwrap().state
    }

    pub fn log_line(&self, line: impl Into<String>) {
        self.data.lock().unwrap().log.push(line.into());
    }

    pub fn view(&self) -> LspView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        let awaiting_documents: Vec<String> = data
            .documents
            .iter()
            .filter(|(uri, document)| {
                document.path.ends_with(".cs")
                    && !current_publication(&data.publications, uri, document)
            })
            .map(|(_, document)| document.path.clone())
            .collect();
        let diagnostic_items = data
            .publications
            .values()
            .map(|item| item.items.len())
            .sum();
        LspView {
            state: data.state,
            session: data.session,
            log_level: if data.verbose { "debug" } else { "info" },
            log,
            log_truncated,
            error: data.error.clone(),
            capabilities: data.capabilities.clone(),
            server_info: data.server_info.clone(),
            host_root: data.host_root.clone(),
            host_root_rebuilt: data.host_root_rebuilt,
            host_root_forced: data.host_root_forced,
            host_root_check: data.host_root_check.clone(),
            solution: data.solution.clone(),
            command: data.command.clone(),
            cwd: data.cwd.clone(),
            process_id: data.process_id,
            pid: data.pid,
            exit: data.exit,
            termination: data.termination.clone(),
            coverage: data.coverage,
            coverage_note: data.coverage_note.clone(),
            publication_count: data.publications.len(),
            documents: data.documents.len(),
            awaiting_documents,
            diagnostic_items,
            output_closed: data.output_closed,
            started_at: data.started_at,
            finished_at: data.finished_at,
            stopped_reason: data.stopped_reason.clone(),
            workspace_id: (data.session > 0).then_some(data.workspace_id),
            context_id: (data.session > 0).then_some(data.context_id),
        }
    }

    pub fn publications(&self) -> Vec<Publication> {
        self.data.lock().unwrap().publications.values().cloned().collect()
    }

    /// Open documents with their client revision, so the snapshot store can tell a publication
    /// for an older buffer apart from a current one.
    pub fn document_revisions(&self) -> Vec<DocumentRevision> {
        let data = self.data.lock().unwrap();
        data.documents
            .iter()
            .map(|(uri, document)| DocumentRevision {
                uri: uri.clone(),
                path: document.path.clone(),
                revision: document.version,
            })
            .collect()
    }

    pub fn document_version(&self, uri: &str) -> Option<i64> {
        self.data
            .lock()
            .unwrap()
            .documents
            .get(uri)
            .map(|document| document.version)
    }

    /// The text this session last sent for the document. A rename preview for an open
    /// document must use this text, not the disk, so unsaved buffer edits stay consistent.
    pub fn open_documents(&self) -> Vec<OpenDocument> {
        self.data
            .lock()
            .unwrap()
            .documents
            .iter()
            .map(|(_, document)| OpenDocument {
                path: document.path.clone(),
                text: document.text.clone(),
                version: document.version,
            })
            .collect()
    }

    pub fn supports_prepare_rename(&self) -> bool {
        self.data
            .lock()
            .unwrap()
            .capabilities
            .as_ref()
            .is_some_and(|capabilities| capabilities.prepare_rename)
    }

    /// Sends one request and waits at most `timeout`. The pending sender is keyed by the
    /// protocol id, so a response can only reach the request it belongs to. A stop clears
    /// pending senders, which turns an in-flight wait into `Unavailable` instead of a stale
    /// answer.
    pub fn request(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, RequestError> {
        if !self.accepts_documents() {
            return Err(RequestError::Unavailable(
                "Analysis is not ready to answer requests.".to_string(),
            ));
        }
        let Some((id, receiver)) = self.post_request(method, params, true) else {
            return Err(RequestError::Unavailable(
                "The language server is not accepting requests.".to_string(),
            ));
        };
        match receiver.recv_timeout(timeout) {
            Ok(response) => {
                if let Some(error) = response.get("error") {
                    let message = error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error");
                    let code = error.get("code").cloned().unwrap_or(Value::Null);
                    self.log_line(format!("[client] {method} id {id} failed: {message} ({code})"));
                    Err(RequestError::Server(format!("{message} ({code})")))
                } else {
                    self.log_line(format!("[client] {method} id {id} answered"));
                    Ok(response.get("result").cloned().unwrap_or(Value::Null))
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                self.data.lock().unwrap().pending.remove(&id);
                self.log_line(format!(
                    "[client] {method} id {id} timed out after {} ms",
                    timeout.as_millis()
                ));
                Err(RequestError::Timeout(timeout))
            }
            Err(RecvTimeoutError::Disconnected) => Err(RequestError::Unavailable(
                "Analysis stopped while the request was pending; nothing was applied.".to_string(),
            )),
        }
    }

    /// True only while the request still belongs to the current session and the document has
    /// the same client revision it had when the request was sent.
    pub fn response_is_current(&self, session: u64, uri: &str, version: Option<i64>) -> bool {
        self.accepts_documents()
            && self.session() == session
            && self.document_version(uri) == version
    }

    pub fn unavailable_view() -> LspView {
        LspView {
            state: "unavailable",
            session: 0,
            log_level: "info",
            log: String::new(),
            log_truncated: false,
            error: None,
            capabilities: None,
            server_info: None,
            host_root: String::new(),
            host_root_rebuilt: false,
            host_root_forced: false,
            host_root_check: String::new(),
            solution: None,
            command: Vec::new(),
            cwd: String::new(),
            process_id: None,
            pid: None,
            exit: None,
            termination: None,
            coverage: "partial",
            coverage_note: Some(
                "Analysis has not started. Install csharp-ls, restore the workspace and start analysis explicitly."
                    .to_string(),
            ),
            publication_count: 0,
            documents: 0,
            awaiting_documents: Vec::new(),
            diagnostic_items: 0,
            output_closed: false,
            started_at: None,
            finished_at: None,
            stopped_reason: None,
            workspace_id: None,
            context_id: None,
        }
    }

    pub fn send_initialize(&self, root: &Path, name: &str) {
        let params = json!({
            "processId": std::process::id(),
            "clientInfo": { "name": "OpenRidr", "version": env!("CARGO_PKG_VERSION") },
            "locale": "en",
            "rootPath": root.to_string_lossy(),
            "rootUri": file_uri(root),
            "workspaceFolders": [{
                "uri": file_uri(root),
                "name": name,
            }],
            "capabilities": {
                "workspace": {
                    "workspaceFolders": true,
                    "configuration": true,
                    "didChangeConfiguration": { "dynamicRegistration": false },
                    "workspaceEdit": { "documentChanges": true }
                },
                "textDocument": {
                    "synchronization": { "dynamicRegistration": false, "willSave": false, "didSave": true },
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "versionSupport": true,
                        "codeDescriptionSupport": true,
                        "dataSupport": true
                    },
                    "completion": { "completionItem": { "snippetSupport": false } },
                    "definition": {},
                    "rename": { "prepareSupport": true },
                    "codeAction": {
                        "codeActionLiteralSupport": {
                            "codeActionKind": { "valueSet": ["quickfix", "refactor"] }
                        }
                    }
                },
                "window": { "workDoneProgress": false }
            }
        });
        let (id, _receiver) = match self.post_request("initialize", params, true) {
            Some(value) => value,
            None => {
                self.fail("The initialize request could not be written to the language server.");
                return;
            }
        };
        self.data.lock().unwrap().initialize_id = Some(id);
        self.log_line(format!("[client] initialize id {id}"));
    }

    pub fn open_document(&self, uri: &str, path: &str, text: &str) -> Result<i64, String> {
        {
            let data = self.data.lock().unwrap();
            if let Some(document) = data.documents.get(uri) {
                return Ok(document.version);
            }
        }
        let version = 1;
        {
            let mut data = self.data.lock().unwrap();
            data.documents.insert(
                uri.to_string(),
                Document {
                    path: path.to_string(),
                    text: text.to_string(),
                    version,
                    last_change_at: now_ms(),
                },
            );
        }
        let sent = self.send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id(path),
                    "version": version,
                    "text": text,
                }
            }),
        );
        if !sent {
            return Err("The language server is not accepting notifications.".to_string());
        }
        self.log_line(format!("[client] didOpen {path} version {version}"));
        self.finalize();
        Ok(version)
    }

    pub fn change_document(&self, uri: &str, path: &str, text: &str) -> Result<i64, String> {
        let version = {
            let mut data = self.data.lock().unwrap();
            let Some(document) = data.documents.get_mut(uri) else {
                return Err("The document was not opened in this analysis session.".to_string());
            };
            document.version += 1;
            document.text = text.to_string();
            document.last_change_at = now_ms();
            document.version
        };
        let sent = self.send_notification(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }]
            }),
        );
        if !sent {
            return Err("The language server is not accepting notifications.".to_string());
        }
        self.log_line(format!(
            "[client] didChange {path} version {version} (full text)"
        ));
        self.finalize();
        Ok(version)
    }

    pub fn mark_partial(&self, reason: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        if matches!(data.state, "failed" | "stopped" | "unavailable") {
            return;
        }
        data.coverage = "partial";
        data.coverage_note = Some(reason.into());
        finalize(&mut data);
    }

    pub fn request_stop(&self, reason: &str) {
        let (initialized, writer_present) = {
            let mut data = self.data.lock().unwrap();
            if data.shutdown_requested {
                return;
            }
            data.shutdown_requested = true;
            data.stopped_reason = Some(reason.to_string());
            (data.initialized, data.writer.is_some())
        };
        if initialized && writer_present {
            if let Some((id, receiver)) = self.post_request("shutdown", Value::Null, false) {
                let response = receiver.recv_timeout(SHUTDOWN_WAIT);
                match response {
                    Ok(value) if value.get("error").is_some() => self.log_line(format!(
                        "[client] shutdown id {id} returned an error; exiting anyway"
                    )),
                    Ok(_) => self.log_line(format!("[client] shutdown id {id} answered")),
                    Err(_) => self.log_line(format!(
                        "[client] shutdown id {id} did not answer within {} ms; exiting anyway",
                        SHUTDOWN_WAIT.as_millis()
                    )),
                }
            }
        }
        let _ = self.send_notification("exit", Value::Null);
        {
            let mut data = self.data.lock().unwrap();
            data.writer = None;
            data.state = "stopped";
            data.finished_at = Some(now_ms());
            data.pending.clear();
            data.kill_tx = None;
            data.log
                .push(format!("[client] stopped: {reason}"));
        }
    }

    pub fn observe_exit(&self, exit: Option<i32>, termination: Option<&Termination>) {
        let mut data = self.data.lock().unwrap();
        data.exit = exit;
        data.termination = termination.cloned();
        if data.state == "stopped" {
            return;
        }
        if data.shutdown_requested {
            data.state = "stopped";
            data.finished_at = Some(now_ms());
            return;
        }
        if data.state != "failed" {
            let code = exit
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            data.state = "failed";
            data.error = Some(format!(
                "The language server exited (exit {code}) before OpenRidr stopped it."
            ));
            data.finished_at = Some(now_ms());
            data.log.push(format!(
                "[client] language server exited with exit {code}; no diagnostics wait."
            ));
        }
        data.pending.clear();
    }

    /// Records what the registry observed for a stop/revoke. A process that did not confirm
    /// termination stays visible as a failure instead of being reported as cleaned up.
    pub fn observe_termination(&self, status: &crate::process::ProcessStatus) {
        let mut data = self.data.lock().unwrap();
        data.exit = status.exit;
        data.termination = status.termination.clone();
        if status.state == "exited" {
            if data.state != "failed" {
                data.state = "stopped";
            }
            data.finished_at = Some(now_ms());
            data.log.push(format!(
                "[client] termination observed; exit {}",
                status
                    .exit
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ));
            return;
        }
        data.state = "failed";
        data.error = Some(
            "The language server did not confirm termination; it remains visible in the process panel."
                .to_string(),
        );
        data.finished_at = Some(now_ms());
    }

    /// Records a failure that happened before a process was attached (spawn or host root).
    pub fn fail_now(&self, message: impl Into<String>) {
        let message = message.into();
        let mut data = self.data.lock().unwrap();
        data.state = "failed";
        data.error = Some(message.clone());
        data.finished_at = Some(now_ms());
        data.pending.clear();
        data.log.push(format!("[client] analysis failed: {message}"));
    }

    fn finalize(&self) {
        let mut data = self.data.lock().unwrap();
        finalize(&mut data);
    }

    fn fail(&self, message: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        data.state = "failed";
        data.error = Some(message.into());
        data.finished_at = Some(now_ms());
        data.initialize_id = None;
        data.pending.clear();
        if let Some(kill) = data.kill_tx.take() {
            let _ = kill.send(());
        }
    }

    fn send_notification(&self, method: &str, params: Value) -> bool {
        let value = json!({ "jsonrpc": "2.0", "method": method, "params": params });
        self.post(value, None).is_some()
    }

    fn send_response(&self, id: Value, result: Value) -> bool {
        let value = json!({ "jsonrpc": "2.0", "id": id, "result": result });
        self.post(value, None).is_some()
    }

    fn post_request(
        &self,
        method: &str,
        params: Value,
        pending: bool,
    ) -> Option<(i64, Receiver<Value>)> {
        let id = {
            let mut data = self.data.lock().unwrap();
            data.next_id += 1;
            data.next_id
        };
        let (sender, receiver) = mpsc::channel();
        if pending {
            self.data.lock().unwrap().pending.insert(id, sender);
        }
        let value = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        if self.post(value, Some(id)).is_none() {
            self.data.lock().unwrap().pending.remove(&id);
            return None;
        }
        Some((id, receiver))
    }

    fn post(&self, value: Value, pending_id: Option<i64>) -> Option<()> {
        let writer = self.data.lock().unwrap().writer.clone()?;
        let result = writer
            .lock()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "writer lock poisoned"))
            .and_then(|mut writer| write_message(&mut *writer, &value));
        if let Err(error) = result {
            if let Some(id) = pending_id {
                self.data.lock().unwrap().pending.remove(&id);
            }
            self.log_line(format!("[client] write failed: {error}"));
            return None;
        }
        Some(())
    }

    fn handle_message(&self, value: Value) {
        if let Some(method) = value.get("method").and_then(Value::as_str) {
            let params = value.get("params").cloned().unwrap_or(Value::Null);
            if let Some(id) = value.get("id") {
                let result = if method == "workspace/configuration" {
                    let count = params
                        .get("items")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                    Value::Array(vec![json!({}); count.max(1)])
                } else {
                    Value::Null
                };
                self.log_line(format!(
                    "[server] request {method} · answered {}",
                    if method == "workspace/configuration" {
                        "[{}]"
                    } else {
                        "null"
                    }
                ));
                self.send_response(id.clone(), result);
                return;
            }
            match method {
                "window/logMessage" => {
                    if let Some(message) = params.get("message").and_then(Value::as_str) {
                        self.note_server_log(message, false);
                    }
                }
                "window/showMessage" => {
                    if let Some(message) = params.get("message").and_then(Value::as_str) {
                        self.note_server_log(message, false);
                    }
                }
                "textDocument/publishDiagnostics" => self.publish(params),
                "$/progress" => {}
                _ => {}
            }
            return;
        }
        let Some(id) = value.get("id").and_then(Value::as_i64) else {
            return;
        };
        let is_initialize = self.data.lock().unwrap().initialize_id == Some(id);
        if is_initialize {
            self.data.lock().unwrap().pending.remove(&id);
            if let Some(error) = value.get("error") {
                self.fail_initialize(error, &value);
            } else {
                self.initialize_ok(value.get("result").cloned().unwrap_or(Value::Null));
            }
            return;
        }
        let sender = self.data.lock().unwrap().pending.remove(&id);
        if let Some(sender) = sender {
            let _ = sender.send(value);
        }
    }

    fn initialize_ok(&self, result: Value) {
        {
            let data = self.data.lock().unwrap();
            if data.shutdown_requested || data.state == "stopped" {
                drop(data);
                self.log_line("[client] late initialize response ignored after stop");
                return;
            }
        }
        let capabilities = capabilities_of(&result["capabilities"]);
        let server_info = result.get("serverInfo").cloned();
        let mut summary = capabilities.summary.clone();
        if capabilities.push_diagnostics {
            summary.push("push diagnostics".to_string());
        }
        {
            let mut data = self.data.lock().unwrap();
            data.initialized = true;
            data.capabilities = Some(capabilities.clone());
            data.server_info = server_info.clone();
            data.state = "loading";
            data.log.push(format!(
                "[client] initialize accepted · {}",
                summary.join(" · ")
            ));
            if let Some(info) = &server_info {
                data.log.push(format!(
                    "[client] server {} {}",
                    info.get("name").and_then(Value::as_str).unwrap_or("unknown"),
                    info
                        .get("version")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                ));
            }
            finalize(&mut data);
        }
        self.send_notification("initialized", json!({}));
        // csharp-ls applies its configuration and starts solution loading after this
        // notification; without it the server stays initialized but never loads the solution.
        self.send_notification(
            "workspace/didChangeConfiguration",
            json!({ "settings": { "csharp": { "ls": {} } } }),
        );
    }

    fn fail_initialize(&self, error: &Value, response: &Value) {
        {
            let data = self.data.lock().unwrap();
            if data.shutdown_requested || data.state == "stopped" {
                drop(data);
                self.log_line("[client] late initialize error ignored after stop");
                return;
            }
        }
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown initialize error");
        let code = error.get("code").cloned().unwrap_or(Value::Null);
        let mut data = self.data.lock().unwrap();
        let sdk_excerpt = data
            .sdk_lines
            .iter()
            .rev()
            .take(6)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" ");
        data.state = "failed";
        data.error = Some(if sdk_excerpt.is_empty() {
            format!("The language server rejected initialize: {message} ({code}).")
        } else {
            format!(
                "The language server rejected initialize: {message} ({code}). The server reported: {sdk_excerpt}"
            )
        });
        data.finished_at = Some(now_ms());
        data.log.push(format!(
            "[client] initialize failed: {}",
            serde_json::to_string(response).unwrap_or_default()
        ));
        if let Some(kill) = data.kill_tx.take() {
            let _ = kill.send(());
        }
    }

    fn note_server_log(&self, message: &str, stderr: bool) {
        let mut data = self.data.lock().unwrap();
        let prefix = if stderr { "[stderr]" } else { "[server]" };
        data.log.push(format!("{prefix} {message}"));
        let lower = message.to_ascii_lowercase();
        if message.contains("Finished loading solution") {
            data.solution_loaded = true;
            data.log
                .push("[client] solution loaded; waiting for diagnostics".to_string());
        }
        if lower.contains("compatible .net sdk was not found")
            || lower.contains("requested sdk version")
            || lower.contains("global.json")
        {
            if data.sdk_lines.len() < 8 {
                data.sdk_lines.push(message.trim().to_string());
            }
        }
        if message.contains("Finished loading solution") {
            finalize(&mut data);
        }
    }

    fn publish(&self, params: Value) {
        let Some(uri) = params.get("uri").and_then(Value::as_str) else {
            return;
        };
        {
            let data = self.data.lock().unwrap();
            if data.shutdown_requested || data.state == "stopped" {
                drop(data);
                self.log_line(format!(
                    "[client] late publishDiagnostics dropped after stop · {uri}"
                ));
                return;
            }
        }
        let mut items = Vec::new();
        if let Some(diagnostics) = params.get("diagnostics").and_then(Value::as_array) {
            for diagnostic in diagnostics {
                items.push(DiagnosticItem {
                    range: diagnostic.get("range").cloned().unwrap_or(Value::Null),
                    severity: diagnostic.get("severity").and_then(Value::as_u64),
                    code: diagnostic.get("code").cloned().unwrap_or(Value::Null),
                    source: diagnostic
                        .get("source")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    message: diagnostic
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    code_description: diagnostic
                        .get("codeDescription")
                        .and_then(|value| value.get("href"))
                        .and_then(Value::as_str)
                        .map(str::to_string),
                });
            }
        }
        let mut data = self.data.lock().unwrap();
        let client_revision = data.documents.get(uri).map(|document| document.version);
        let path = data
            .documents
            .get(uri)
            .map(|document| document.path.clone())
            .or_else(|| uri_to_path(uri));
        let publication = Publication {
            uri: uri.to_string(),
            path,
            received_at: now_ms(),
            client_revision,
            server_version: params.get("version").cloned().unwrap_or(Value::Null),
            session: data.session,
            workspace_id: data.workspace_id,
            context_id: data.context_id,
            items,
        };
        data.log.push(format!(
            "[server] publishDiagnostics {} item(s) · {uri} · server version {} · client revision {}",
            publication.items.len(),
            publication.server_version,
            publication
                .client_revision
                .map(|version| version.to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
        if !data.solution_loaded {
            data.solution_loaded = true;
        }
        data.publications.insert(uri.to_string(), publication);
        finalize(&mut data);
    }
}

fn finalize(data: &mut Data) {
    if matches!(data.state, "failed" | "stopped" | "unavailable" | "starting") {
        return;
    }
    if !data.initialized {
        return;
    }
    if !data.solution_loaded {
        data.state = "loading";
        return;
    }
    if data.coverage == "partial" {
        data.state = "partial";
        return;
    }
    let awaiting = data
        .documents
        .iter()
        .filter(|(uri, document)| {
            document.path.ends_with(".cs")
                && !current_publication(&data.publications, uri, document)
        })
        .count();
    data.state = if awaiting > 0 {
        "awaiting_diagnostics"
    } else {
        "ready"
    };
}

fn current_publication(
    publications: &BTreeMap<String, Publication>,
    uri: &str,
    document: &Document,
) -> bool {
    publications
        .get(uri)
        .is_some_and(|publication| publication.received_at >= document.last_change_at)
}

pub fn spawn_reader(lsp: Arc<Lsp>, stdout: ChildStdout) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_frame(&mut reader) {
                Ok(Some(Frame::Message(value))) => lsp.handle_message(value),
                Ok(Some(Frame::Invalid(error))) => {
                    lsp.log_line(format!("[stdout] {error}"));
                }
                Ok(None) => {
                    lsp.data.lock().unwrap().output_closed = true;
                    break;
                }
                Err(error) => {
                    lsp.log_line(format!("[stdout] frame read failed: {error}"));
                    lsp.data.lock().unwrap().output_closed = true;
                    break;
                }
            }
        }
    });
}

pub fn spawn_stderr(lsp: Arc<Lsp>, stderr: ChildStderr) {
    std::thread::spawn(move || {
        logbuf::pump_lines(stderr, true, |line, _| lsp.note_server_log(line, true));
    });
}

/// Kills the server process group after an initialize failure. The registry keeps ownership
/// of the child; this only makes sure a rejected initialize cannot leave it running.
pub fn spawn_killer(pid: u32) -> Sender<()> {
    let (sender, receiver) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        if receiver.recv().is_err() {
            return;
        }
        kill_group(pid, false);
        let deadline = Instant::now() + KILL_GRACE;
        while Instant::now() < deadline && process_alive(pid) {
            std::thread::sleep(Duration::from_millis(50));
        }
        if process_alive(pid) {
            kill_group(pid, true);
        }
    });
    sender
}

pub fn file_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut encoded = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/'
            | b':' => encoded.push(byte as char),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

pub fn uri_to_path(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    let mut bytes = Vec::with_capacity(rest.len());
    let mut chars = rest.bytes().peekable();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let high = chars.next()?;
            let low = chars.next()?;
            let text = format!("{}{}", high as char, low as char);
            bytes.push(u8::from_str_radix(&text, 16).ok()?);
        } else {
            bytes.push(byte);
        }
    }
    let mut path = String::from_utf8(bytes).ok()?;
    if cfg!(windows) && path.starts_with('/') && path.chars().nth(2) == Some(':') {
        path.remove(0);
    }
    Some(path)
}

fn language_id(path: &str) -> &'static str {
    if path.ends_with(".cs") {
        "csharp"
    } else {
        "plaintext"
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

/// A reader that returns at most `chunk` bytes per call, so framing tests exercise split
/// headers and bodies rather than one convenient buffer.
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    struct ChunkReader {
        data: Vec<u8>,
        offset: usize,
        chunk: usize,
    }

    impl Read for ChunkReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let remaining = self.data.len().saturating_sub(self.offset);
            let count = remaining.min(self.chunk).min(buffer.len());
            if count == 0 {
                return Ok(0);
            }
            buffer[..count].copy_from_slice(&self.data[self.offset..self.offset + count]);
            self.offset += count;
            Ok(count)
        }
    }

    fn frame(body: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
    }

    #[test]
    fn reads_a_frame_split_into_tiny_chunks() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let chunked = ChunkReader {
            data: frame(body),
            offset: 0,
            chunk: 1,
        };
        let mut reader = BufReader::new(chunked);
        let value = match read_frame(&mut reader).unwrap().unwrap() {
            Frame::Message(value) => value,
            Frame::Invalid(error) => panic!("invalid frame: {error}"),
        };
        assert_eq!(value["id"], 1);
    }

    #[test]
    fn reads_merged_frames_and_lf_only_headers() {
        let first = r#"{"jsonrpc":"2.0","method":"window/logMessage","params":{"message":"one"}}"#;
        let second = r#"{"jsonrpc":"2.0","method":"window/logMessage","params":{"message":"two"}}"#;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(format!("Content-Length: {}\n\n{}", first.len(), first).as_bytes());
        bytes.extend_from_slice(&frame(second));
        let mut reader = BufReader::new(io::Cursor::new(bytes));
        let one = match read_frame(&mut reader).unwrap().unwrap() {
            Frame::Message(value) => value,
            Frame::Invalid(error) => panic!("{error}"),
        };
        let two = match read_frame(&mut reader).unwrap().unwrap() {
            Frame::Message(value) => value,
            Frame::Invalid(error) => panic!("{error}"),
        };
        assert_eq!(one["params"]["message"], "one");
        assert_eq!(two["params"]["message"], "two");
    }

    #[test]
    fn content_length_counts_bytes_for_multibyte_text() {
        let body = r#"{"jsonrpc":"2.0","method":"window/logMessage","params":{"message":"Klebiškis"}}"#;
        assert!(body.len() > body.chars().count());
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&frame(body));
        let mut reader = BufReader::new(io::Cursor::new(bytes));
        let value = match read_frame(&mut reader).unwrap().unwrap() {
            Frame::Message(value) => value,
            Frame::Invalid(error) => panic!("{error}"),
        };
        assert_eq!(value["params"]["message"], "Klebiškis");
    }

    #[test]
    fn writes_the_byte_length_and_flushes() {
        let mut out = Vec::new();
        write_message(
            &mut out,
            &json!({ "jsonrpc": "2.0", "method": "exit", "params": null }),
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        let (header, body) = text.split_once("\r\n\r\n").unwrap();
        assert_eq!(
            header,
            format!("Content-Length: {}", body.as_bytes().len())
        );
    }

    #[test]
    fn unparsable_json_is_reported_without_stopping_the_stream() {
        let body = "not json";
        let mut reader = BufReader::new(io::Cursor::new(frame(body)));
        match read_frame(&mut reader).unwrap().unwrap() {
            Frame::Invalid(error) => assert!(error.contains("unparsable JSON")),
            Frame::Message(_) => panic!("expected an invalid frame"),
        }
    }

    #[test]
    fn file_uris_percent_encode_spaces_and_non_ascii() {
        let uri = file_uri(Path::new("/tmp/Order Desk/Klebiškis.cs"));
        assert!(uri.starts_with("file:///tmp/Order%20Desk/"));
        assert!(uri.contains("%C5%A1"));
        assert_eq!(
            uri_to_path(&uri).unwrap(),
            "/tmp/Order Desk/Klebiškis.cs"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/a;b&c.cs")),
            "file:///tmp/a%3Bb%26c.cs"
        );
    }

    fn test_lsp(session: u64) -> Arc<Lsp> {
        Lsp::begin(
            session,
            1,
            1,
            "",
            false,
            false,
            "",
            None,
            Vec::new(),
            "/tmp".to_string(),
            false,
            "full",
            None,
        )
    }

    /// A real child with a pipe on stdin and nothing on stdout: requests are written, no
    /// response ever arrives, and the pending map behaves like a live session.
    #[cfg(unix)]
    fn attached_lsp(session: u64) -> (Arc<Lsp>, std::process::Child) {
        use std::process::{Command, Stdio};
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("cat is available");
        let stdin = child.stdin.take().unwrap();
        let (kill_tx, _kill_rx) = mpsc::channel();
        let lsp = test_lsp(session);
        lsp.attach(0, child.id(), Some(stdin), kill_tx);
        (lsp, child)
    }

    #[cfg(unix)]
    #[test]
    fn requests_are_correlated_by_id_even_when_answers_arrive_out_of_order() {
        let (lsp, mut child) = attached_lsp(1);
        lsp.initialize_ok(json!({ "capabilities": {} }));
        let (first_id, first) = lsp
            .post_request("textDocument/completion", json!({}), true)
            .expect("the pipe accepts the request");
        let (second_id, second) = lsp
            .post_request("textDocument/definition", json!({}), true)
            .expect("the pipe accepts the request");
        lsp.handle_message(json!({ "jsonrpc": "2.0", "id": second_id, "result": { "which": "two" } }));
        assert_eq!(second.recv_timeout(Duration::from_millis(200)).unwrap()["result"]["which"], "two");
        assert!(first.try_recv().is_err(), "the second answer reached the first request");
        lsp.handle_message(json!({ "jsonrpc": "2.0", "id": first_id, "result": { "which": "one" } }));
        assert_eq!(first.recv_timeout(Duration::from_millis(200)).unwrap()["result"]["which"], "one");
        drop(lsp);
        let _ = child.wait();
    }

    #[cfg(unix)]
    #[test]
    fn a_request_without_a_response_times_out_and_leaves_no_pending_entry() {
        let (lsp, mut child) = attached_lsp(1);
        lsp.initialize_ok(json!({ "capabilities": {} }));
        let started = Instant::now();
        let error = lsp
            .request(
                "textDocument/completion",
                json!({ "textDocument": { "uri": "file:///tmp/a.cs" } }),
                Duration::from_millis(150),
            )
            .expect_err("nothing answers");
        let elapsed = started.elapsed();
        assert!(matches!(error, RequestError::Timeout(_)), "{error:?}");
        assert!(elapsed >= Duration::from_millis(140), "{elapsed:?}");
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
        assert!(lsp.data.lock().unwrap().pending.is_empty());
        drop(lsp);
        let _ = child.wait();
    }

    #[test]
    fn a_response_is_current_only_for_the_same_session_and_document_version() {
        let lsp = test_lsp(3);
        lsp.initialize_ok(json!({ "capabilities": { "completionProvider": {} } }));
        let uri = "file:///tmp/a.cs";
        assert!(lsp.open_document(uri, "a.cs", "class A {}\n").is_err());
        assert_eq!(lsp.document_version(uri), Some(1));
        assert!(lsp.response_is_current(3, uri, Some(1)));
        assert!(!lsp.response_is_current(4, uri, Some(1)));
        assert!(!lsp.response_is_current(3, uri, None));

        assert!(lsp.change_document(uri, "a.cs", "class A { int B; }\n").is_err());
        assert_eq!(lsp.document_version(uri), Some(2));
        assert!(!lsp.response_is_current(3, uri, Some(1)));
        assert!(lsp.response_is_current(3, uri, Some(2)));
    }

    #[test]
    fn capabilities_absent_or_null_gate_the_matching_request() {
        let none = capabilities_of(&json!({}));
        assert!(!none.completion);
        assert!(!none.definition);
        assert!(!none.supports("textDocument/completion"));
        assert!(!none.supports("textDocument/definition"));

        let only_definition = capabilities_of(&json!({
            "completionProvider": null,
            "definitionProvider": { "workDoneProgress": false }
        }));
        assert!(!only_definition.completion);
        assert!(only_definition.definition);
        assert!(!only_definition.supports("textDocument/completion"));
        assert!(only_definition.supports("textDocument/definition"));
    }

    #[test]
    fn completion_accepts_arrays_and_lists_and_caps_the_returned_items() {
        let array = json!([
            { "label": "Order", "kind": 7, "detail": "class OrderDesk.Core.Order", "insertText": "Order", "textEdit": { "range": { "start": { "line": 3, "character": 0 }, "end": { "line": 3, "character": 5 } }, "newText": "Order" } },
            { "label": "OrderId" }
        ]);
        let parsed = parse_completion(&array, 200).expect("array");
        assert_eq!(parsed.total, 2);
        assert_eq!(parsed.items[0].kind.as_deref(), Some("class"));
        assert_eq!(parsed.items[0].kind_raw, Some(7));
        assert_eq!(
            parsed.items[0].text_edit.as_ref().unwrap()["range"]["end"]["character"],
            5
        );
        assert!(!parsed.truncated);

        let list = json!({
            "isIncomplete": true,
            "items": [
                { "label": "A" }, { "label": "B" }, { "label": "C" }
            ]
        });
        let capped = parse_completion(&list, 2).expect("list");
        assert!(capped.truncated);
        assert!(capped.is_incomplete);
        assert_eq!(capped.total, 3);
        assert_eq!(capped.items.len(), 2);
        assert!(parse_completion(&json!("nope"), 2).is_none());
    }

    #[test]
    fn definition_accepts_null_single_and_array_and_counts_malformed_entries() {
        let range = json!({ "start": { "line": 2, "character": 20 }, "end": { "line": 2, "character": 25 } });
        let single = json!({ "uri": "file:///tmp/Order.cs", "range": range });
        let parsed = parse_definition(&single).expect("single");
        assert_eq!(parsed.targets.len(), 1);
        assert_eq!(parsed.rejected, 0);
        assert_eq!(parsed.targets[0].uri, "file:///tmp/Order.cs");

        let link = json!({
            "targetUri": "file:///tmp/Order.cs",
            "targetRange": range,
            "targetSelectionRange": range
        });
        let parsed = parse_definition(&json!([link, { "uri": "file:///tmp/NoRange.cs" }])).expect("array");
        assert_eq!(parsed.targets.len(), 1);
        assert_eq!(parsed.rejected, 1);
        assert!(parsed.targets[0].selection_range.is_some());

        assert_eq!(parse_definition(&Value::Null).unwrap().targets.len(), 0);
        assert!(parse_definition(&json!("nope")).is_none());
    }
    #[test]
    fn code_action_capability_is_required_and_announced() {
        let absent = capabilities_of(&json!({}));
        assert!(!absent.code_actions);
        assert!(!absent.supports("textDocument/codeAction"));

        let present = capabilities_of(&json!({
            "codeActionProvider": { "resolveProvider": true }
        }));
        assert!(present.code_actions);
        assert!(present.supports("textDocument/codeAction"));
        assert!(present.summary.contains(&"code actions".to_string()));
    }

    #[test]
    fn code_actions_keep_titles_edits_commands_and_disabled_reasons() {
        let parsed = parse_code_actions(
            &json!([
                {
                    "title": "Convert to positional record",
                    "kind": "refactor.rewrite",
                    "isPreferred": true,
                    "edit": { "changes": { "file:///tmp/Order.cs": [] } }
                },
                {
                    "title": "Fix with a command",
                    "command": { "title": "Apply fix", "command": "csharp.applyFix", "arguments": [1] }
                },
                { "title": "Disabled here", "disabled": { "reason": "not available in this context" } },
                { "title": "Deferred action", "data": { "uri": "file:///tmp/Order.cs" } },
                { "noTitle": true }
            ]),
            50,
        )
        .expect("array");
        assert_eq!(parsed.total, 5);
        assert_eq!(parsed.actions.len(), 4);
        assert_eq!(parsed.rejected, 1);
        assert!(!parsed.truncated);
        assert_eq!(parsed.actions[0].kind.as_deref(), Some("refactor.rewrite"));
        assert!(parsed.actions[0].is_preferred);
        assert!(parsed.actions[0].edit.is_some());
        assert_eq!(
            parsed.actions[1]
                .command
                .as_ref()
                .map(|command| command.command.as_str()),
            Some("csharp.applyFix")
        );
        assert!(parsed.actions[1].edit.is_none());
        assert_eq!(
            parsed.actions[2].disabled.as_deref(),
            Some("not available in this context")
        );
        assert!(parsed.actions[3].has_data);
        assert!(parsed.actions[3].edit.is_none() && parsed.actions[3].command.is_none());
        assert_eq!(parse_code_actions(&Value::Null, 50).unwrap().actions.len(), 0);
        assert!(parse_code_actions(&json!({ "not": "an array" }), 50).is_none());
    }

    #[test]
    fn code_actions_are_capped_and_malformed_entries_are_never_actions() {
        let items: Vec<Value> = (0..5)
            .map(|index| json!({ "title": format!("Action {index}") }))
            .collect();
        let parsed = parse_code_actions(&json!(items), 3).expect("array");
        assert_eq!(parsed.actions.len(), 3);
        assert_eq!(parsed.total, 5);
        assert!(parsed.truncated);
    }

    #[test]
    fn rename_capability_includes_prepare_rename_only_when_advertised() {
        let plain = capabilities_of(&json!({ "renameProvider": true }));
        assert!(plain.rename);
        assert!(!plain.prepare_rename);
        assert!(plain.supports("textDocument/rename"));
        assert!(plain.summary.contains(&"rename".to_string()));

        let prepared = capabilities_of(&json!({
            "renameProvider": { "prepareProvider": true }
        }));
        assert!(prepared.rename);
        assert!(prepared.prepare_rename);
        assert!(prepared.summary.contains(&"rename (prepare)".to_string()));

        let absent = capabilities_of(&json!({}));
        assert!(!absent.rename);
        assert!(!absent.supports("textDocument/rename"));
    }

    #[test]
    fn workspace_symbols_parse_both_location_shapes_and_are_capped() {
        let symbols = json!([
            {
                "name": "Order",
                "kind": 5,
                "containerName": "OrderDesk.Domain",
                "location": {
                    "uri": "file:///tmp/Order.cs",
                    "range": { "start": { "line": 4, "character": 6 }, "end": { "line": 4, "character": 11 } }
                }
            },
            {
                "name": "Save",
                "kind": 6,
                "location": {
                    "targetUri": "file:///tmp/Order.cs",
                    "targetSelectionRange": { "start": { "line": 9, "character": 11 }, "end": { "line": 9, "character": 15 } },
                    "targetRange": { "start": { "line": 9, "character": 4 }, "end": { "line": 9, "character": 15 } }
                }
            },
            { "name": "Broken" }
        ]);
        let parsed = parse_workspace_symbols(&symbols, 500).expect("array");
        assert_eq!(parsed.total, 2);
        assert!(!parsed.truncated);
        assert_eq!(parsed.items[0].name, "Order");
        assert_eq!(parsed.items[0].kind, 5);
        assert_eq!(parsed.items[0].container_name, "OrderDesk.Domain");
        assert_eq!(parsed.items[1].container_name, "");
        assert_eq!(parsed.items[1].range["start"]["line"], 9);

        let capped = parse_workspace_symbols(&symbols, 1).expect("array");
        assert_eq!(capped.items.len(), 1);
        assert_eq!(capped.total, 2);
        assert!(capped.truncated);

        assert_eq!(
            parse_workspace_symbols(&Value::Null, 500)
                .expect("null")
                .items
                .len(),
            0
        );
        assert!(parse_workspace_symbols(&json!({ "not": "an array" }), 500).is_none());
    }

    #[test]
    fn workspace_symbol_capability_is_read_from_the_server() {
        let advertised = capabilities_of(&json!({ "workspaceSymbolProvider": true }));
        assert!(advertised.workspace_symbol);
        assert!(advertised.supports("workspace/symbol"));
        assert!(advertised.summary.contains(&"workspace symbols".to_string()));

        let absent = capabilities_of(&json!({}));
        assert!(!absent.workspace_symbol);
        assert!(!absent.supports("workspace/symbol"));
    }

    #[test]
    fn workspace_edits_accept_changes_and_document_changes() {
        let edit = |line: u64, start: u64, end: u64, text: &str| {
            json!({
                "range": {
                    "start": { "line": line, "character": start },
                    "end": { "line": line, "character": end }
                },
                "newText": text
            })
        };
        let parsed = parse_workspace_edit(&json!({
            "changes": {
                "file:///tmp/Order.cs": [edit(4, 11, 16, "Purchase"), edit(9, 12, 17, "Purchase")]
            }
        }))
        .expect("changes");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].edits.len(), 2);
        assert_eq!(
            parsed[0].edits[0].start,
            Position {
                line: 4,
                character: 11
            }
        );
        assert_eq!(parsed[0].version, None);

        let parsed = parse_workspace_edit(&json!({
            "documentChanges": [{
                "textDocument": { "uri": "file:///tmp/Program.cs", "version": 2 },
                "edits": [edit(2, 20, 25, "Purchase")]
            }]
        }))
        .expect("documentChanges");
        assert_eq!(parsed[0].version, Some(2));
        assert_eq!(parsed[0].edits[0].new_text, "Purchase");

        assert_eq!(parse_workspace_edit(&Value::Null).unwrap().len(), 0);
    }

    #[test]
    fn workspace_edits_reject_file_operations_and_malformed_entries() {
        let error = parse_workspace_edit(&json!({
            "documentChanges": [{ "kind": "rename", "oldUri": "file:///tmp/a.cs", "newUri": "file:///tmp/b.cs" }]
        }))
        .expect_err("file operation");
        assert!(error.contains("file operation"), "{error}");

        let error = parse_workspace_edit(&json!({
            "documentChanges": [{
                "textDocument": { "uri": "file:///tmp/a.cs" },
                "edits": [{ "range": { "start": { "line": 0, "character": 0 } }, "newText": "x" }]
            }]
        }))
        .expect_err("missing end");
        assert!(error.contains("range.end"), "{error}");

        assert!(parse_workspace_edit(&json!({ "changes": [] })).is_err());
        assert!(parse_workspace_edit(&json!({ "other": true })).is_err());
    }

    #[test]
    fn utf16_offsets_handle_astral_characters_and_crlf() {
        let text = "first\r\nhéllo 😀x\r\nlast";
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 0,
                    character: 0
                }
            ),
            Some(0)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 0,
                    character: 5
                }
            ),
            Some(5)
        );
        // `\r` still belongs to the line; positions before it map before it.
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 0
                }
            ),
            Some(7)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 5
                }
            ),
            Some(13)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 6
                }
            ),
            Some(14)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 1
                }
            ),
            Some(8)
        );
        // The emoji occupies two UTF-16 units: 8 is `x`, 9 is the end of the line text,
        // and the middle of the surrogate pair (7) is not a valid position.
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 7
                }
            ),
            None
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 8
                }
            ),
            Some(18)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 9
                }
            ),
            Some(19)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 10
                }
            ),
            Some(20)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 1,
                    character: 11
                }
            ),
            None
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 2,
                    character: 4
                }
            ),
            Some(25)
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 3,
                    character: 0
                }
            ),
            None
        );
        assert_eq!(
            utf16_offset(
                text,
                Position {
                    line: 2,
                    character: 5
                }
            ),
            None
        );
    }

    fn text_edit(line: u64, start: u64, end: u64, new_text: &str) -> RenameTextEdit {
        RenameTextEdit {
            start: Position {
                line,
                character: start,
            },
            end: Position {
                line,
                character: end,
            },
            new_text: new_text.to_string(),
        }
    }

    #[test]
    fn rename_edits_apply_against_the_base_in_any_order() {
        let base = "class Order\n{\n    Order() {}\n}\n";
        let applied = apply_rename_edits(
            base,
            &[
                text_edit(2, 4, 9, "Purchase"),
                text_edit(0, 6, 11, "Purchase"),
            ],
        )
        .expect("valid edits");
        assert_eq!(applied.content, "class Purchase\n{\n    Purchase() {}\n}\n");
        assert_eq!(applied.changes.len(), 2);
        assert_eq!(applied.changes[0].line, 1);
        assert_eq!(applied.changes[0].before, "class Order");
        assert_eq!(applied.changes[0].after, "class Purchase");
        assert_eq!(applied.changes[1].line, 3);
    }

    #[test]
    fn rename_edits_reject_overlaps_invalid_and_reversed_ranges() {
        let base = "class Order {}\n";
        assert!(
            apply_rename_edits(base, &[text_edit(0, 6, 11, "A"), text_edit(0, 9, 11, "B")])
                .is_err()
        );
        assert!(
            apply_rename_edits(base, &[text_edit(0, 6, 6, "A"), text_edit(0, 6, 6, "B")]).is_err()
        );
        assert!(apply_rename_edits(base, &[text_edit(0, 11, 6, "A")]).is_err());
        assert!(apply_rename_edits(base, &[text_edit(1, 0, 1, "A")]).is_err());
        assert!(apply_rename_edits(base, &[text_edit(0, 0, 100, "A")]).is_err());
    }
}
