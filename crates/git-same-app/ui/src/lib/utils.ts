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
