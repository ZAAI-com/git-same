# GitHub API Access Strategy

## Overview

Gisa needs to discover all organizations and repositories a user has access to. This document details the API endpoints, authentication methods, and implementation considerations.

**Important distinction**: Gisa uses the `gh` CLI only to **obtain the authentication token**. All GitHub API calls are made directly by Gisa using HTTP requests (not via `gh api`). This provides:
- Full control over pagination and rate limiting
- Parallel API requests for faster discovery
- Custom progress reporting and error handling

## Required API Endpoints

### 1. List User's Organizations

```
GET /user/orgs
```

**Response** (paginated, 30 per page default, max 100):
```json
[
  {
    "login": "my-org",
    "id": 12345,
    "url": "https://api.github.com/orgs/my-org",
    "repos_url": "https://api.github.com/orgs/my-org/repos"
  }
]
```

**Required Scope**: `read:org`

### 2. List Organization Repositories

```
GET /orgs/{org}/repos
```

**Parameters**:
- `type`: `all`, `public`, `private`, `forks`, `sources`, `member`
- `sort`: `created`, `updated`, `pushed`, `full_name`
- `per_page`: up to 100

**Response** (paginated):
```json
[
  {
    "id": 67890,
    "name": "repo-name",
    "full_name": "my-org/repo-name",
    "private": false,
    "clone_url": "https://github.com/my-org/repo-name.git",
    "ssh_url": "git@github.com:my-org/repo-name.git",
    "archived": false,
    "default_branch": "main"
  }
]
```

**Required Scope**: `repo` (for private repos)

### 3. List User's Personal Repositories

```
GET /user/repos
```

**Parameters**:
- `visibility`: `all`, `public`, `private`
- `affiliation`: `owner`, `collaborator`, `organization_member`
- `type`: `all`, `owner`, `public`, `private`, `member`

For personal repos only, use: `affiliation=owner&type=owner`

**Required Scope**: `repo`

## Authentication Methods

### Priority Order

| Priority | Method | How it Works | Pros | Cons |
| --- | --- | --- | --- | --- |
| 1 | GitHub CLI | `gh auth token` | Secure, managed tokens, SSO support | Requires `gh` installed |
| 2 | SSH Keys | Uses existing `~/.ssh` keys | Already configured for most devs | Only for git operations, not API |
| 3 | PAT (env) | `GITHUB_TOKEN` or `GISA_TOKEN` | Simple, CI-friendly | User manages token security |
| 4 | PAT (config) | Stored in `.gisarc` | Persistent | Less secure if committed |

### Recommended: GitHub CLI Integration

```bash
# Check if gh is authenticated
gh auth status

# Get token for API calls
gh auth token
```

**Benefits**:
- Handles OAuth flow and token refresh
- Supports SSO-enabled organizations
- Secure credential storage (OS keychain)
- Users likely already have it configured

### SSH for Clone Operations

SSH keys authenticate git clone/fetch operations, not API calls.

```bash
# Test SSH access
ssh -T git@github.com

# Clone URL format
git@github.com:{org}/{repo}.git
```

### PAT (Personal Access Token) Fallback

Required scopes:
- `repo` — Full access to private repositories
- `read:org` — Read organization membership

```bash
# Environment variable
export GITHUB_TOKEN=ghp_xxxxxxxxxxxx

# Or in .gisarc
auth:
  token: ghp_xxxxxxxxxxxx  # Not recommended for shared configs
```

## Pagination Handling

GitHub API uses Link headers for pagination:

```
Link: <https://api.github.com/user/repos?page=2>; rel="next",
      <https://api.github.com/user/repos?page=5>; rel="last"
```

### Implementation Strategy

```
function fetchAllPages(url):
    results = []
    while url:
        response = GET(url + "?per_page=100")
        results.append(response.body)
        url = parseLinkHeader(response.headers["Link"], "next")
    return flatten(results)
```

## Rate Limiting

| Auth Type | Rate Limit |
| --- | --- |
| Unauthenticated | 60 requests/hour |
| Authenticated | 5,000 requests/hour |
| GitHub App | 15,000 requests/hour |

### Headers to Monitor

```
X-RateLimit-Limit: 5000
X-RateLimit-Remaining: 4990
X-RateLimit-Reset: 1609459200  # Unix timestamp
```

### Handling Rate Limits

1. Check `X-RateLimit-Remaining` before operations
2. If low, warn user and estimate time needed
3. If exhausted, calculate wait time from `X-RateLimit-Reset`
4. Implement exponential backoff for 403 responses

## Discovery Algorithm

```
1. Authenticate (gh → SSH → PAT)

2. Fetch organizations
   orgs = fetchAllPages("/user/orgs")

3. For each org, fetch repos (parallel)
   for org in orgs:
       repos[org] = fetchAllPages("/orgs/{org}/repos?type=all")

4. Fetch personal repos
   personal = fetchAllPages("/user/repos?affiliation=owner")

5. Build unified repo list
   all_repos = []
   for org, repos in repos:
       for repo in repos:
           all_repos.append({
               org: org.login,
               name: repo.name,
               ssh_url: repo.ssh_url,
               https_url: repo.clone_url,
               archived: repo.archived,
               private: repo.private
           })

   for repo in personal:
       all_repos.append({
           org: "personal",
           name: repo.name,
           ...
       })

6. Return all_repos for clone/sync planning
```

## Caching Considerations

For large organizations, consider caching discovery results:

```yaml
# .gisa-cache.json (auto-generated)
{
  "last_discovery": "2024-01-15T10:30:00Z",
  "orgs": ["org-a", "org-b"],
  "repo_count": 234
}
```

- Cache invalidation: 1 hour default, or `--refresh` flag
- Incremental: store `pushed_at` to detect changes
- Skip cache with `--no-cache`

## Error Scenarios

| Error | Cause | Handling |
| --- | --- | --- |
| 401 Unauthorized | Invalid/expired token | Prompt re-auth |
| 403 Forbidden | Rate limit or insufficient scope | Check headers, advise user |
| 404 Not Found | Org/repo deleted or no access | Skip, log warning |
| 422 Unprocessable | Bad parameters | Log, likely a bug |
| 5xx Server Error | GitHub outage | Retry with backoff |

## Token Storage Strategy

**Gisa does not store tokens itself.** It retrieves tokens at runtime from external sources:

| Source | Storage Location | Managed By |
| --- | --- | --- |
| `gh` CLI (recommended) | OS keychain (macOS Keychain, Windows Credential Manager, Linux secret-service) | GitHub CLI |
| Environment variable | Shell session / CI secrets | User / CI system |
| `.gisarc` config | Project directory | User (not recommended) |

**Why this approach:**
- No token management code to maintain in Gisa
- No security liability for storing secrets
- Leverages existing secure storage mechanisms
- Users don't need to generate/paste tokens if they already use `gh`

**Runtime flow:**
```
gisa sync ~/github
    │
    ├─→ Check: `gh auth token` succeeds? → Use returned token
    │
    ├─→ Check: $GITHUB_TOKEN or $GISA_TOKEN set? → Use env var
    │
    └─→ Check: .gisarc has auth.token? → Use config token (warn user)
```

## Security Considerations

1. **Never log tokens** — Mask in debug output
2. **Prefer ****`gh`**** CLI** — It handles secure storage
3. **Warn about ****`.gisarc`**** tokens** — Suggest `.gitignore`
4. **Minimal scopes** — Request only `repo` and `read:org`
5. **Token rotation** — Support for short-lived tokens via `gh`
