use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub token: String,
    pub name: String,
    pub slug: String,
    pub prefix: String,
}

#[derive(Debug, Clone)]
pub struct McpServer;

impl McpServer {
    pub fn dummy_projects() -> Vec<ProjectSummary> {
        vec![ProjectSummary {
            token: "P_0000000000001".to_owned(),
            name: "IssueBoss".to_owned(),
            slug: "issueboss".to_owned(),
            prefix: "IB".to_owned(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::McpServer;

    #[test]
    fn list_projects_returns_dummy_entry() {
        let projects = McpServer::dummy_projects();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].slug, "issueboss");
    }
}
