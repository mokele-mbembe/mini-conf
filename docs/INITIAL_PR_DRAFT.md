# 首次开源提交说明草案

## Commit Message

```text
docs: define mini-conf MVP architecture, workflows, and repo scaffolding
```

如果你想更强调差异化定位，也可以用：

```text
docs: define deployment-first config platform MVP and engineering scaffold
```

## PR Title

```text
docs: define deployment-first MVP architecture and engineering scaffold
```

## PR Description

```markdown
## Summary

This PR establishes the planning and engineering baseline for `mini-conf`.

It defines the project as a deployment-first, HTTP-first lightweight online config platform rather than a traditional SDK-heavy config center.

## What changed

- clarified the product positioning and MVP scope
- documented the core domain model around `DeploymentInstance`
- documented admin APIs and open client-facing HTTP APIs
- switched planning from MySQL to PostgreSQL
- defined project-level permissions and auth strategy
- documented Linux / WSL2-first development constraints
- added `justfile`, GitHub Actions, repository config files, and hook scaffolding
- added a lightweight performance smoke scaffold for future benchmark-driven workflows
- recorded post-MVP ideas such as JWT, OAuth 2.0, and template sync with diff-driven batch replace

## MVP model

The current MVP is centered on:

`Project -> Environment -> DeploymentInstance -> multiple ConfigFiles`

This is intentionally optimized for scenarios like:

- edge deployments
- store machines
- device hosts
- multi-process workloads sharing one deployment credential

## Notes

- admin auth is designed to support both Session Cookie and JWT, but MVP will fully implement Session Cookie first
- client access uses long-lived deployment tokens with manual reset / revoke
- repeated publish of identical draft content still creates a new revision
- missing config resolution should fail explicitly, leaving fallback behavior to the client
- dynamic `Scope / labels` remain a future extension, not the MVP primary path

## Follow-ups

- initialize the Rust workspace and frontend workspace in Linux / WSL2
- implement the open client API first
- connect the benchmark scaffold to a real `/api/open/configs/resolve` performance measurement
```

## 发布说明草案

如果你想在首次 release note 或项目说明里提前说明认证策略，可以直接复用这段：

```markdown
This MVP is designed to support both Session Cookie and JWT for admin authentication.
However, the first implementation will fully ship with Session Cookie support first,
while JWT and OAuth 2.0 integration are planned for a later release.
```
