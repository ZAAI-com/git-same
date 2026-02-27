# GitHub Security Hardening Runbook (ZAAI-com)

This runbook gives direct links plus step-by-step actions and why each action matters for `ZAAI-com/git-same`.

Note: GitHub can return `404` on settings URLs when not logged in with sufficient permissions.

## Goal

1. Prevent secrets from being exposed in logs, artifacts, or workflow configuration.
2. Enforce least privilege for workflow tokens and repository permissions.
3. Add guardrails so insecure workflow changes are blocked automatically.

## Phase 1: Organization Baseline (ZAAI-com)

### 1) Organization Actions policy
Link: https://github.com/orgs/ZAAI-com/settings/actions

Why: Central policy prevents unsafe workflow behavior across all repos.

How:
1. Open the link as org owner/security admin.
2. Restrict allowed actions to GitHub-owned and verified creators, or an allowlist.
3. If available in your plan, enable requiring full-length commit SHA pinning for actions.
4. Set default workflow permissions to read-only where possible.
5. Save settings.

Verify:
1. In another repo test workflow, try adding an unapproved action tag and confirm it is blocked.

### 2) Organization Actions secrets
Link: https://github.com/orgs/ZAAI-com/settings/secrets/actions

Why: Centralized secret policy reduces sprawl and accidental over-sharing.

How:
1. Review existing org secrets and repository access scopes.
2. Remove broad secrets that are not reused.
3. Keep only shared low-risk secrets at org level.
4. For publish tokens, prefer repo environment secrets instead of org-wide scope.

Verify:
1. Confirm only intended repositories have access to each org secret.

### 3) Organization Actions variables
Link: https://github.com/orgs/ZAAI-com/settings/variables/actions

Why: Non-secret constants belong in variables, reducing secret misuse.

How:
1. Move non-sensitive values from secrets to variables.
2. Use clear names such as `HOMEBREW_TAP_REPO` or `RELEASE_REPO`.

Verify:
1. Workflows still resolve variables and no secret is used where a variable is enough.

### 4) Organization rulesets
Link: https://github.com/orgs/ZAAI-com/settings/rules

Why: Rulesets standardize protections so repos cannot drift to weaker settings.

How:
1. Create or update a ruleset for production repositories.
2. Require pull requests for default branch changes.
3. Require status checks before merge.
4. Require code-owner reviews for `.github/workflows/**` changes.
5. Apply ruleset to `ZAAI-com/git-same`.

Verify:
1. Open a test PR touching `.github/workflows/` and confirm approvals/checks are required.

### 5) Organization security analysis
Link: https://github.com/orgs/ZAAI-com/settings/security_analysis

Why: Secret scanning and dependency security catch leaks and known vulnerabilities early.

How:
1. Enable secret scanning for supported repositories.
2. Enable push protection for supported repositories.
3. Enable Dependabot alerts and security updates.

Verify:
1. Security features show enabled for `git-same` in repo security settings.

### 6) Organization audit log
Link: https://github.com/orgs/ZAAI-com/settings/audit-log

Why: Audit trails support incident response and change accountability.

How:
1. Filter by `action:org.update_actions_secret` and repo name `git-same`.
2. Review recent changes to secrets, rules, and actions policy.
3. Export events if you need compliance records.

Verify:
1. Confirm all recent critical setting changes are attributable to expected admins.

## Phase 2: Repository Hardening (ZAAI-com/git-same)

### 1) Repository Actions settings
Link: https://github.com/ZAAI-com/git-same/settings/actions

Why: Repo-level workflow controls reduce blast radius from `GITHUB_TOKEN`.

How:
1. Open the link as repo admin.
2. Set `Workflow permissions` to read repository contents by default.
3. Disable broad write permissions unless a workflow explicitly requires it.
4. Keep approval requirements for external contributors enabled if available.

Verify:
1. Workflows still run.
2. Only publish jobs have explicit write permissions in YAML.

### 2) Repository Actions secrets
Link: https://github.com/ZAAI-com/git-same/settings/secrets/actions

