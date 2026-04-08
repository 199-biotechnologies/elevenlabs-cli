---
name: Feature request
about: Propose a new command, flag, or behaviour.
title: 'feat: '
labels: enhancement
---

**What do you want to do, and why can't you do it today?**

**Proposed CLI shape**

```bash
elevenlabs <new-command> [flags]
```

**What should the JSON envelope look like?**

```json
{
  "version": "1",
  "status": "success",
  "data": { }
}
```

**Upstream reference**

<!-- Link to the ElevenLabs API endpoint this maps to, if any. -->

**Scope check**

- [ ] This is a thin wrapper over an ElevenLabs HTTP endpoint.
- [ ] This does not add a daemon, MCP server, REPL, or GUI.
- [ ] This does not add a non-Rust runtime dependency.
