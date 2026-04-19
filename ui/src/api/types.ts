export interface TorrentResult {
  info_hash: string
  name: string
  size: number
  content_type: string | null
  resolution: string | null
  codec: string | null
  video_source: string | null
  year: number | null
  season: number | null
  episode: number | null
  seed_count: number
  peer_count: number
  discovered_at: string
  resolved_at: string | null
  tags: string[]
  audio_codec: string | null
  hdr: string | null
  platform: string | null
}

export interface RecentResult {
  info_hash: string
  name: string
  size: number
  content_type: string | null
  resolution: string | null
  seed_count: number
  peer_count: number
  resolved_at: string | null
  source: string | null
}

export interface FacetValue {
  value: string
  count: number
}

export interface Facets {
  content_type: FacetValue[]
  resolution: FacetValue[]
  codec: FacetValue[]
  video_source: FacetValue[]
  hdr: FacetValue[]
  year: FacetValue[]
  source: FacetValue[]
  audio_codec: FacetValue[]
  language: FacetValue[]
  modifier: FacetValue[]
  platform: FacetValue[]
  music_format: FacetValue[]
}

export interface SearchResponse {
  results: TorrentResult[]
  total: number
  offset: number
  limit: number
  facets: Facets | null
}

export interface RecentResponse {
  results: RecentResult[]
}

export interface AnnouncedResult {
  info_hash: string
  name: string
  size: number
  content_type: string | null
  resolution: string | null
  seed_count: number
  peer_count: number
  announced_at: string | null
  source: string | null
}

export interface TorrentContent {
  content_type: string | null
  title: string | null
  year: number | null
  season: number | null
  episode: number | null
  group: string | null
  resolution: string | null
  codec: string | null
  video_source: string | null
  modifier: string | null
  hdr: string | null
  audio_codec: string | null
  audio_channels: string | null
  tmdb_id: number | null
  imdb_id: string | null
}

export interface TorrentFile {
  path: string
  size: number
  extension: string | null
}

export interface TorrentDetail {
  info_hash: string
  name: string
  size: number
  source: string | null
  discovered_at: string
  resolved_at: string | null
  announced_at: string | null
  seed_count: number
  peer_count: number
  private: boolean
  observations: number
  nfo: string | null
  trackers: string[] | null
  tags: string[]
  content: TorrentContent | null
  files: TorrentFile[]
  magnet_uri: string | null
  vote_score: number
  comment_count: number
  nuke_count: number
  user_vote: number | null
}

export interface SourceCount {
  source: string
  count: number
}

export interface ContentTypeCount {
  content_type: string
  count: number
}

export interface Stats {
  total: number
  resolved: number
  unresolved: number
  pending: number
  dead: number
  no_peers: number
  private: number
  with_nfo: number
  announced_count: number
  pending_announce_count: number
  by_source: SourceCount[]
  by_content_type: ContentTypeCount[]
}

export interface QueueItem {
  info_hash: string
  source: string | null
  discovered_at: string
  resolve_attempts: number
  observations: number
}

export interface QueueResponse {
  results: QueueItem[]
  total: number
  offset: number
  limit: number
}

export interface DHTStatus {
  dht_running: boolean
  instances: number
  routing_table_nodes: number
  routing_table_good: number
  hash_queue_size: number
  total_hashes: number
  resolved: number
  unresolved: number
  resolve_rate: number
  sync_enabled: boolean
  sync_sequence: number
  sync_peers: number
}

export interface TagsRequest {
  info_hashes: string[]
  tags: string[]
}

export interface TagsResponse {
  affected: number
}

export interface HealthResponse {
  status: string
  version: string
}

export interface SearchFilters {
  content_type?: string
  resolution?: string
  codec?: string
  video_source?: string
  hdr?: string
  year?: string
  source?: string
  audio_codec?: string
  modifier?: string
  platform?: string
  has_subtitles?: string
  music_format?: string
  category?: string
  language?: string
}

export interface SystemStatus {
  version: string
  python_version: string
  is_docker: boolean
  db_backend: string
  db_version: string
  uptime_seconds: number
  data_dir: string
  host: string
  port: number
  workers: string[]
  debug: boolean
  total_hashes: number
  resolved_hashes: number
}

export interface ApiKeyResponse {
  key: string
  has_key: boolean
}

export interface ApiKeyGenerateResponse {
  key: string
}

