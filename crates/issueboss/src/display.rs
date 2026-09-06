use std::fmt::Write as _;

use ib_api::grpc::admin_proto::{
    ApiKeyEntry, ArtifactResponse, CreateApiKeyResponse, IssueRelationshipsProto, IssueResponse, ProjectMemberResponse, ProjectResponse, RelationshipResponse,
    SuperAdminResponse, UserResponse,
};

// ── Issue ─────────────────────────────────────────────────────────────────────

pub fn format_issue(i: &IssueResponse) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Reference:   {}", i.slug);
    let _ = writeln!(out, "Token:       {}", i.token);
    let _ = writeln!(out, "Title:       {}", i.title);
    let _ = writeln!(out, "Status:      {}", i.status);
    let _ = writeln!(out, "Priority:    {}", i.priority);
    let _ = writeln!(out, "Size:        {}", i.size.as_deref().unwrap_or("-"));
    if !i.description.is_empty() {
        let _ = writeln!(out, "Description: {}", i.description);
    }
    let _ = writeln!(out, "Created:     {}", i.created_at);
    let _ = writeln!(out, "Updated:     {}", i.updated_at);
    out
}

pub fn format_issue_list(issues: &[IssueResponse]) -> String {
    if issues.is_empty() {
        return "No issues.\n".to_owned();
    }
    let mut out = String::new();
    let _ = writeln!(out, "{:<10} {:<12} {:<10} {:<8} TITLE", "REF", "STATUS", "PRIORITY", "SIZE");
    let _ = writeln!(out, "{}", "-".repeat(80));
    for i in issues {
        let _ = writeln!(
            out,
            "{:<10} {:<12} {:<10} {:<8} {}",
            i.slug,
            i.status,
            i.priority,
            i.size.as_deref().unwrap_or("-"),
            i.title,
        );
    }
    out
}

// ── Project
// ───────────────────────────────────────────────────────────────────

pub fn format_project(p: &ProjectResponse) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Token:    {}", p.token);
    let _ = writeln!(out, "Name:     {}", p.name);
    let _ = writeln!(out, "Slug:     {}", p.slug);
    let _ = writeln!(out, "Prefix:   {}", p.prefix);
    if let Some(desc) = &p.description {
        let _ = writeln!(out, "Desc:     {desc}");
    }
    let _ = writeln!(out, "Created:  {}", p.created_at);
    let _ = writeln!(out, "Updated:  {}", p.updated_at);
    out
}

pub fn format_project_list(projects: &[ProjectResponse]) -> String {
    if projects.is_empty() {
        return "No projects.\n".to_owned();
    }
    let mut out = String::new();
    let _ = writeln!(out, "{:<15} {:<25} {:<8} TOKEN", "SLUG", "NAME", "PREFIX");
    let _ = writeln!(out, "{}", "-".repeat(80));
    for p in projects {
        let _ = writeln!(out, "{:<15} {:<25} {:<8} {}", p.slug, p.name, p.prefix, p.token);
    }
    out
}

pub fn format_project_member(m: &ProjectMemberResponse) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Project:  {}", m.project_token);
    let _ = writeln!(out, "User:     {}", m.username);
    let _ = writeln!(out, "Token:    {}", m.user_token);
    let _ = writeln!(out, "Caps:     {}", m.capabilities.join(", "));
    let _ = writeln!(out, "Created:  {}", m.created_at);
    let _ = writeln!(out, "Updated:  {}", m.updated_at);
    out
}

pub fn format_project_member_list(members: &[ProjectMemberResponse]) -> String {
    if members.is_empty() {
        return "No members.\n".to_owned();
    }
    let mut out = String::new();
    let _ = writeln!(out, "{:<20} CAPABILITIES", "USERNAME");
    let _ = writeln!(out, "{}", "-".repeat(50));
    for m in members {
        let _ = writeln!(out, "{:<20} {}", m.username, m.capabilities.join(", "));
    }
    out
}

// ── User ──────────────────────────────────────────────────────────────────────