Why: Secret scope should match job scope.

How:
1. Review existing repo secrets.
2. Keep only secrets that cannot be moved to environment scope.
3. Remove stale tokens and rotate long-lived tokens.

Verify:
1. No unused secret names remain.
2. Secret update timestamps are current after rotation.

### 3) Repository Actions variables
Link: https://github.com/ZAAI-com/git-same/settings/variables/actions

Why: Avoid storing non-sensitive constants as secrets.

How:
1. Add repository variables for non-sensitive values used by workflows.
2. Update workflows to reference variables where appropriate.

Verify:
1. Workflows pass with variables.
2. Secret count is reduced.

### 4) Environments (`release`, `homebrew`, `crates`)
Link: https://github.com/ZAAI-com/git-same/settings/environments

Why: Environment secrets and reviewer gates protect high-risk publish operations.

How:
1. Create environment `release`.
2. Add required reviewers for `release`.
3. Create environment `homebrew`.
4. Add `HOMEBREW_TAP_REPO_COMMIT_TOKEN` to `homebrew`.
5. Add required reviewers for `homebrew`.
6. Create environment `crates`.
7. Add `CARGO_REGISTRY_TOKEN` to `crates`.
8. Add required reviewers for `crates`.
9. Update workflows so publish jobs declare `environment: homebrew` or `environment: crates`.

Verify:
1. Trigger S3 and S4 with `workflow_dispatch`.
2. Confirm each run pauses for required reviewer approval before secret access.

### 5) Repository rulesets
Link: https://github.com/ZAAI-com/git-same/settings/rules

Why: Prevent bypass of CI checks and workflow guardrails.

How:
1. Create a `main` branch ruleset.
2. Require pull requests and at least one reviewer.
3. Require status checks from CI workflows before merge.
4. Include checks that enforce workflow safety.
5. Restrict force-push and branch deletion.

Verify:
1. PR to `main` cannot merge without required checks and review.

### 6) Branch protection (legacy, if needed)
Link: https://github.com/ZAAI-com/git-same/settings/branches

Why: Some teams still use branch protection rules instead of rulesets.

How:
1. If rulesets are active, keep this page minimal to avoid conflicting policy.
2. If not using rulesets, configure equivalent protections on `main`.

Verify:
1. Only one protection mechanism is authoritative to avoid confusion.

### 7) Repository security analysis
Link: https://github.com/ZAAI-com/git-same/settings/security_analysis

Why: Repo-level toggle confirms scanning is active where supported.

How:
1. Enable secret scanning and push protection if available.
2. Enable Dependabot alerts and updates.
3. Enable dependency graph if disabled.

Verify:
1. Security tab reports all intended features as enabled.

### 8) Repository access
Link: https://github.com/ZAAI-com/git-same/settings/access

Why: Least-privilege human access is as important as token security.

How:
1. Remove direct admin access where unnecessary.
2. Grant access via teams with defined roles.
3. Limit write/admin to maintainers responsible for releases.

Verify:
1. Access list matches expected team ownership model.

## Phase 3: Workflow Validation

### 1) Run required workflows
Why: Validates that security controls are enforced without breaking delivery.

How:
1. Trigger S1 manually.
2. Trigger S2 via `workflow_dispatch` and confirm all S1 gate jobs pass before artifact build starts.
3. Trigger S3 via `workflow_dispatch` and verify environment approval for Homebrew publish.
4. Trigger S4 via `workflow_dispatch` and verify environment approval for crates publish.

Verify:
1. No secret value appears in logs.
2. No publish job runs without reviewer gate.
3. Required checks block merges when failing.

## Quick Acceptance Checklist

1. Org Actions policy restricted and saved.
2. Org security analysis enabled where supported.
3. Repo environments `release`, `homebrew`, `crates` configured with reviewers.
4. Publish secrets moved to environment scope.
5. Repo ruleset for `main` requires reviews and checks.
6. Workflow runs verified with no secret leakage.
