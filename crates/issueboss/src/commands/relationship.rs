use ib_api::grpc::{
    admin::relationship,
    admin_proto::{IssueRelationshipsProto, RelationshipResponse},
};

// ── Args ─────────────────────────────────────────────────────────────────────

#[derive(Debug, clap::Args)]
pub(crate) struct RelationshipArgs {
    #[clap(subcommand)]
    pub command: RelationshipCommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum RelationshipCommands {
    #[command(about = "Add a relationship between two issues")]
    Add(AddArgs),
    #[command(about = "Remove a relationship between two issues")]
    Remove(RemoveArgs),
    #[command(about = "List relationships for an issue")]
    List(ListArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct AddArgs {
    /// From issue slug (e.g. IB-1)
    #[arg(long)]
    pub from: String,
    /// To issue slug (e.g. IB-2)
    #[arg(long)]
    pub to: String,
    /// Relationship kind (DependsOn, RelatedTo)
    #[arg(long)]
    pub kind: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RemoveArgs {
    /// From issue slug (e.g. IB-1)
    #[arg(long)]
    pub from: String,
    /// To issue slug (e.g. IB-2)
    #[arg(long)]
    pub to: String,
    /// Relationship kind (DependsOn, RelatedTo)
    #[arg(long)]
    pub kind: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ListArgs {
    /// Issue slug (e.g. IB-1)
    #[arg(long)]
    pub issue: String,
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_relationship(host: &str, port: u16, args: RelationshipArgs) -> anyhow::Result<()> {
    match args.command {
        RelationshipCommands::Add(a) => cmd_add(host, port, a).await,
        RelationshipCommands::Remove(a) => cmd_remove(host, port, a).await,
        RelationshipCommands::List(a) => cmd_list(host, port, a).await,
    }
}

// ── Implementations ──────────────────────────────────────────────────────────

async fn cmd_add(host: &str, port: u16, args: AddArgs) -> anyhow::Result<()> {
    let r = relationship::api::add_relationship(host, port, &args.from, &args.to, &args.kind)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_relationship(&r);
    Ok(())
}

async fn cmd_remove(host: &str, port: u16, args: RemoveArgs) -> anyhow::Result<()> {
    relationship::api::remove_relationship(host, port, &args.from, &args.to, &args.kind)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Relationship removed.");
    Ok(())
}

async fn cmd_list(host: &str, port: u16, args: ListArgs) -> anyhow::Result<()> {
    let resp = relationship::api::list_relationships(host, port, &args.issue)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    match resp.relationships {
        None => println!("No relationships."),
        Some(rels) if rels.depends_on.is_empty() && rels.blocks.is_empty() && rels.related_to.is_empty() => {
            println!("No relationships.");
        }
        Some(rels) => print_relationships(&rels),
    }
    Ok(())
}

// ── Output helpers ───────────────────────────────────────────────────────────

fn print_relationship(r: &RelationshipResponse) {
    println!("From: {}", r.from_slug);
    println!("To:   {}", r.to_slug);
    println!("Kind: {}", r.kind);
}

fn print_relationships(rels: &IssueRelationshipsProto) {
    if !rels.depends_on.is_empty() {
        println!("Depends on:");
        for r in &rels.depends_on {
            println!("  {} — {}", r.slug, r.title);
        }
    }
    if !rels.blocks.is_empty() {
        println!("Blocks:");
        for r in &rels.blocks {
            println!("  {} — {}", r.slug, r.title);
        }
    }
    if !rels.related_to.is_empty() {
        println!("Related to:");
        for r in &rels.related_to {
            println!("  {} — {}", r.slug, r.title);
        }
    }
}
