use serde::{Deserialize, Serialize};
use std::process::Command;

// --- Output structs (sent to frontend) ---

#[derive(Debug, Clone, Serialize)]
pub struct GhCliStatus {
    pub installed: bool,
    pub authenticated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author_login: String,
    pub head_ref_name: String,
    pub state: String,
    pub updated_at: String,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub body: String,
    pub review_decision: Option<String>,
    pub status_check_rollup: Vec<CheckStatus>,
    pub labels: Vec<PrLabel>,
    pub files: Vec<PrFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckStatus {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrLabel {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrFile {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
}

// --- Deserialization structs (from gh CLI JSON output) ---

#[derive(Debug, Deserialize)]
struct GhAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GhCheckContext {
    name: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhStatusCheckRollup {
    contexts: Option<Vec<GhCheckContext>>,
}

#[derive(Debug, Deserialize)]
struct GhLabelNode {
    name: String,
    color: String,
}

#[derive(Debug, Deserialize)]
struct GhLabels {
    nodes: Option<Vec<GhLabelNode>>,
}

#[derive(Debug, Deserialize)]
struct GhFileNode {
    path: String,
    additions: u32,
    deletions: u32,
}

#[derive(Debug, Deserialize)]
struct GhFiles {
    nodes: Option<Vec<GhFileNode>>,
}

#[derive(Debug, Deserialize)]
struct GhPullRequest {
    number: u32,
    title: String,
    author: Option<GhAuthor>,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    state: String,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
    additions: Option<u32>,
    deletions: Option<u32>,
    #[serde(rename = "changedFiles")]
    changed_files: Option<u32>,
    body: Option<String>,
    #[serde(rename = "reviewDecision")]
    review_decision: Option<String>,
    #[serde(rename = "statusCheckRollup")]
    status_check_rollup: Option<GhStatusCheckRollup>,
    labels: Option<GhLabels>,
    files: Option<GhFiles>,
}

impl From<GhPullRequest> for PullRequest {
    fn from(gh: GhPullRequest) -> Self {
        let status_check_rollup = gh
            .status_check_rollup
            .and_then(|s| s.contexts)
            .unwrap_or_default()
            .into_iter()
            .map(|c| CheckStatus {
                name: c.name.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                conclusion: c.conclusion,
            })
            .collect();

        let labels = gh
            .labels
            .and_then(|l| l.nodes)
            .unwrap_or_default()
            .into_iter()
            .map(|l| PrLabel {
                name: l.name,
                color: l.color,
            })
            .collect();

        let files = gh
            .files
            .and_then(|f| f.nodes)
            .unwrap_or_default()
            .into_iter()
            .map(|f| PrFile {
                path: f.path,
                additions: f.additions,
                deletions: f.deletions,
            })
            .collect();

        PullRequest {
            number: gh.number,
            title: gh.title,
            author_login: gh.author.map(|a| a.login).unwrap_or_default(),
            head_ref_name: gh.head_ref_name,
            state: gh.state,
            updated_at: gh.updated_at.unwrap_or_default(),
            additions: gh.additions.unwrap_or(0),
            deletions: gh.deletions.unwrap_or(0),
            changed_files: gh.changed_files.unwrap_or(0),
            body: gh.body.unwrap_or_default(),
            review_decision: gh.review_decision,
            status_check_rollup,
            labels,
            files,
        }
    }
}

/// The JSON fields we request from `gh pr list` and `gh pr view`.
const GH_PR_JSON_FIELDS: &str = "number,title,author,headRefName,state,updatedAt,additions,deletions,changedFiles,body,reviewDecision,statusCheckRollup,labels,files";

/// Check if `gh` CLI is installed and authenticated.
pub fn check_gh_cli() -> GhCliStatus {
    let installed = Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !installed {
        return GhCliStatus {
            installed: false,
            authenticated: false,
        };
    }

    let authenticated = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    GhCliStatus {
        installed,
        authenticated,
    }
}

/// Parse JSON output from `gh pr list --json ...` into a list of pull requests.
pub fn parse_pr_list_json(json_str: &str) -> Result<Vec<PullRequest>, String> {
    let gh_prs: Vec<GhPullRequest> =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse PR JSON: {}", e))?;
    Ok(gh_prs.into_iter().map(PullRequest::from).collect())
}

/// Execute `gh pr list` with JSON output and parse results.
pub fn list_pull_requests(repo_path: String) -> Result<Vec<PullRequest>, String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--json",
            GH_PR_JSON_FIELDS,
            "--limit",
            "50",
        ])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("Failed to execute gh pr list: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr list failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pr_list_json(&stdout)
}

/// Execute `gh pr view` for a single PR and parse the result.
pub fn get_pull_request_detail(
    repo_path: String,
    number: u32,
) -> Result<PullRequest, String> {
    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            &number.to_string(),
            "--json",
            GH_PR_JSON_FIELDS,
        ])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("Failed to execute gh pr view: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr view failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let gh_pr: GhPullRequest =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse PR JSON: {}", e))?;
    Ok(PullRequest::from(gh_pr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_gh_cli_returns_status() {
        // Should not panic regardless of whether gh is installed
        let status = check_gh_cli();
        // If not installed, authenticated must be false
        if !status.installed {
            assert!(!status.authenticated);
        }
    }

    #[test]
    fn test_parse_pr_list_json_valid() {
        let json = r#"[
            {
                "number": 42,
                "title": "Add new feature",
                "author": {"login": "octocat"},
                "headRefName": "feature-branch",
                "state": "OPEN",
                "updatedAt": "2025-01-15T10:30:00Z",
                "additions": 150,
                "deletions": 30,
                "changedFiles": 5,
                "body": "This PR adds a new feature.",
                "reviewDecision": "APPROVED",
                "statusCheckRollup": {
                    "contexts": [
                        {
                            "name": "CI / Build",
                            "status": "COMPLETED",
                            "conclusion": "SUCCESS"
                        },
                        {
                            "name": "CI / Lint",
                            "status": "COMPLETED",
                            "conclusion": "FAILURE"
                        }
                    ]
                },
                "labels": {
                    "nodes": [
                        {"name": "enhancement", "color": "a2eeef"},
                        {"name": "priority:high", "color": "ff0000"}
                    ]
                },
                "files": {
                    "nodes": [
                        {"path": "src/main.rs", "additions": 100, "deletions": 20},
                        {"path": "src/lib.rs", "additions": 50, "deletions": 10}
                    ]
                }
            }
        ]"#;

        let result = parse_pr_list_json(json).unwrap();
        assert_eq!(result.len(), 1);

        let pr = &result[0];
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "Add new feature");
        assert_eq!(pr.author_login, "octocat");
        assert_eq!(pr.head_ref_name, "feature-branch");
        assert_eq!(pr.state, "OPEN");
        assert_eq!(pr.updated_at, "2025-01-15T10:30:00Z");
        assert_eq!(pr.additions, 150);
        assert_eq!(pr.deletions, 30);
        assert_eq!(pr.changed_files, 5);
        assert_eq!(pr.body, "This PR adds a new feature.");
        assert_eq!(pr.review_decision, Some("APPROVED".to_string()));

        // Check status checks
        assert_eq!(pr.status_check_rollup.len(), 2);
        assert_eq!(pr.status_check_rollup[0].name, "CI / Build");
        assert_eq!(pr.status_check_rollup[0].status, "COMPLETED");
        assert_eq!(
            pr.status_check_rollup[0].conclusion,
            Some("SUCCESS".to_string())
        );
        assert_eq!(
            pr.status_check_rollup[1].conclusion,
            Some("FAILURE".to_string())
        );

        // Check labels
        assert_eq!(pr.labels.len(), 2);
        assert_eq!(pr.labels[0].name, "enhancement");
        assert_eq!(pr.labels[0].color, "a2eeef");
        assert_eq!(pr.labels[1].name, "priority:high");

        // Check files
        assert_eq!(pr.files.len(), 2);
        assert_eq!(pr.files[0].path, "src/main.rs");
        assert_eq!(pr.files[0].additions, 100);
        assert_eq!(pr.files[0].deletions, 20);
        assert_eq!(pr.files[1].path, "src/lib.rs");
    }

    #[test]
    fn test_parse_pr_list_json_empty() {
        let result = parse_pr_list_json("[]").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_pr_list_json_invalid() {
        let result = parse_pr_list_json("not valid json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse PR JSON"));
    }

    #[test]
    fn test_parse_pr_list_json_missing_optional_fields() {
        let json = r#"[
            {
                "number": 1,
                "title": "Minimal PR",
                "author": null,
                "headRefName": "fix/bug",
                "state": "OPEN",
                "updatedAt": null,
                "additions": null,
                "deletions": null,
                "changedFiles": null,
                "body": null,
                "reviewDecision": null,
                "statusCheckRollup": null,
                "labels": {"nodes": []},
                "files": null
            }
        ]"#;

        let result = parse_pr_list_json(json).unwrap();
        assert_eq!(result.len(), 1);

        let pr = &result[0];
        assert_eq!(pr.number, 1);
        assert_eq!(pr.title, "Minimal PR");
        assert_eq!(pr.author_login, "");
        assert_eq!(pr.head_ref_name, "fix/bug");
        assert_eq!(pr.updated_at, "");
        assert_eq!(pr.additions, 0);
        assert_eq!(pr.deletions, 0);
        assert_eq!(pr.changed_files, 0);
        assert_eq!(pr.body, "");
        assert!(pr.review_decision.is_none());
        assert!(pr.status_check_rollup.is_empty());
        assert!(pr.labels.is_empty());
        assert!(pr.files.is_empty());
    }
}