export interface Comment {
  id: number
  parent_id: number | null
  nickname: string
  body: string
  created_at: string
  edited_at: string | null
  deleted: boolean
  is_own: boolean
  replies: Comment[]
}

export interface CommentsResponse {
  comments: Comment[]
  total: number
}

export interface VoteSummary {
  upvotes: number
  downvotes: number
  score: number
  user_vote: number | null
}

export interface NukeSuggestionItem {
  id: number
  info_hash: string
  torrent_name: string | null
  reason: string
  created_at: string
  suggestion_count: number
}

export interface PendingNukesResponse {
  suggestions: NukeSuggestionItem[]
  total: number
}

export interface LogEntry {
  timestamp: number
  category: string
  message: string
  level: string
}

export interface LogsResponse {
  entries: LogEntry[]
  categories: string[]
  debug_enabled: boolean
}

export interface IdentityStatus {
  initialized: boolean
  needs_onboarding: boolean
  contributor_id: string | null
  public_key: string | null
  recovery_key: string | null
}

export interface EpochInfo {
  epoch: number
  reason: string
  effective_at: string
  grace_hours: number
  purge_policy: string
  maintainer_pubkey_set: boolean
}

export interface BanListResponse {
  bans: Record<string, string>
  total: number
}

export interface UploadResultItem {
  info_hash: string
  name: string
  size: number
  content_type: string | null
  private: boolean
  status: 'imported' | 'duplicate' | 'error'
  message: string | null
}

export interface UploadResponse {
  results: UploadResultItem[]
  imported: number
  duplicates: number
  errors: number
}

export interface MagnetResultItem {
  info_hash: string
  name: string | null
  status: 'queued' | 'duplicate' | 'error'
  message: string | null
}

export interface MagnetUploadResponse {
  results: MagnetResultItem[]
  queued: number
  duplicates: number
  errors: number
}

export interface ScraperSourceStatus {
  name: string
  last_count: number
}

export interface ScraperSourceCount {
  source: string
  count: number
}

export interface ScraperStatus {
  running: boolean
  enabled: boolean
  cycle: number
  interval: number
  enabled_sources: string[]
  last_cycle_total: number
  last_cycle_results: ScraperSourceStatus[]
  total_scraped: number
  uptime_seconds: number
  last_cycle_ago: number | null
  source_counts: ScraperSourceCount[]
}

export interface SyncPeer {
  peer_id: string
  url: string
  source: string
  healthy: boolean
  fail_count: number
  last_seen: string | null
  last_sequence: number
}

export interface SyncActivityEntry {
  timestamp: string
  event: string
  message: string
  peer_id: string | null
  detail: string | null
}

export interface SyncChannel {
  name: string
  running: boolean
}

export interface BootstrapProgress {
  active: boolean
  phase: 'connecting' | 'downloading' | 'importing' | 'complete'
  started_at: string | null
  peers_found: number
  deltas_downloaded: number
  deltas_total: number
  records_imported: number
  records_total: number
}

export interface SyncDashboard {
  contributor_id: string | null
  enabled: boolean
  sequence: number
  total_imported: number
  total_exported: number
  gossip_rounds: number
  last_gossip_at: string | null
  last_export_at: string | null
  last_discovery_at: string | null
  import_interval: number
  export_interval: number
  discovery_interval: number
  gossip_fanout: number
  max_peers: number
  peers: SyncPeer[]
  channels: SyncChannel[]
  activity: SyncActivityEntry[]
  bootstrap: BootstrapProgress | null
}

export interface TrendingTorrent {
  info_hash: string
  name: string
  size: number
  content_type: string | null
  seed_count: number
  peer_count: number
  resolved_at: string | null
}

export interface TrendingResponse {
  results: TrendingTorrent[]
}

export interface RecentComment {
  id: number
  info_hash: string
  torrent_name: string | null
  nickname: string
  body: string
  created_at: string
}

export interface RecentCommentsResponse {
  comments: RecentComment[]
}

export interface AnnouncerStatus {
  running: boolean
  enabled: boolean
  pool_size: number
  pool_active: number
  pool_settled: number
  settle_time: number
  rotate_interval: number
  port: number
  download_nfo: boolean
  announced_count: number
  pending_announce_count: number
  total_announced_session: number
  uptime_seconds: number
  proxy_configured: boolean
  oldest_validation: string | null
  validated_24h: number
  validated_7d: number
}
