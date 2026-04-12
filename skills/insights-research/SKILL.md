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
   If the locator returns zero documents, skip steps 4–5 and return:

   > No prior context found for `<topic>` in `.insights/`.

4. **Dispatch `insights-analyzer`** via the Agent tool — prompt includes the 5 file paths and total found count. Wait for result.

5. **Return findings** — present the synthesized findings inline to the user or calling skill.
