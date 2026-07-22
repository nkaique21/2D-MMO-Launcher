export type GameManifest = {
  id: string;
  name: string;
  description: string;
  assets: ManifestAssets;
  installation: InstallationConfig;
  launch: LaunchConfig;
  update: UpdateConfig;
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

export type LaunchResult = {
  gameId: string;
  runner: string;
  command: string;
  workingDir: string;
  logPath: string | null;
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
};