pub fn format_user(u: &UserResponse) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Username:   {}", u.username);
    let _ = writeln!(out, "Full name:  {}", u.full_name);
    let _ = writeln!(out, "Email:      {}", u.email);
    let _ = writeln!(out, "Token:      {}", u.token);
    let _ = writeln!(out, "Caps:       {}", u.capabilities.join(", "));
    let _ = writeln!(out, "Created:    {}", u.created_at);
    let _ = writeln!(out, "Updated:    {}", u.updated_at);
    out
}

pub fn format_user_list(users: &[UserResponse]) -> String {
    if users.is_empty() {
        return "No users.\n".to_owned();
    }
    let mut out = String::new();
    let _ = writeln!(out, "{:<20} {:<25} {:<30} CAPABILITIES", "USERNAME", "FULL NAME", "EMAIL");
    let _ = writeln!(out, "{}", "-".repeat(90));
    for u in users {
        let _ = writeln!(out, "{:<20} {:<25} {:<30} {}", u.username, u.full_name, u.email, u.capabilities.join(", "));
    }
    out
}

// ── Artifact
// ──────────────────────────────────────────────────────────────────

pub fn format_artifact(a: &ArtifactResponse) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Kind:    {}", a.kind);
    let _ = writeln!(out, "Slug:    {}", a.slug.as_deref().unwrap_or("-"));
    let _ = writeln!(out, "Body:    {}", a.body_json);
    let _ = writeln!(out, "Created: {}", a.created_at);
    let _ = writeln!(out, "Updated: {}", a.updated_at);
    out
}

pub fn format_artifact_list(artifacts: &[ArtifactResponse]) -> String {
    if artifacts.is_empty() {
        return "No artifacts.\n".to_owned();
    }
    let mut out = String::new();
    let _ = writeln!(out, "{:<16} {:<24} BODY", "KIND", "SLUG");
    let _ = writeln!(out, "{}", "-".repeat(80));
    for a in artifacts {
        let _ = writeln!(out, "{:<16} {:<24} {}", a.kind, a.slug.as_deref().unwrap_or("-"), a.body_json);
    }
    out
}

// ── Relationship
// ──────────────────────────────────────────────────────────────

pub fn format_relationship(r: &RelationshipResponse) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "From: {}", r.from_slug);
    let _ = writeln!(out, "To:   {}", r.to_slug);
    let _ = writeln!(out, "Kind: {}", r.kind);
    out
}

pub fn format_relationships(rels: &IssueRelationshipsProto) -> String {
    let mut out = String::new();
    if !rels.depends_on.is_empty() {
        out.push_str("Depends on:\n");
        for r in &rels.depends_on {
            let _ = writeln!(out, "  {} — {}", r.slug, r.title);
        }
    }
    if !rels.blocks.is_empty() {
        out.push_str("Blocks:\n");
        for r in &rels.blocks {
            let _ = writeln!(out, "  {} — {}", r.slug, r.title);
        }
    }
    if !rels.related_to.is_empty() {
        out.push_str("Related to:\n");
        for r in &rels.related_to {
            let _ = writeln!(out, "  {} — {}", r.slug, r.title);
        }
    }
    out
}

// ── Api Key
// ───────────────────────────────────────────────────────────────────

pub fn format_api_key_created(resp: &CreateApiKeyResponse) -> String {
    let mut out = String::new();
    let _ = write!(out, "\nAPI key created for: {}\n", resp.username);
    let _ = writeln!(out, "  Type:    {}", resp.key_type);
    let _ = writeln!(out, "  Prefix:  {}", resp.key_prefix);
    let _ = writeln!(out, "  ID:      {}", resp.api_key_id);
    let _ = writeln!(out, "  API key: {}", resp.api_key);
    out.push_str("\nStore this key securely — it will not be shown again.\n");
    out
}

pub fn format_api_key_list(username: &str, keys: &[ApiKeyEntry]) -> String {
    if keys.is_empty() {
        return format!("No API keys for {username}.\n");
    }
    let mut out = String::new();
    let _ = writeln!(out, "{:<20} {:<12} {:<20} {:<25} LAST USED", "ID", "TYPE", "PREFIX", "NAME");
    let _ = writeln!(out, "{}", "-".repeat(100));
    for k in keys {
        let last = if k.last_used_at.is_empty() { "-".to_owned() } else { k.last_used_at.clone() };
        let _ = writeln!(out, "{:<20} {:<12} {:<20} {:<25} {}", k.api_key_id, k.key_type, k.key_prefix, k.name, last);
    }
    out
}

