---
name: insights-research
description: Orchestrate insights-locator and insights-analyzer to find and distill prior context from .insights/. Use when a topic may have prior research, at the start of brainstorming/spec/plan phases, or when the user says "check insights", "research insights", "look for prior context", or "what do we know about X".
argument-hint: [topic or question]
---

# Insights Research

You are using the `insights-research` skill to surface prior context from `.insights/`.

## Trigger Phrases

This skill applies when:

- User says "check insights", "research insights", "look for prior context", "what do we know about [topic]"
- At the start of brainstorming, spec writing, or planning work where prior research may exist
- Another skill or agent requests insights context about a topic

## Flow

1. **Get topic** — use the argument provided, or ask: "What topic should I research in `.insights/`?"

2. **Dispatch `insights-locator`** via the Agent tool — prompt: `Find documents in .insights/ related to: <topic>` (include issue slug if known). Wait for result.

3. **Auto-select top 5** from the locator's ranked list. Note the total count for the analyzer.
   Pass all found documents (up to 5) — if fewer than 5 were found, pass all of them.
   If the locator returns zero documents, skip steps 4–7 and return:

   > No prior context found for `<topic>` in `.insights/`.

4. **Dispatch `insights-analyzer`** via the Agent tool — prompt includes the 5 file paths and total found count. Wait for result.

5. **Synthesize** — combine the analyzer's output into a concise summary.

6. **Save research doc** to `.insights/shared/research/`:
   - Kebab-case conversion: lowercase the topic, replace spaces with hyphens, strip punctuation. Example: "Database Migrations" → `database-migrations`
   - Filename: `YYYY-MM-DD-<kebab-topic>.md`
   - Same-day check: look for a file whose name is exactly `YYYY-MM-DD-<kebab-topic>.md` (today's date + same kebab conversion). If found, append a `## Follow-up Research [HH:MM]` section — do not create a new file
   - Use the format below

7. **Run `insights sync`** — run `insights sync` in the terminal.
   - If it fails for any reason: warn the user (see Sync Failure Handling) and continue
   - Do not abort or discard the research doc on sync failure

8. **Return findings** — present the synthesized findings inline to the user or calling skill.

## Research Doc Format

```markdown
---
date: YYYY-MM-DDTHH:MM:SSZ
topic: "<topic>"
tags: [research, <relevant-tags>]
status: complete
---

# Research: <Topic>

**Date:** YYYY-MM-DD
**Topic:** <topic>

## Summary

[High-level answer to the research question — 2-4 sentences]

## Key Findings

### Decisions

- [Decision] — rationale

### Constraints

- [Constraint]

### Technical Specs

- [Spec detail]

### Open Questions

- [Question]

## Document References

- `.insights/path/to/doc.md` — brief description

## Additional Documents (not analyzed)

[N documents were found but not analyzed due to the 5-document cap.
Run insights-research again with a narrower topic or doc type filter to analyze them.]
```

Omit any section that has no content.

## Follow-up Append Format

When appending to an existing same-day file, add at the end:

```markdown
## Follow-up Research [HH:MM]

**Topic:** <topic or follow-up question>

[Key findings in the same Key Findings structure as above]

### Document References

- `.insights/path/to/doc.md` — brief description
```

## Sync Failure Handling

If `insights sync` exits non-zero or is not found, print:

> Warning: `insights sync` failed — the `.insights/searchable/` hardlink tree may be stale.
> The research doc was saved. Run `insights sync` manually to update the searchable tree.

Then continue normally — return the findings to the user.
