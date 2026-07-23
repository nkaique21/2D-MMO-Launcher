export type GameManifest = {
  schemaVersion?: number;
  id: string;
  name: string;
  description: string;
  assets: ManifestAssets;
  installation: InstallationConfig;
  launch: LaunchConfig;
  update: UpdateConfig;
  verification?: VerificationConfig;
};

export type VerificationConfig = {
  requiredFiles?: string[];
  checksums?: VerificationChecksumConfig[];
};

export type VerificationChecksumConfig = {
  path: string;
  algorithm: 'crc32' | string;
  value: string;
};

export type ManifestAssets = {
  banner: string;
  icon: string;
};

export type InstallationConfig = {
  methods: InstallMethod[];
};

export type InstallMethod = {
  type: string;
  label: string;
  url?: string;
  runner?: string;
  compatPrefix?: string;
  installPath?: string;
  launchAfterInstall?: boolean;
  format?: string;
  stripTopLevelDir?: boolean;
  headers?: Record<string, string>;
};

export type LaunchConfig = {
  runner: string;
  executable: string | null;
  args: string[];
  env?: Record<string, string>;
  unsetEnv?: string[];
  battlEye?: BattlEyeConfig | null;
};

export type BattlEyeConfig = {
  enabled?: boolean;
  executable: string;
  args?: string[];
  installArgs?: string[];
  installBeforeLaunch?: boolean;
  launchMode?: "beforeMain" | "main" | "replaceMain" | string;
  pathBase?: string;
  workingDir?: string;
  workingDirBase?: string;
  required?: boolean;
};

export type UpdateConfig = {
  strategy: string;
  runner?: string;
  compatPrefix?: string;
  executable?: string;
  args?: string[];
  pathBase?: string;
  workingDir?: string;
  workingDirBase?: string;
  env?: Record<string, string>;
  unsetEnv?: string[];
  manifestUrl?: string;
  manifestFormat?: string;
  targetDir?: string;
  targetDirBase?: string;
  maxConcurrentDownloads?: number;
};

export type GameInstall = {
  gameId: string;
  installPath: string;
  runnerOverride: string | null;
  createdAt: string;
  updatedAt: string;
};

export type GameSettings = {
  gameId: string;
  runnerOverride: string | null;
  envOverrides: Record<string, string>;
  createdAt: string | null;
  updatedAt: string | null;
};

export type LaunchResult = {
  gameId: string;
  runner: string;
  command: string;
  workingDir: string;
  logPath: string | null;
};

export type GameProcessStatus = 'starting' | 'running' | 'exited' | 'failed';

export type GameProcessState = {
  executionId: string;
  gameId: string;
  status: GameProcessStatus;
  processId: number | null;
  runner: string | null;
  sessionId: number | null;
  startedAt: number | null;
  endedAt: number | null;
  exitCode: number | null;
  error: string | null;
};

export type GameActivity = {
  gameId: string;
  process: GameProcessState | null;
  totalPlaytimeSeconds: number;
  completedSessions: number;
  lastPlayedAt: string | null;
};

export type PlaytimeSession = {
  id: number;
  gameId: string;
  processId: number | null;
  runner: string | null;
  startedAt: string;
  endedAt: string | null;
  durationSeconds: number | null;
  exitCode: number | null;
  endReason: string | null;
};

export type InstallVerificationResult = {
  gameId: string;
  valid: boolean;
  installPath: string;
  installPathExists: boolean;
  executablePath: string | null;
  executableExists: boolean;
  missingFiles: string[];
  checksumResults: ChecksumVerificationResult[];
  issues: string[];
  repairStrategy: 'archive' | 'remoteManifest' | 'windowsInstaller' | 'existing' | string | null;
};

export type ChecksumVerificationResult = {
  path: string;
  algorithm: string;
  expected: string;
  actual: string | null;
  valid: boolean;
};

export type GameUpdateResult = {
  gameId: string;
  checkedFiles: number;
  updatedFiles: number;
  skippedFiles: number;
  downloadedBytes: number;
  targetDir: string;
  logPath: string | null;
};

export type GameUpdateProgress = {
  gameId: string;
  status: string;
  stage: string | null;
  stageLabel: string | null;
  checkedFiles: number;
  updatedFiles: number;
  totalFiles: number;
  currentFile: string | null;
  message: string;
  targetDir: string | null;
  logPath: string | null;
  error: string | null;
};

export type RunnerInfo = {
  id: string;
  kind: string;
  label: string;
  status: string;
  source: string;
  path: string | null;
  installable: boolean;
  installHint: string | null;
  managed: boolean;
  version: string | null;
  canRemove: boolean;
};

export type ManagedRunner = {
  id: string;
  kind: string;
  version: string;
  label: string;
  source: string;
  installPath: string;
  executablePath: string;
  status: string;
  createdAt: string;
  updatedAt: string;
};

export type ManagedRunnerRelease = {
  version: string;
  name: string;
  downloadUrl: string;
  size: number;
  releaseUrl: string;
  installed: boolean;
  runnerId: string;
};

export type RunnerInstallProgress = {
  status: string;
  stage: string;
  version: string;
  downloadedBytes: number;
  totalBytes: number;
  message: string;
  error: string | null;
};

export type CatalogStatus = {
  activeSource: 'remote-cache' | 'embedded' | string;
  remoteUrl: string;
  catalogVersion: string | null;
  generatedAt: string | null;
  lastCheckedAt: number | null;
  lastUpdatedAt: number | null;
  lastError: string | null;
  gameCount: number;
};
