use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineProject {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub tfms: Vec<String>,
    pub references: Vec<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalJson {
    pub path: String,
    pub version: Option<String>,
    pub roll_forward: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outline {
    pub source: &'static str,
    pub projects: Vec<OutlineProject>,
    pub global_json: Option<GlobalJson>,
}

pub fn build_outline(root: &Path, workspace_file: &Path, is_solution: bool) -> Outline {
    let projects = if is_solution {
        let text = fs::read_to_string(workspace_file).unwrap_or_default();
        let entries = if workspace_file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("slnx"))
        {
            parse_slnx(&text)
        } else {
            parse_solution(&text)
        };
        entries
            .into_iter()
            .map(|(name, rel)| outline_project(root, &rel, &name))
            .collect()
    } else {
        let rel = relative(root, workspace_file);
        let name = workspace_file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.clone());
        vec![outline_project(root, &rel, &name)]
    };

    Outline {
        source: "text",
        projects,
        global_json: read_global_json(root),
    }
}

fn outline_project(root: &Path, rel: &str, name: &str) -> OutlineProject {
    let rel = normalize_rel(rel);
    let unknown = || OutlineProject {
        name: name.to_string(),
        path: rel.clone(),
        kind: "unknown".to_string(),
        tfms: Vec::new(),
        references: Vec::new(),
        files: Vec::new(),
    };

    // Never read project text outside the workspace, even if the .sln names such a path.
    let canonical = match fs::canonicalize(root.join(&rel)) {
        Ok(path) if path.starts_with(root) => path,
        _ => return unknown(),
    };
    let Ok(text) = fs::read_to_string(&canonical) else {
        return unknown();
    };

    OutlineProject {
        name: name.to_string(),
        path: rel,
        kind: project_kind(&text).to_string(),
        tfms: project_tfms(&text),
        references: project_references(&text),
        files: list_cs_files(canonical.parent().unwrap_or(root), root),
    }
}

fn project_kind(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if text.contains("<IsTestProject>true")
        || lower.contains("microsoft.net.test.sdk")
        || lower.contains("xunit")
    {
        "test"
    } else if matches!(
        element(text, "OutputType").as_deref(),
        Some("Exe") | Some("WinExe")
    ) {
        "console"
    } else {
        "library"
    }
}

fn project_tfms(text: &str) -> Vec<String> {
    if let Some(multi) = element(text, "TargetFrameworks") {
        return multi
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
    }
    element(text, "TargetFramework")
        .map(|tfm| vec![tfm])
        .unwrap_or_default()
}

fn project_references(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.contains("<ProjectReference"))
        .filter_map(|line| attribute(line, "Include"))
        .map(|include| {
            let normalized = include.replace('\\', "/");
            normalized
                .rsplit('/')
                .next()
                .unwrap_or(&normalized)
                .trim_end_matches(".csproj")
                .to_string()
        })
        .collect()
}

fn read_global_json(root: &Path) -> Option<GlobalJson> {
    let path = root.join("global.json");
    let canonical = fs::canonicalize(&path).ok()?;
    if !canonical.starts_with(root) {
        return None;
    }
    let text = fs::read_to_string(&canonical).ok()?;
    Some(GlobalJson {
        path: relative(root, &canonical),
        version: json_string(&text, "version"),
        roll_forward: json_string(&text, "rollForward"),
    })
}

pub fn parse_solution(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter(|line| line.trim_start().starts_with("Project("))
        .filter_map(|line| {
            let after_eq = &line[line.find('=')?..];
            let parts = quoted(after_eq);
            if parts.len() < 2 {
                return None;
            }
            let path = &parts[1];
            if !path.to_lowercase().ends_with(".csproj") {
                return None;
            }
            Some((parts[0].clone(), path.clone()))
        })
        .collect()
}

/// `.slnx` lists projects as `<Project Path="..."/>`. A tag may span lines; `<Folder>` is not a project.
fn parse_slnx(text: &str) -> Vec<(String, String)> {
    let mut projects = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<Project") {
        let tag = &rest[start..];
        let Some(end) = tag.find('>') else { break };
        // `<ProjectExtensions>` and similar names are not projects.
        let attributes = &tag["<Project".len()..end];
        if attributes.starts_with(char::is_whitespace) {
            if let Some(path) = attribute(attributes, "Path")
                .filter(|path| path.to_lowercase().ends_with(".csproj"))
            {
                let name = Path::new(&normalize_rel(&path))
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_default();
                projects.push((name, path));
            }
        }
        rest = &tag[end..];
    }
    projects
}

fn quoted(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else { break };
        values.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    values
}

fn attribute(line: &str, name: &str) -> Option<String> {
    ['"', '\''].into_iter().find_map(|quote| {
        let needle = format!("{name}={quote}");
        let start = line.find(&needle)? + needle.len();
        let end = line[start..].find(quote)? + start;
        Some(line[start..end].to_string())
    })
}

fn element(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    let value = text[start..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn json_string(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let key_start = text.find(&needle)?;
    let rest = &text[key_start + needle.len()..];
    let after_colon = &rest[rest.find(':')? + 1..];
    let start = after_colon.find('"')? + 1;
    let end = after_colon[start..].find('"')? + start;
    Some(after_colon[start..end].to_string())
}

fn normalize_rel(rel: &str) -> String {
    rel.replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn list_cs_files(project_dir: &Path, root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk(project_dir, root, &mut files);
    files.sort();
    files
}

fn walk(dir: &Path, root: &Path, files: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(|entry| entry.ok()).map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            if name == "bin" || name == "obj" || name.starts_with('.') {
                continue;
            }
            walk(&path, root, files);
        } else if path.extension().is_some_and(|ext| ext == "cs") {
            files.push(relative(root, &path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slnx_compact_tags_list_projects_in_file_order_and_skip_folders() {
        let text = r#"<Solution>
  <Folder Name="/Project/"><Project Path="Api/Api.csproj" /></Folder>
  <Project Path="Core/Core.csproj"/>
</Solution>"#;
        assert_eq!(
            parse_slnx(text),
            vec![
                ("Api".to_string(), "Api/Api.csproj".to_string()),
                ("Core".to_string(), "Core/Core.csproj".to_string()),
            ]
        );
    }

    #[test]
    fn slnx_multiline_tags_accept_single_and_double_quotes() {
        let text = "<Solution>\n  <Project\n    Path='Web Ui/Web Ui.csproj'\n  />\n  <Project Path=\"Tests\\Tests.csproj\"></Project>\n</Solution>";
        assert_eq!(
            parse_slnx(text),
            vec![
                ("Web Ui".to_string(), "Web Ui/Web Ui.csproj".to_string()),
                ("Tests".to_string(), "Tests\\Tests.csproj".to_string()),
            ]
        );
    }

    #[test]
    fn project_reference_with_single_quoted_include_is_listed() {
        assert_eq!(
            project_references("<ProjectReference Include='../Lib/Lib.csproj' />"),
            vec!["Lib".to_string()]
        );
    }
}
