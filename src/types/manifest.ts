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
};

export type LaunchConfig = {
  runner: string;
  executable: string | null;
  args: string[];
};

export type UpdateConfig = {
  strategy: string;
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
