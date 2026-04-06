import { apiFetch, buildQueryString } from './client'
import type {
  SearchResponse,
  RecentResponse,
  TorrentDetail,
  Stats,
  QueueResponse,
  DHTStatus,
  TagsResponse,
  SearchFilters,
  SystemStatus,
  ApiKeyResponse,
  ApiKeyGenerateResponse,
  LogsResponse,
  CommentsResponse,
  Comment,
  VoteSummary,
  PendingNukesResponse,
  IdentityStatus,
  EpochInfo,
  BanListResponse,
  UploadResponse,
  MagnetUploadResponse,
  ScraperStatus,
  AnnouncerStatus,
  SyncDashboard,
  TrendingResponse,
  RecentCommentsResponse,
} from './types'

export async function searchTorrents(
  q: string,
  filters: SearchFilters = {},
  sort = 'resolved_at',
  order = 'desc',
  limit = 50,
  offset = 0,
  facets = true
): Promise<SearchResponse> {
  const qs = buildQueryString({
    q,
    ...filters,
    sort,
    order,
    limit,
    offset,
    facets,
  })
  return apiFetch<SearchResponse>(`/search${qs}`)
}

export async function getRecent(limit = 50): Promise<RecentResponse> {
  const qs = buildQueryString({ limit })
  return apiFetch<RecentResponse>(`/recent${qs}`)
}

export async function getTorrent(infoHash: string): Promise<TorrentDetail> {
  return apiFetch<TorrentDetail>(`/torrent/${infoHash}`)
}

export async function getStats(): Promise<Stats> {
  return apiFetch<Stats>('/stats')
}

export async function getQueue(
  limit = 50,
  offset = 0
): Promise<QueueResponse> {
  const qs = buildQueryString({ limit, offset })
  return apiFetch<QueueResponse>(`/queue${qs}`)
}

export async function getDHTStatus(): Promise<DHTStatus> {
  return apiFetch<DHTStatus>('/dht/status')
}

export async function addTags(
  infoHashes: string[],
  tags: string[]
): Promise<TagsResponse> {
  return apiFetch<TagsResponse>('/torrents/tags', {
    method: 'POST',
    body: JSON.stringify({ info_hashes: infoHashes, tags }),
  })
}

export async function removeTags(
  infoHashes: string[],
  tags: string[]
): Promise<TagsResponse> {
  return apiFetch<TagsResponse>('/torrents/tags', {
    method: 'DELETE',
    body: JSON.stringify({ info_hashes: infoHashes, tags }),
  })
}

export async function getSystemStatus(): Promise<SystemStatus> {
  return apiFetch<SystemStatus>('/system/status')
}

export async function getApiKey(): Promise<ApiKeyResponse> {
  return apiFetch<ApiKeyResponse>('/system/apikey')
}

export async function generateApiKey(): Promise<ApiKeyGenerateResponse> {
  return apiFetch<ApiKeyGenerateResponse>('/system/apikey/generate', { method: 'POST' })
}

export async function getRecentLogs(count = 500, category?: string): Promise<LogsResponse> {
  const params: Record<string, unknown> = { count }
  if (category) params.category = category
  const qs = buildQueryString(params)
  return apiFetch<LogsResponse>(`/system/logs/recent${qs}`)
}

export async function getLogCategories(): Promise<string[]> {
  return apiFetch<string[]>('/system/logs/categories')
}

export async function setDebugMode(enabled: boolean): Promise<{ debug_enabled: boolean }> {
  return apiFetch<{ debug_enabled: boolean }>('/system/logs/debug', {
    method: 'POST',
    body: JSON.stringify({ enabled }),
  })
}

export async function getComments(infoHash: string): Promise<CommentsResponse> {
  return apiFetch<CommentsResponse>(`/torrent/${infoHash}/comments`)
}

export async function postComment(
  infoHash: string,
  body: string,
  parentId?: number
): Promise<Comment> {
  return apiFetch<Comment>(`/torrent/${infoHash}/comments`, {
    method: 'POST',
    body: JSON.stringify({ body, parent_id: parentId ?? null }),
  })
}

export async function deleteComment(infoHash: string, commentId: number): Promise<void> {
  await apiFetch(`/torrent/${infoHash}/comments/${commentId}`, { method: 'DELETE' })
}

export async function getVotes(infoHash: string): Promise<VoteSummary> {
  return apiFetch<VoteSummary>(`/torrent/${infoHash}/votes`)
}

export async function castVote(infoHash: string, value: 1 | -1): Promise<VoteSummary> {
  return apiFetch<VoteSummary>(`/torrent/${infoHash}/vote`, {
    method: 'POST',
    body: JSON.stringify({ value }),
  })
}

