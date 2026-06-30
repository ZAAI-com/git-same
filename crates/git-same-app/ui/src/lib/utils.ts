import type { Badge, FinderRepoStatus } from './types';

export function summarize(items: FinderRepoStatus[]) {
  return items.reduce(
    (acc, repo) => {
      acc.total += 1;
      acc[repo.badge] += 1;
      return acc;
    },
    { total: 0, green: 0, blue: 0, orange: 0, red: 0, gray: 0 } as Record<
      Badge | 'total',
      number
    >,
  );
}

export function badgeLabel(badge: Badge): string {
  return {
    green: 'Synced',
    blue: 'Local config',
    orange: 'Branches',
    red: 'Local work',
    gray: 'Pending',
  }[badge];
}

export function repoName(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path;
}

export function folderName(path: string): string {
  return repoName(path) || path;
}

export function parentPath(path: string): string {
  const parts = path.split('/').filter(Boolean);
  if (parts.length <= 1) return path;
  return `${path.startsWith('/') ? '/' : ''}${parts.slice(0, -1).join('/')}`;
}

export function linesToList(value: string): string[] {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);
}

export function listToLines(value: string[] | null | undefined): string {
  return (value ?? []).join('\n');
}

export function formatCount(value: number, singular: string, plural = `${singular}s`) {
  return `${value} ${value === 1 ? singular : plural}`;
}

export function repoChangeCount(repo: FinderRepoStatus): number {
  return repo.staged_count + repo.unstaged_count + repo.untracked_count;
}

export function isHighRiskRepo(repo: FinderRepoStatus): boolean {
  return repo.badge === 'red' || Boolean(repo.read_error);
}

export function relativeTime(value: string | null | undefined): string {
  if (!value) return 'Never';
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return value;
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ago`;
  return new Intl.DateTimeFormat(undefined, {
    month: 'short',
    day: 'numeric',
  }).format(timestamp);
}