// ── Super Admin
// ───────────────────────────────────────────────────────────────

pub fn format_super_admin(resp: &SuperAdminResponse) -> String {
    let mut out = String::new();
    out.push_str("\nSuperAdmin user created.\n\n");
    let _ = writeln!(out, "  Username: {}", resp.username);
    let _ = writeln!(out, "  Email:    {}", resp.email);
    let _ = writeln!(out, "  API key:  {}", resp.api_key);
    out.push_str("\nStore this key securely — it will not be shown again.\n");
    out.push_str("Set it as ISSUEBOSS_API_KEY to use the admin CLI.\n");
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_issue(slug: &str, title: &str, description: &str, size: Option<&str>) -> IssueResponse {
        IssueResponse {
            slug: slug.to_owned(),
            token: "tok-abc".to_owned(),
            project_slug: "issueboss".to_owned(),
            number: 1,
            title: title.to_owned(),
            status: "DevInProgress".to_owned(),
            priority: "High".to_owned(),
            size: size.map(str::to_owned),
            description: description.to_owned(),
            created_at: "2024-01-01T00:00:00Z".to_owned(),
            updated_at: "2024-01-02T00:00:00Z".to_owned(),
            relationships: None,
            submitter: "test-user".to_owned(),
            assigned: None,
        }
    }

    fn make_project(name: &str, slug: &str, description: Option<&str>) -> ProjectResponse {
        ProjectResponse {
            token: "proj-tok".to_owned(),
            name: name.to_owned(),
            slug: slug.to_owned(),
            prefix: "IB".to_owned(),
            description: description.map(str::to_owned),
            created_at: "2024-01-01T00:00:00Z".to_owned(),
            updated_at: "2024-01-02T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn format_issue_includes_all_fields() {
        let i = make_issue("IB-1", "My Issue", "A description", Some("Large"));
        let out = format_issue(&i);
        assert!(out.contains("IB-1"), "slug missing");
        assert!(out.contains("tok-abc"), "token missing");
        assert!(out.contains("My Issue"), "title missing");
        assert!(out.contains("DevInProgress"), "status missing");
        assert!(out.contains("High"), "priority missing");
        assert!(out.contains("Large"), "size missing");
        assert!(out.contains("A description"), "description missing");
        assert!(out.contains("2024-01-01T00:00:00Z"), "created_at missing");
        assert!(out.contains("2024-01-02T00:00:00Z"), "updated_at missing");
    }

    #[test]
    fn format_issue_omits_description_when_empty() {
        let i = make_issue("IB-2", "No Desc", "", None);
        let out = format_issue(&i);
        assert!(!out.contains("Description:"), "Description line should be absent");
        assert!(out.contains("Size:        -"), "size should show '-' when None");
    }

    #[test]
    fn format_issue_list_header_contains_column_names() {
        let issues = vec![make_issue("IB-1", "First", "", Some("Small"))];
        let out = format_issue_list(&issues);
        assert!(out.contains("REF"), "REF column missing");
        assert!(out.contains("STATUS"), "STATUS column missing");
        assert!(out.contains("PRIORITY"), "PRIORITY column missing");
        assert!(out.contains("SIZE"), "SIZE column missing");
        assert!(out.contains("TITLE"), "TITLE column missing");
    }

    #[test]
    fn format_project_includes_all_fields() {
        let p = make_project("IssueBoss", "issueboss", Some("Track your issues"));
        let out = format_project(&p);
        assert!(out.contains("IssueBoss"), "name missing");
        assert!(out.contains("issueboss"), "slug missing");
        assert!(out.contains("Track your issues"), "description missing");
        assert!(out.contains("proj-tok"), "token missing");
        assert!(out.contains("IB"), "prefix missing");
    }

    #[test]
    fn format_project_omits_desc_when_none() {
        let p = make_project("NoDesc Project", "nodesc", None);
        let out = format_project(&p);
        assert!(!out.contains("Desc:"), "Desc line should be absent when None");
        assert!(out.contains("NoDesc Project"), "name should still appear");
    }
}