export async function suggestNuke(infoHash: string, reason: string): Promise<void> {
  await apiFetch(`/torrent/${infoHash}/nuke`, {
    method: 'POST',
    body: JSON.stringify({ reason }),
  })
}

export async function getPendingNukes(limit = 50, offset = 0): Promise<PendingNukesResponse> {
  const qs = buildQueryString({ limit, offset })
  return apiFetch<PendingNukesResponse>(`/nuke/pending${qs}`)
}

export async function reviewNuke(suggestionId: number, outcome: 'nuked' | 'dismissed'): Promise<void> {
  await apiFetch(`/nuke/${suggestionId}/review`, {
    method: 'POST',
    body: JSON.stringify({ outcome }),
  })
}

export async function getIdentityStatus(): Promise<IdentityStatus> {
  return apiFetch<IdentityStatus>('/identity/status')
}

export async function restoreIdentity(recoveryKey: string): Promise<IdentityStatus> {
  return apiFetch<IdentityStatus>('/identity/restore', {
    method: 'POST',
    body: JSON.stringify({ recovery_key: recoveryKey }),
  })
}

export async function acknowledgeIdentity(): Promise<IdentityStatus> {
  return apiFetch<IdentityStatus>('/identity/acknowledge', { method: 'POST' })
}

export async function getEpochInfo(): Promise<EpochInfo> {
  return apiFetch<EpochInfo>('/identity/epoch')
}

export async function getBanList(): Promise<BanListResponse> {
  return apiFetch<BanListResponse>('/identity/bans')
}

export async function uploadTorrents(files: File[]): Promise<UploadResponse> {
  const formData = new FormData()
  for (const file of files) {
    formData.append('files', file)
  }
  const response = await fetch('/api/v1/upload', {
    method: 'POST',
    body: formData,
  })
  if (!response.ok) {
    const body = await response.text().catch(() => '')
    throw new Error(body || `${response.status} ${response.statusText}`)
  }
  return response.json()
}

export async function uploadMagnets(magnets: string[]): Promise<MagnetUploadResponse> {
  return apiFetch<MagnetUploadResponse>('/upload/magnets', {
    method: 'POST',
    body: JSON.stringify({ magnets }),
  })
}

export async function getScraperStatus(): Promise<ScraperStatus> {
  return apiFetch<ScraperStatus>('/scraper/status')
}

export async function getAnnouncerStatus(): Promise<AnnouncerStatus> {
  return apiFetch<AnnouncerStatus>('/announcer/status')
}

export async function getRecentlyAnnounced(limit: number = 50): Promise<{ results: import('./types').AnnouncedResult[] }> {
  return apiFetch(`/announced/recent?limit=${limit}`)
}

export async function getTrending(hours = 12, limit = 10): Promise<TrendingResponse> {
  const qs = buildQueryString({ hours, limit })
  return apiFetch<TrendingResponse>(`/trending${qs}`)
}

export async function getRecentComments(limit = 20): Promise<RecentCommentsResponse> {
  const qs = buildQueryString({ limit })
  return apiFetch<RecentCommentsResponse>(`/comments/recent${qs}`)
}

export async function getSyncDashboard(): Promise<SyncDashboard> {
  return apiFetch<SyncDashboard>('/sync/dashboard')
}

export async function getSyncPreferences(): Promise<{ import_categories: string[], all_categories: string[], sync_comments: boolean }> {
  return apiFetch('/system/sync/preferences')
}

export async function setSyncPreferences(categories: string[], syncComments: boolean): Promise<{ import_categories: string[], all_categories: string[], sync_comments: boolean }> {
  return apiFetch('/system/sync/preferences', {
    method: 'POST',
    body: JSON.stringify({ import_categories: categories, sync_comments: syncComments }),
  })
}

export type {
  SearchResponse,
  RecentResponse,
  TorrentDetail,
  TorrentResult,
  RecentResult,
  Stats,
  QueueResponse,
  QueueItem,
  DHTStatus,
  Facets,
  FacetValue,
  SearchFilters,
  TorrentContent,
  TorrentFile,
  SystemStatus,
  ApiKeyResponse,
  ApiKeyGenerateResponse,
  LogEntry as LogEntryType,
  LogsResponse,
  Comment,
  CommentsResponse,
  VoteSummary,
  NukeSuggestionItem,
  PendingNukesResponse,
  IdentityStatus,
  EpochInfo,
  BanListResponse,
  UploadResultItem,
  UploadResponse,
  MagnetResultItem,
  MagnetUploadResponse,
  ScraperStatus,
  ScraperSourceStatus,
  ScraperSourceCount,
  AnnouncerStatus,
  AnnouncedResult,
  SyncDashboard,
  SyncPeer,
  TrendingTorrent,
  TrendingResponse,
  RecentComment,
  RecentCommentsResponse,
} from './types'
