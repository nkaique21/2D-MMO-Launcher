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
};

export type LaunchConfig = {
  runner: string;
  executable: string | null;
  args: string[];
};

export type UpdateConfig = {
  strategy: string;
};
