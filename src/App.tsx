import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  listGames,
  listInstalls,
  listRunners,
  getGameActivity,
  getLatestProtonGeRelease,
  installLatestProtonGe,
  removeManagedRunner,
  downloadAndInstallArchive,
  downloadAndRunInstaller,
  launchGame,
  locateExistingInstall,
  openInstallFolder,
  removeInstall,
  runGameUpdate,
  runGameRemoteUpdate,
  getGameUpdateProgress,
  getGameSettings,
  installGameFromRemoteManifest,
  resetGameSettings,
  saveGameSettings,
  verifyGameInstall,
} from './lib/tauri';
import type { GameActivity, GameInstall, GameManifest, GameProcessState, GameSettings, GameUpdateProgress, InstallVerificationResult, ManagedRunnerRelease, RunnerInfo, RunnerInstallProgress } from './types/manifest';

type InstallationStatus = 'installed' | 'available';

type GameVisualMetadata = {
  shortName: string;
  accent: string;
  softAccent: string;
  meta: string;
};

type GameViewModel = GameManifest &
  GameVisualMetadata & {
    installLabel: string;
    protonOnly: boolean;
    runnerLabel: string;
    status: InstallationStatus;
  };

type SecondaryAction = {
  id: string;
  label: string;
  installMethodType?: string;
  type: 'installedAction' | 'installMethod' | 'manifestDetails' | 'runnerSettings';
};

type UpdateStageDefinition = {
  id: string;
  label: string;
};

type InstallFlowProgress = {
  gameId: string;
  status: string;
  message: string;
};

const fallbackVisualMetadata: GameVisualMetadata = {
  shortName: '2D',
  accent: 'from-purple-500 via-indigo-500 to-sky-500',
  softAccent: 'bg-purple-500/15 text-purple-100 ring-purple-300/20',
  meta: 'MMORPG 2D • Manifesto local',
};

// Metadados puramente visuais até esses campos evoluírem para o manifesto.
// Não representam regras de negócio nem estado local de instalação.
const visualMetadataByGameId: Record<string, GameVisualMetadata> = {
  ravenquest: {
    shortName: 'RQ',
    accent: 'from-violet-500 via-fuchsia-500 to-rose-500',
    softAccent: 'bg-violet-500/15 text-violet-100 ring-violet-300/20',
    meta: 'Sandbox • Open world',
  },
  archlight: {
    shortName: 'AL',
    accent: 'from-orange-400 via-red-500 to-purple-700',
    softAccent: 'bg-orange-500/15 text-orange-100 ring-orange-300/20',
    meta: 'Seasonal • Custom systems',
  },
  pokexgames: {
    shortName: 'PXG',
    accent: 'from-indigo-500 via-purple-500 to-sky-500',
    softAccent: 'bg-indigo-500/15 text-indigo-100 ring-indigo-300/20',
    meta: 'Monster catching • Manual',
  },
  'grand-line-adventures': {
    shortName: 'GLA',
    accent: 'from-sky-400 via-cyan-500 to-blue-700',
    softAccent: 'bg-sky-500/15 text-sky-100 ring-sky-300/20',
    meta: 'Anime • Adventure',
  },
  pokemmo: {
    shortName: 'PM',
    accent: 'from-red-500 via-amber-400 to-sky-600',
    softAccent: 'bg-red-500/15 text-red-100 ring-red-300/20',
    meta: 'Monster catching • Online world',
  },
  medivia: {
    shortName: 'MV',
    accent: 'from-amber-300 via-yellow-600 to-stone-900',
    softAccent: 'bg-amber-500/15 text-amber-100 ring-amber-300/20',
    meta: 'Old school • Exploration',
  },
};

function buildShortName(name: string) {
  const initials = name
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => word[0])
    .join('')
    .slice(0, 3)
    .toUpperCase();

  return initials || fallbackVisualMetadata.shortName;
}

function getVisualMetadata(game: GameManifest): GameVisualMetadata {
  return visualMetadataByGameId[game.id] ?? {
    ...fallbackVisualMetadata,
    shortName: buildShortName(game.name),
  };
}

function formatRunner(runner: string) {
  const normalizedRunner = runner.toLowerCase();

  if (normalizedRunner === 'native') return 'Native';
  if (normalizedRunner === 'proton') return 'Proton';
  if (normalizedRunner === 'wine') return 'Wine';
  if (normalizedRunner === 'steam') return 'Steam';

  return runner || 'Não definido';
}

function formatRunnerStatus(status: string) {
  if (status === 'available') return 'Disponível';
  if (status === 'installable') return 'Instalável';

  return status || 'Indefinido';
}

function formatLaunchMessage(prefix: string, result: { runner: string; command: string; logPath: string | null }) {
  const logMessage = result.logPath ? ` Log: ${result.logPath}` : '';

  return `${prefix} via ${result.runner}: ${result.command}.${logMessage}`;
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;

  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

const integerFormatter = new Intl.NumberFormat('pt-BR');

function formatInteger(value: number) {
  return integerFormatter.format(value);
}

function formatUpdateStatus(status: string) {
  if (status === 'preparing') return 'Preparando';
  if (status === 'manifest') return 'Manifesto';
  if (status === 'checking') return 'Verificando';
  if (status === 'downloading') return 'Baixando';
  if (status === 'validating') return 'Validando';
  if (status === 'applying') return 'Aplicando';
  if (status === 'error') return 'Erro';
  if (status === 'done') return 'Concluído';

  return status || 'Atualizando';
}

const remoteUpdateStages: UpdateStageDefinition[] = [
  { id: 'start', label: 'Preparar update' },
  { id: 'openDatabase', label: 'Banco local' },
  { id: 'loadInstall', label: 'Instalação' },
  { id: 'loadLocalManifest', label: 'Manifesto local' },
  { id: 'reconcileInstall', label: 'Reconciliar' },
  { id: 'spawnBlockingTask', label: 'Background' },
  { id: 'resolveRemoteManifest', label: 'Config remota' },
  { id: 'resolveTargetDir', label: 'Pasta alvo' },
  { id: 'prepareTargetDir', label: 'Preparar pasta' },
  { id: 'downloadRemoteManifest', label: 'Baixar manifesto' },
  { id: 'decodeRemoteManifest', label: 'Decodificar' },
  { id: 'buildFileList', label: 'Lista de arquivos' },
  { id: 'checkingFiles', label: 'Verificar arquivos' },
  { id: 'planUpdate', label: 'Plano de update' },
  { id: 'prepareStagingDir', label: 'Preparar staging' },
  { id: 'downloadingFiles', label: 'Baixar divergentes' },
  { id: 'validateStagedFiles', label: 'Validar staging' },
  { id: 'applyStagedFiles', label: 'Aplicar arquivos' },
  { id: 'done', label: 'Concluído' },
];

function createPreparingUpdateProgress(gameId: string): GameUpdateProgress {
  return {
    gameId,
    status: 'preparing',
    stage: 'start',
    stageLabel: 'Preparar update',
    checkedFiles: 0,
    updatedFiles: 0,
    totalFiles: 0,
    currentFile: null,
    message: 'Preparando atualização dos arquivos...',
    targetDir: null,
    logPath: null,
    error: null,
  };
}

function getUpdateStageIndex(progress: GameUpdateProgress | null) {
  if (!progress) return -1;

  const stageIndex = remoteUpdateStages.findIndex((stage) => stage.id === progress.stage);

  if (stageIndex >= 0) return stageIndex;
  if (progress.status === 'manifest') return remoteUpdateStages.findIndex((stage) => stage.id === 'downloadRemoteManifest');
  if (progress.status === 'checking') return remoteUpdateStages.findIndex((stage) => stage.id === 'checkingFiles');
  if (progress.status === 'downloading') return remoteUpdateStages.findIndex((stage) => stage.id === 'downloadingFiles');
  if (progress.status === 'validating') return remoteUpdateStages.findIndex((stage) => stage.id === 'validateStagedFiles');
  if (progress.status === 'applying') return remoteUpdateStages.findIndex((stage) => stage.id === 'applyStagedFiles');
  if (progress.status === 'done') return remoteUpdateStages.length - 1;

  return 0;
}

function formatElapsedSeconds(timestamp: number | null, now: number) {
  if (!timestamp) return 'sem evento ainda';

  const elapsedSeconds = Math.max(0, Math.floor((now - timestamp) / 1000));

  if (elapsedSeconds <= 1) return 'agora mesmo';

  return `há ${elapsedSeconds}s`;
}

function formatPlaytime(totalSeconds: number) {
  const safeSeconds = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safeSeconds / 3600);
  const minutes = Math.floor((safeSeconds % 3600) / 60);

  if (safeSeconds === 0) return '0 min';
  if (hours > 0) return `${hours}h ${minutes.toString().padStart(2, '0')}min`;
  if (minutes > 0) return `${minutes} min`;
  return '< 1 min';
}

function formatLastPlayed(timestamp: string | null | undefined) {
  if (!timestamp) return 'Ainda não jogado';

  const seconds = Number(timestamp);
  if (!Number.isFinite(seconds)) return timestamp;

  return new Date(seconds * 1000).toLocaleString('pt-BR', {
    dateStyle: 'short',
    timeStyle: 'short',
  });
}

function isUpdateFinished(progress: GameUpdateProgress | null) {
  return progress?.status === 'done' || progress?.status === 'error';
}

function isRemoteUpdateAction(actionId: string | null) {
  return actionId === 'run-remote-update' || actionId === 'repair-files';
}

function getUpdatePercent(progress: GameUpdateProgress | null) {
  if (!progress) return 0;
  if (progress.totalFiles <= 0) return progress.status === 'manifest' ? 5 : 0;

  return Math.min(100, Math.max(1, Math.round((progress.checkedFiles / progress.totalFiles) * 100)));
}

function toViewModel(game: GameManifest, installedGameIds: Set<string>): GameViewModel {
  const status: InstallationStatus = installedGameIds.has(game.id) ? 'installed' : 'available';
  const runner = game.launch.runner.toLowerCase();

  return {
    ...game,
    ...getVisualMetadata(game),
    status,
    installLabel: status === 'installed' ? 'Instalado' : 'Manifesto disponível',
    protonOnly: runner === 'proton',
    runnerLabel: formatRunner(game.launch.runner),
  };
}

function getSecondaryActions(game: GameViewModel): SecondaryAction[] {
  if (game.status === 'installed') {
    const installedActions: SecondaryAction[] = [
      { id: 'verify-files', label: 'Verificar arquivos', type: 'installedAction' },
      { id: 'open-folder', label: 'Abrir pasta', type: 'installedAction' },
      { id: 'remove-install', label: 'Desvincular instalação', type: 'installedAction' },
      { id: 'configure', label: 'Configurar', type: 'installedAction' },
    ];

    if (game.update.strategy === 'remoteManifest') {
      installedActions.unshift({
        id: 'run-remote-update',
        label: 'Atualizar arquivos do jogo',
        type: 'installedAction',
      });
    }

    if (game.update.strategy === 'externalLauncher') {
      installedActions.unshift({
        id: 'run-update',
        label: 'Atualizar pelo launcher oficial',
        type: 'installedAction',
      });
    }

    return installedActions;
  }

  const installActions = game.installation.methods.map<SecondaryAction>((method) => ({
    id: `install-method:${method.type}`,
    installMethodType: method.type,
    label: method.label,
    type: 'installMethod',
  }));

  return [
    ...installActions,
    { id: 'manifest-details', label: 'Detalhes do manifesto', type: 'manifestDetails' },
    { id: 'runner-settings', label: 'Configurar runner', type: 'runnerSettings' },
  ];
}

function App() {
  const [manifests, setManifests] = useState<GameManifest[]>([]);
  const [installs, setInstalls] = useState<GameInstall[]>([]);
  const [runners, setRunners] = useState<RunnerInfo[]>([]);
  const [gameActivity, setGameActivity] = useState<GameActivity | null>(null);
  const [selectedGameId, setSelectedGameId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [pendingActionId, setPendingActionId] = useState<string | null>(null);
  const [updateProgress, setUpdateProgress] = useState<GameUpdateProgress | null>(null);
  const [updateProgressReceivedAt, setUpdateProgressReceivedAt] = useState<number | null>(null);
  const [updateProgressSource, setUpdateProgressSource] = useState<string | null>(null);
  const [nowTimestamp, setNowTimestamp] = useState(() => Date.now());
  const [isLaunching, setIsLaunching] = useState(false);
  const [isDetailsOpen, setIsDetailsOpen] = useState(false);
  const [isLibraryOpen, setIsLibraryOpen] = useState(false);
  const [installFlow, setInstallFlow] = useState<InstallFlowProgress | null>(null);
  const [verificationResult, setVerificationResult] = useState<InstallVerificationResult | null>(null);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [gameSettings, setGameSettings] = useState<GameSettings | null>(null);
  const [settingsRunner, setSettingsRunner] = useState('');
  const [settingsEnv, setSettingsEnv] = useState<Record<string, string>>({});
  const [runnerRelease, setRunnerRelease] = useState<ManagedRunnerRelease | null>(null);
  const [runnerProgress, setRunnerProgress] = useState<RunnerInstallProgress | null>(null);
  const [runnerActionId, setRunnerActionId] = useState<string | null>(null);
  const [reloadSignal, setReloadSignal] = useState(0);

  async function refreshRunnerCatalog() {
    setRunnerActionId('catalog');

    try {
      const release = await getLatestProtonGeRelease();
      setRunnerRelease(release);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setRunnerActionId(null);
    }
  }

  async function installProtonGe() {
    setRunnerActionId('install');
    setActionError(null);
    setRunnerProgress({ status: 'preparing', stage: 'catalog', version: runnerRelease?.version ?? '', downloadedBytes: 0, totalBytes: runnerRelease?.size ?? 0, message: 'Preparando instalação do Proton-GE...', error: null });

    try {
      const installed = await installLatestProtonGe();
      const [detectedRunners, release] = await Promise.all([listRunners(), getLatestProtonGeRelease()]);
      setRunners(detectedRunners);
      setRunnerRelease(release);
      setActionMessage(`${installed.label} instalado e disponível nas configurações por jogo.`);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setRunnerActionId(null);
    }
  }

  async function uninstallManagedRunner(runner: RunnerInfo) {
    if (!window.confirm(`Remover ${runner.label} do launcher? Jogos configurados com este ID precisarão escolher outro runner.`)) return;
    setRunnerActionId(`remove:${runner.id}`);
    setActionError(null);

    try {
      await removeManagedRunner(runner.id);
      const detectedRunners = await listRunners();
      setRunners(detectedRunners);
      setRunnerRelease((current) => current?.runnerId === runner.id ? { ...current, installed: false } : current);
      setActionMessage(`${runner.label} removido do launcher.`);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setRunnerActionId(null);
    }
  }

  async function openGameSettings() {
    setPendingActionId('load-settings');
    setActionError(null);

    try {
      const settings = await getGameSettings(selectedGame.id);
      setGameSettings(settings);
      setSettingsRunner(settings.runnerOverride ?? '');
      setSettingsEnv(settings.envOverrides);
      setIsSettingsOpen(true);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setPendingActionId(null);
    }
  }

  async function persistGameSettings() {
    setPendingActionId('save-settings');
    setActionError(null);

    try {
      const envOverrides = Object.fromEntries(
        Object.entries(settingsEnv).filter(([, value]) => value !== ''),
      );
      const settings = await saveGameSettings(selectedGame.id, settingsRunner || null, envOverrides);
      setGameSettings(settings);
      setSettingsEnv(settings.envOverrides);
      setActionMessage('Configurações locais salvas. Elas serão aplicadas no próximo launch/update.');
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setPendingActionId(null);
    }
  }

  async function restoreGameSettings() {
    setPendingActionId('reset-settings');
    setActionError(null);

    try {
      const settings = await resetGameSettings(selectedGame.id);
      setGameSettings(settings);
      setSettingsRunner('');
      setSettingsEnv({});
      setActionMessage('Defaults do manifesto restaurados.');
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setPendingActionId(null);
    }
  }

  useEffect(() => {
    let isMounted = true;

    setIsLoading(true);
    setLoadError(null);

    Promise.all([listGames(), listInstalls(), listRunners()])
      .then(([catalog, localInstalls, detectedRunners]) => {
        if (!isMounted) return;

        setManifests(catalog);
        setInstalls(localInstalls);
        setRunners(detectedRunners);
        setSelectedGameId((currentGameId) => {
          const currentGameStillExists = catalog.some((game) => game.id === currentGameId);

          if (currentGameStillExists) return currentGameId;

          return catalog[0]?.id ?? null;
        });
      })
      .catch((error: unknown) => {
        if (!isMounted) return;

        setManifests([]);
        setInstalls([]);
        setRunners([]);
        setSelectedGameId(null);
        setLoadError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => {
        if (isMounted) setIsLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, [reloadSignal]);

  useEffect(() => {
    setIsSettingsOpen(false);
    setGameSettings(null);
    setSettingsRunner('');
    setSettingsEnv({});
    setGameActivity(null);
  }, [selectedGameId]);

  useEffect(() => {
    if (!selectedGameId) return undefined;

    let isMounted = true;

    void getGameActivity(selectedGameId)
      .then((activity) => {
        if (isMounted) setGameActivity(activity);
      })
      .catch((error: unknown) => {
        if (isMounted) setActionError(error instanceof Error ? error.message : String(error));
      });

    return () => {
      isMounted = false;
    };
  }, [selectedGameId]);

  useEffect(() => {
    let isMounted = true;
    const unlistenPromise = listen<GameActivity>('game-activity-updated', (event) => {
      if (!isMounted || event.payload.gameId !== selectedGameId) return;
      setGameActivity(event.payload);
    });

    return () => {
      isMounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [selectedGameId]);

  useEffect(() => {
    if (!selectedGameId) return undefined;

    const processIsActive = gameActivity?.gameId === selectedGameId
      && (gameActivity.process?.status === 'starting' || gameActivity.process?.status === 'running');

    if (!processIsActive) return undefined;

    let isMounted = true;
    let requestInFlight = false;

    const refreshActivity = async () => {
      if (requestInFlight) return;
      requestInFlight = true;

      try {
        const activity = await getGameActivity(selectedGameId);
        if (isMounted) setGameActivity(activity);
      } catch (error) {
        if (isMounted) {
          console.warn('Não foi possível reconciliar o estado do processo do jogo.', error);
        }
      } finally {
        requestInFlight = false;
      }
    };

    const intervalId = window.setInterval(() => {
      void refreshActivity();
    }, 2_000);

    return () => {
      isMounted = false;
      window.clearInterval(intervalId);
    };
  }, [selectedGameId, gameActivity?.gameId, gameActivity?.process?.status]);

  useEffect(() => {
    let isMounted = true;

    const unlistenPromise = listen<GameInstall>('install-updated', (event) => {
      if (!isMounted) return;

      setInstalls((currentInstalls) => {
        const otherInstalls = currentInstalls.filter((install) => install.gameId !== event.payload.gameId);

        return [...otherInstalls, event.payload];
      });
      setSelectedGameId(event.payload.gameId);
    });

    return () => {
      isMounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    let isMounted = true;
    const unlistenPromise = listen<RunnerInstallProgress>('runner-install-progress', (event) => {
      if (isMounted) setRunnerProgress(event.payload);
    });

    return () => {
      isMounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    const unlistenPromise = listen<InstallFlowProgress>('game-install-flow', (event) => {
      if (!isMounted) return;

      setInstallFlow(event.payload);
      setActionMessage(event.payload.message);

      if (event.payload.status === 'error') {
        setActionError(event.payload.message);
        setPendingActionId(null);
      }

      if (event.payload.status === 'done') {
        setActionError(null);
        setPendingActionId(null);
        void listInstalls().then(setInstalls);
      }
    });

    return () => {
      isMounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    const unlistenPromise = listen<GameUpdateProgress>('game-update-progress', (event) => {
      if (!isMounted) return;

      setUpdateProgress(event.payload);
      setUpdateProgressReceivedAt(Date.now());
      setUpdateProgressSource('evento Tauri');
    });

    return () => {
      isMounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => setNowTimestamp(Date.now()), 1000);

    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    if (!selectedGameId) return undefined;

    const progressBelongsToSelectedGame = updateProgress?.gameId === selectedGameId;
    const shouldPollRunnerLog = isRemoteUpdateAction(pendingActionId)
      || (progressBelongsToSelectedGame && !isUpdateFinished(updateProgress));

    if (!shouldPollRunnerLog) return undefined;

    let isCancelled = false;

    async function pollRunnerLogProgress() {
      if (!selectedGameId || isCancelled) return;

      const lastProgressAge = updateProgressReceivedAt ? Date.now() - updateProgressReceivedAt : Number.POSITIVE_INFINITY;
      const shouldUseLogFallback = lastProgressAge > 2500
        || updateProgressSource === null
        || updateProgressSource === 'local'
        || updateProgressSource === 'runner.log';

      if (!shouldUseLogFallback) return;

      try {
        const progressFromLog = await getGameUpdateProgress(selectedGameId);

        if (!progressFromLog || isCancelled) return;

        setUpdateProgress(progressFromLog);
        setUpdateProgressReceivedAt(Date.now());
        setUpdateProgressSource('runner.log');

        if (progressFromLog.status === 'done') {
          setPendingActionId((currentActionId) => (
            isRemoteUpdateAction(currentActionId) ? null : currentActionId
          ));
          setActionError(null);
          setActionMessage((currentMessage) => (
            currentMessage === null || currentMessage === 'Preparando atualização dos arquivos...'
              ? 'Update concluído conforme runner.log.'
              : currentMessage
          ));
        }

        if (progressFromLog.status === 'error') {
          setPendingActionId((currentActionId) => (
            isRemoteUpdateAction(currentActionId) ? null : currentActionId
          ));
          setActionError(progressFromLog.error ?? 'Falha detectada no runner.log durante o update remoto.');
        }
      } catch {
        // O fallback por log é diagnóstico e não deve substituir mensagens do fluxo principal.
      }
    }

    const initialTimeout = window.setTimeout(() => void pollRunnerLogProgress(), 1500);
    const interval = window.setInterval(() => void pollRunnerLogProgress(), 2500);

    return () => {
      isCancelled = true;
      window.clearTimeout(initialTimeout);
      window.clearInterval(interval);
    };
  }, [pendingActionId, selectedGameId, updateProgress, updateProgressReceivedAt, updateProgressSource]);

  const installedGameIds = useMemo(
    () => new Set(installs.map((install) => install.gameId)),
    [installs],
  );
  const games = useMemo(
    () => manifests.map((manifest) => toViewModel(manifest, installedGameIds)),
    [installedGameIds, manifests],
  );
  const installedGames = useMemo(() => games.filter((game) => game.status === 'installed'), [games]);
  const manifestGames = useMemo(() => games.filter((game) => game.status === 'available'), [games]);
  const availableRunners = useMemo(
    () => runners.filter((runner) => runner.status === 'available'),
    [runners],
  );
  const installableRunners = useMemo(
    () => runners.filter((runner) => runner.installable || runner.status === 'installable'),
    [runners],
  );
  const managedRunners = useMemo(() => runners.filter((runner) => runner.managed), [runners]);
  const runnerInstallPercent = runnerProgress && runnerProgress.totalBytes > 0
    ? Math.min(100, Math.round((runnerProgress.downloadedBytes / runnerProgress.totalBytes) * 100))
    : 0;

  const selectedGame = useMemo(
    () => games.find((game) => game.id === selectedGameId) ?? games[0] ?? null,
    [games, selectedGameId],
  );
  const selectedInstall = useMemo(
    () => installs.find((install) => install.gameId === selectedGame?.id) ?? null,
    [installs, selectedGame?.id],
  );

  if (isLoading && !selectedGame) {
    return (
      <main className="grid min-h-screen place-items-center bg-launcher-bg px-6 text-launcher-text">
        <section className="max-w-md rounded-[2rem] border border-white/10 bg-launcher-panel p-8 text-center shadow-2xl shadow-black/40">
          <div className="mx-auto grid h-16 w-16 place-items-center rounded-[1.35rem] bg-white/10 ring-1 ring-white/15 shadow-glow">
            <span className="bg-gradient-to-br from-white to-purple-200 bg-clip-text text-lg font-black text-transparent">
              2D
            </span>
          </div>
          <p className="mt-6 text-xs font-black uppercase tracking-[0.28em] text-purple-300">
            Catálogo por manifesto
          </p>
          <h1 className="mt-2 text-2xl font-black tracking-tight">Carregando jogos locais...</h1>
          <p className="mt-3 text-sm leading-6 text-launcher-muted">
            O launcher está lendo os manifestos disponíveis no backend Tauri.
          </p>
        </section>
      </main>
    );
  }

  if (loadError && !selectedGame) {
    return (
      <main className="grid min-h-screen place-items-center bg-launcher-bg px-6 text-launcher-text">
        <section className="max-w-lg rounded-[2rem] border border-red-300/20 bg-launcher-panel p-8 text-center shadow-2xl shadow-black/40">
          <p className="text-xs font-black uppercase tracking-[0.28em] text-red-200">
            Falha ao carregar catálogo
          </p>
          <h1 className="mt-2 text-2xl font-black tracking-tight">Não foi possível listar os manifestos</h1>
          <p className="mt-3 rounded-2xl bg-black/25 p-4 text-left text-sm leading-6 text-launcher-muted ring-1 ring-white/[0.08]">
            {loadError}
          </p>
          <button
            className="mt-6 rounded-2xl bg-white px-6 py-3 text-sm font-black uppercase tracking-[0.16em] text-slate-950 transition hover:-translate-y-0.5 hover:bg-purple-100"
            onClick={() => setReloadSignal((signal) => signal + 1)}
            type="button"
          >
            Tentar novamente
          </button>
        </section>
      </main>
    );
  }

  if (!selectedGame) {
    return (
      <main className="grid min-h-screen place-items-center bg-launcher-bg px-6 text-launcher-text">
        <section className="max-w-md rounded-[2rem] border border-white/10 bg-launcher-panel p-8 text-center shadow-2xl shadow-black/40">
          <p className="text-xs font-black uppercase tracking-[0.28em] text-purple-300">
            Catálogo vazio
          </p>
          <h1 className="mt-2 text-2xl font-black tracking-tight">Nenhum manifesto encontrado</h1>
          <p className="mt-3 text-sm leading-6 text-launcher-muted">
            Adicione arquivos JSON em <strong>src-tauri/manifests</strong> para popular o launcher.
          </p>
        </section>
      </main>
    );
  }

  const secondaryActions = getSecondaryActions(selectedGame);
  const activeUpdateProgress = updateProgress?.gameId === selectedGame.id ? updateProgress : null;
  const updatePercent = getUpdatePercent(activeUpdateProgress);
  const updateStageIndex = getUpdateStageIndex(activeUpdateProgress);
  const lastUpdateEventLabel = formatElapsedSeconds(updateProgressReceivedAt, nowTimestamp);
  const isRemoteUpdateRunning = isRemoteUpdateAction(pendingActionId);
  const activeProcess: GameProcessState | null = gameActivity?.gameId === selectedGame.id
    ? gameActivity.process
    : null;
  const isGameProcessActive = activeProcess?.status === 'starting' || activeProcess?.status === 'running';
  const activeSessionSeconds = activeProcess?.status === 'running' && activeProcess.startedAt
    ? Math.max(0, Math.floor(nowTimestamp / 1000) - activeProcess.startedAt)
    : 0;
  const displayedPlaytimeSeconds = (gameActivity?.totalPlaytimeSeconds ?? 0) + activeSessionSeconds;
  const processStatusLabel = activeProcess?.status === 'starting'
    ? 'Iniciando'
    : activeProcess?.status === 'running'
      ? 'Em execução'
      : null;
  const activityStatusLabel = processStatusLabel
    ?? (activeProcess?.status === 'failed'
      ? 'Falhou'
      : activeProcess?.status === 'exited'
        ? 'Encerrado'
        : 'Parado');

  async function executeRemoteUpdate(actionId: 'run-remote-update' | 'repair-files') {
    const isRepair = actionId === 'repair-files';

    setPendingActionId(actionId);
    setUpdateProgress(createPreparingUpdateProgress(selectedGame.id));
    setUpdateProgressReceivedAt(Date.now());
    setUpdateProgressSource('local');
    setActionError(null);
    setActionMessage(isRepair
      ? 'Preparando reparo dos arquivos pelo manifesto remoto...'
      : 'Preparando atualização dos arquivos...');

    try {
      const result = await runGameRemoteUpdate(selectedGame.id);
      const logMessage = result.logPath ? ` Log: ${result.logPath}` : '';
      const verification = await verifyGameInstall(selectedGame.id);

      setVerificationResult(verification);
      setActionMessage(
        `${isRepair ? 'Reparo' : 'Update'} concluído: ${result.updatedFiles} arquivo(s) baixado(s), ${result.skippedFiles} já estavam atualizados, ${formatBytes(result.downloadedBytes)} transferidos. ${verification.valid
          ? 'A instalação passou na verificação estrutural.'
          : `Ainda restam ${verification.issues.length} problema(s).`}${logMessage}`,
      );
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setPendingActionId(null);
    }
  }

  async function handlePrimaryAction() {
    setActionError(null);
    setActionMessage(null);

    if (selectedGame.status !== 'installed') {
      if (selectedGame.update.strategy === 'remoteManifest') {
        setPendingActionId('primary-install');
        setInstallFlow({ gameId: selectedGame.id, status: 'preparing', message: 'Preparando instalação gerenciada...' });
        setUpdateProgress(createPreparingUpdateProgress(selectedGame.id));
        setUpdateProgressReceivedAt(Date.now());
        setUpdateProgressSource('local');
        setActionMessage('Preparando instalação automática...');

        try {
          const result = await installGameFromRemoteManifest(selectedGame.id);
          setInstalls(await listInstalls());
          setActionMessage(formatLaunchMessage('Jogo instalado e iniciado', result));
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          setActionError(message);
          setInstallFlow({ gameId: selectedGame.id, status: 'error', message });
        } finally {
          setPendingActionId(null);
        }

        return;
      }

      const archiveMethod = selectedGame.installation.methods.find(
        (method) => method.type === 'archive',
      );

      if (archiveMethod) {
        setPendingActionId('primary-install');
        setInstallFlow({ gameId: selectedGame.id, status: 'preparing', message: 'Preparando instalação do arquivo...' });
        setActionMessage('Preparando download do cliente Linux...');

        try {
          const result = await downloadAndInstallArchive(selectedGame.id);
          setInstalls(await listInstalls());
          setActionMessage(formatLaunchMessage('Jogo instalado e iniciado', result));
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          setActionError(message);
          setInstallFlow({ gameId: selectedGame.id, status: 'error', message });
        } finally {
          setPendingActionId(null);
        }

        return;
      }

      const windowsInstallerMethod = selectedGame.installation.methods.find(
        (method) => method.type === 'windowsInstaller',
      );

      if (!windowsInstallerMethod) {
        setActionMessage('Use “Localizar instalação existente” para registrar este jogo antes de jogar.');
        return;
      }

      setPendingActionId('primary-install');
      setInstallFlow({ gameId: selectedGame.id, status: 'downloading', message: 'Baixando o instalador...' });
      setActionMessage('Baixando instalador e preparando runner...');

      try {
        await downloadAndRunInstaller(selectedGame.id);
        const refreshedInstalls = await listInstalls();

        setInstalls(refreshedInstalls);
        setActionMessage('Instalador aberto. Ao concluir, o launcher atualizará os arquivos e abrirá o jogo automaticamente.');

        if (!windowsInstallerMethod.launchAfterInstall) {
          setPendingActionId(null);
        }
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
        setPendingActionId(null);
      }

      return;
    }

    setIsLaunching(true);

    try {
      const result = await launchGame(selectedGame.id);
      const activity = await getGameActivity(selectedGame.id);

      setGameActivity(activity);
      setActionMessage(formatLaunchMessage('Jogo iniciado', result));
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setPendingActionId(null);
      setIsLaunching(false);
    }
  }

  async function handleSecondaryAction(action: SecondaryAction) {
    setActionError(null);
    setActionMessage(null);

    if (action.id === 'configure' || action.type === 'runnerSettings') {
      await openGameSettings();
      return;
    }

    if (action.type === 'installedAction' && action.id === 'verify-files') {
      setPendingActionId(action.id);
      setVerificationResult(null);

      try {
        const result = await verifyGameInstall(selectedGame.id);

        setVerificationResult(result);
        setActionMessage(result.valid
          ? 'Instalação verificada: arquivos essenciais encontrados.'
          : `Verificação encontrou ${result.issues.length} problema(s). Consulte os detalhes.`);
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

      return;
    }

    if (action.type === 'installMethod' && action.installMethodType === 'existing') {
      setPendingActionId(action.id);

      try {
        const install = await locateExistingInstall(selectedGame.id);

        if (!install) {
          setActionMessage('Localização cancelada. Nenhuma instalação foi registrada.');
          return;
        }

        setInstalls((currentInstalls) => {
          const otherInstalls = currentInstalls.filter((currentInstall) => currentInstall.gameId !== install.gameId);

          return [...otherInstalls, install];
        });
        setSelectedGameId(install.gameId);
        setActionMessage(`Instalação registrada em: ${install.installPath}`);
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

      return;
    }

    if (action.type === 'installMethod' && action.installMethodType === 'windowsInstaller') {
      setPendingActionId(action.id);

      try {
        const result = await downloadAndRunInstaller(selectedGame.id);
        const refreshedInstalls = await listInstalls();

        setInstalls(refreshedInstalls);
        setActionMessage(formatLaunchMessage('Instalador baixado e iniciado', result));
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

      return;
    }

    if (action.type === 'installMethod' && action.installMethodType === 'archive') {
      setPendingActionId(action.id);

      try {
        const result = await downloadAndInstallArchive(selectedGame.id);
        setInstalls(await listInstalls());
        setActionMessage(formatLaunchMessage('Jogo instalado e iniciado', result));
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

      return;
    }

    if (action.type === 'installedAction' && action.id === 'open-folder') {
      setPendingActionId(action.id);

      try {
        await openInstallFolder(selectedGame.id);
        setActionMessage('Pasta da instalação aberta.');
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

      return;
    }

    if (action.type === 'installedAction' && action.id === 'run-update') {
      setPendingActionId(action.id);

      try {
        const result = await runGameUpdate(selectedGame.id);

        setActionMessage(formatLaunchMessage('Updater iniciado', result));
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

      return;
    }

    if (action.type === 'installedAction' && action.id === 'run-remote-update') {
      await executeRemoteUpdate('run-remote-update');

      return;
    }

    if (action.type === 'installedAction' && action.id === 'remove-install') {
      setPendingActionId(action.id);

      try {
        const removed = await removeInstall(selectedGame.id);

        if (!removed) {
          setActionMessage('Nenhuma instalação registrada foi encontrada para desvincular.');
          return;
        }

        setInstalls((currentInstalls) => (
          currentInstalls.filter((currentInstall) => currentInstall.gameId !== selectedGame.id)
        ));
        setActionMessage('Instalação desvinculada. O jogo voltou para o catálogo.');
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

      return;
    }

    setActionMessage('Ação preparada para a próxima etapa do MVP.');
  }

  return (
    <main className="h-screen min-h-[560px] overflow-hidden bg-launcher-bg text-launcher-text">
      <div className="relative flex h-full bg-launcher-bg">
        <aside className="relative z-30 flex w-[84px] shrink-0 flex-col items-center border-r border-white/10 bg-black/45 px-3 py-4 backdrop-blur-2xl">
          <div className="grid h-12 w-12 place-items-center rounded-2xl bg-white/10 ring-1 ring-white/15 shadow-glow">
            <span className="bg-gradient-to-br from-white to-purple-200 bg-clip-text text-lg font-black text-transparent">
              2D
            </span>
          </div>

          <div className="mt-5 flex min-h-0 w-full flex-1 flex-col items-center gap-2 overflow-hidden">
            {installedGames.length > 0 ? (
              installedGames.map((game) => {
                const isActive = game.id === selectedGame.id;

                return (
                  <button
                    aria-label={game.name}
                    className={`group relative grid h-13 w-13 shrink-0 place-items-center rounded-2xl border transition duration-200 ${
                      isActive
                        ? 'border-purple-300/70 bg-white/15 shadow-glow'
                        : 'border-white/10 bg-white/[0.055] hover:border-white/25 hover:bg-white/10'
                    }`}
                    key={game.id}
                    onClick={() => setSelectedGameId(game.id)}
                    title={game.name}
                    type="button"
                  >
                    <span className={`absolute inset-1.5 rounded-xl bg-gradient-to-br ${game.accent} opacity-80`} />
                    <span className="relative text-[0.68rem] font-black tracking-tight text-white drop-shadow">
                      {game.shortName}
                    </span>
                    {isActive && <span className="absolute -right-[15px] h-7 w-0.5 rounded-full bg-white" />}
                  </button>
                );
              })
            ) : (
              <p className="max-w-16 text-center text-[0.65rem] font-semibold leading-4 text-launcher-muted">
                Nenhuma instalação registrada
              </p>
            )}
          </div>

          <button
            className="grid h-11 w-11 place-items-center rounded-2xl border border-white/10 bg-white/[0.055] text-xl text-launcher-muted transition hover:border-purple-300/40 hover:text-white"
            onClick={() => setIsLibraryOpen(true)}
            title="Abrir catálogo"
            type="button"
          >
            +
          </button>
        </aside>

        <section className="relative flex min-w-0 flex-1 flex-col">
          <header className="pointer-events-none absolute left-6 right-6 top-5 z-20 flex h-8 items-center justify-between">
            <p className="text-[0.65rem] font-black uppercase tracking-[0.28em] text-white/45">
              2D MMO Launcher
            </p>

            <div className="flex items-center gap-2 rounded-full border border-white/10 bg-black/35 px-3 py-2 text-[0.68rem] font-semibold text-white/55 backdrop-blur-md">
              <span className={`h-2 w-2 rounded-full ${loadError ? 'bg-amber-300 shadow-[0_0_18px_rgba(252,211,77,0.75)]' : 'bg-emerald-400 shadow-[0_0_18px_rgba(52,211,153,0.75)]'}`} />
              {loadError ? 'Catálogo local em modo degradado' : 'Manifestos locais carregados'}
            </div>

            <div className="hidden">
              {manifestGames.length > 0 ? (
                manifestGames.map((game) => {
                  const isActive = game.id === selectedGame.id;

                  return (
                    <button
                      className={`group min-w-[218px] rounded-3xl border p-3 text-left transition duration-200 ${
                        isActive
                          ? 'border-purple-300/60 bg-white/[0.14] shadow-glow'
                          : 'border-white/10 bg-white/[0.055] hover:border-white/25 hover:bg-white/10'
                      }`}
                      key={game.id}
                      onClick={() => setSelectedGameId(game.id)}
                      type="button"
                    >
                      <div className="flex items-center gap-3">
                        <div className={`grid h-11 w-11 shrink-0 place-items-center rounded-2xl bg-gradient-to-br ${game.accent} text-xs font-black shadow-lg shadow-black/30`}>
                          {game.shortName}
                        </div>
                        <div className="min-w-0">
                          <h2 className="truncate text-sm font-black text-white">{game.name}</h2>
                          <p className="mt-0.5 truncate text-xs text-launcher-muted">{game.installLabel}</p>
                        </div>
                      </div>
                      <div className="mt-3 flex items-center gap-2">
                        <span className={`rounded-full px-2.5 py-1 text-[0.63rem] font-black uppercase tracking-[0.16em] ring-1 ${game.softAccent}`}>
                          {game.runnerLabel}
                        </span>
                        {game.protonOnly && (
                          <span className="rounded-full bg-white/[0.08] px-2.5 py-1 text-[0.63rem] font-bold text-purple-100 ring-1 ring-white/10">
                            exclusivo
                          </span>
                        )}
                      </div>
                    </button>
                  );
                })
              ) : (
                <div className="min-w-[260px] rounded-3xl border border-white/10 bg-white/[0.055] p-4 text-sm text-launcher-muted">
                  Nenhuma instalação foi registrada no SQLite ainda. Use os métodos do manifesto para localizar ou instalar jogos.
                </div>
              )}
            </div>
          </header>

          <div className="relative min-h-0 flex-1 overflow-hidden">
            <section className="h-full min-h-0 min-w-0">
              <article className="relative h-full min-h-0 overflow-hidden bg-launcher-panel shadow-2xl shadow-black/40">
                <div
                  className="absolute inset-0 bg-cover bg-center opacity-70"
                  style={{
                    backgroundImage: `linear-gradient(90deg, rgba(7,7,16,0.93) 0%, rgba(7,7,16,0.52) 43%, rgba(7,7,16,0.12) 78%), linear-gradient(0deg, rgba(7,7,16,0.92) 0%, transparent 50%), url(${selectedGame.assets.banner})`,
                  }}
                />
                <div className={`absolute inset-0 bg-gradient-to-br ${selectedGame.accent} opacity-20`} />
                <div className="absolute inset-x-0 bottom-0 h-48 bg-gradient-to-t from-launcher-bg via-launcher-bg/70 to-transparent" />

                <div className="relative flex h-full min-h-0 flex-col p-7 lg:p-8">
                  <div className="flex flex-wrap items-center gap-3">
                    <span className="rounded-full border border-white/[0.12] bg-black/30 px-3 py-1.5 text-xs font-black uppercase tracking-[0.2em] text-white/85 backdrop-blur-md">
                      {selectedGame.status === 'installed' ? 'Na sua biblioteca' : 'Disponível para instalar'}
                    </span>
                    {selectedGame.protonOnly && (
                      <span className="rounded-full border border-purple-200/20 bg-purple-500/[0.18] px-3 py-1.5 text-xs font-black uppercase tracking-[0.2em] text-purple-100 backdrop-blur-md">
                        Proton obrigatório
                      </span>
                    )}
                  </div>

                  <div className="mt-auto flex items-end justify-between gap-6">
                    <section className="flex min-w-0 max-w-[470px] items-center gap-4 rounded-2xl border border-white/10 bg-black/50 p-3 pr-5 shadow-2xl backdrop-blur-xl">
                      <div className={`grid h-16 w-16 shrink-0 place-items-center rounded-xl bg-gradient-to-br ${selectedGame.accent} text-sm font-black shadow-lg shadow-black/30`}>
                        {selectedGame.shortName}
                      </div>
                      <div className="min-w-0">
                        <p className="truncate text-[0.65rem] font-black uppercase tracking-[0.2em] text-purple-200/80">
                          {selectedGame.meta}
                        </p>
                        <h2 className="mt-0.5 truncate text-2xl font-black tracking-tight text-white">
                          {selectedGame.name}
                        </h2>
                        <p className="mt-1 truncate text-xs text-white/50">
                          {selectedGame.description}
                        </p>
                        {selectedGame.status === 'installed' && (
                          <div className="mt-2 flex items-center gap-2 text-[0.68rem] font-bold text-white/70">
                            <span>Tempo jogado: {formatPlaytime(displayedPlaytimeSeconds)}</span>
                            {processStatusLabel && (
                              <span className="rounded-full bg-emerald-400/15 px-2 py-0.5 text-emerald-100 ring-1 ring-emerald-300/20">
                                {processStatusLabel}
                              </span>
                            )}
                          </div>
                        )}
                      </div>
                    </section>

                    <div className="flex w-[min(440px,44vw)] shrink-0 flex-col items-end gap-2">
                      {activeUpdateProgress && (
                        <button
                          className={`w-full overflow-hidden rounded-lg border bg-black/55 text-left text-xs shadow-xl backdrop-blur-xl transition hover:bg-black/65 ${
                            activeUpdateProgress.status === 'error'
                              ? 'border-red-300/25 text-red-100'
                              : 'border-white/10 text-white/75'
                          }`}
                          onClick={() => setIsDetailsOpen(true)}
                          type="button"
                        >
                          <div className="flex items-center gap-3 px-3 py-2">
                            <span className="min-w-0 flex-1 truncate font-semibold" title={activeUpdateProgress.message}>
                              {activeUpdateProgress.stageLabel ?? activeUpdateProgress.message}
                            </span>
                            <strong className="shrink-0 text-white/80">{updatePercent}%</strong>
                          </div>
                          <div className="h-0.5 overflow-hidden bg-white/[0.07]">
                            <div className="h-full bg-white/75 transition-all duration-300" style={{ width: `${updatePercent}%` }} />
                          </div>
                        </button>
                      )}

                      <div className="flex items-center justify-end gap-3">
                        <button
                          className="grid h-12 w-12 place-items-center rounded-full border border-white/[0.14] bg-black/50 px-0 text-xl font-bold text-white/70 shadow-lg backdrop-blur-xl transition hover:border-white/25 hover:bg-white/[0.12] hover:text-white"
                          onClick={() => setIsDetailsOpen((open) => !open)}
                          type="button"
                        >
                          ⋯
                        </button>
                        <button
                          className="min-w-[190px] rounded-xl bg-white px-8 py-4 text-sm font-black uppercase tracking-[0.16em] text-slate-950 shadow-[0_18px_60px_rgba(0,0,0,0.35)] transition hover:-translate-y-0.5 hover:bg-purple-100 disabled:cursor-not-allowed disabled:opacity-60"
                          disabled={isLaunching || isGameProcessActive || pendingActionId === 'primary-install' || isRemoteUpdateRunning}
                          onClick={() => void handlePrimaryAction()}
                          type="button"
                        >
                          {activeProcess?.status === 'running'
                            ? 'Jogando'
                            : activeProcess?.status === 'starting' || isLaunching
                              ? 'Iniciando...'
                            : pendingActionId === 'primary-install'
                              ? installFlow?.status === 'installing'
                                ? 'Instalando...'
                                : installFlow?.status === 'preparing'
                                  ? 'Preparando...'
                                : installFlow?.status === 'updating'
                                  ? 'Atualizando...'
                                  : installFlow?.status === 'launching'
                                    ? 'Iniciando...'
                                    : 'Baixando...'
                              : selectedGame.status === 'installed'
                                ? 'Jogar'
                                : 'Baixar e instalar'}
                        </button>
                      </div>
                    </div>
                  </div>

                </div>
              </article>
            </section>

            <aside className="pointer-events-none absolute bottom-[112px] right-8 z-20 flex w-[min(440px,42vw)] flex-col items-end gap-2">
              <section className="hidden rounded-[1.75rem] border border-white/10 bg-white/[0.055] p-4 backdrop-blur-2xl">
                <div className="flex items-center gap-4">
                  <div className={`grid h-16 w-16 place-items-center rounded-3xl bg-gradient-to-br ${selectedGame.accent} text-sm font-black shadow-glow`}>
                    {selectedGame.shortName}
                  </div>
                  <div>
                    <p className="text-xs font-black uppercase tracking-[0.2em] text-launcher-muted">
                      Selecionado
                    </p>
                    <h3 className="mt-1 text-xl font-black">{selectedGame.name}</h3>
                  </div>
                </div>

                <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
                  <div className="rounded-2xl bg-black/20 p-3 ring-1 ring-white/[0.08]">
                    <p className="text-xs text-launcher-muted">Runner</p>
                    <p className="mt-1 font-black">{selectedGame.runnerLabel}</p>
                  </div>
                  <div className="rounded-2xl bg-black/20 p-3 ring-1 ring-white/[0.08]">
                    <p className="text-xs text-launcher-muted">Estado</p>
                    <p className="mt-1 font-black">{selectedGame.status === 'installed' ? 'Instalado' : 'Catálogo'}</p>
                  </div>
                </div>

                {selectedInstall && (
                  <div className="mt-3 rounded-2xl bg-black/20 p-3 text-sm ring-1 ring-white/[0.08]">
                    <p className="text-xs text-launcher-muted">Caminho da instalação</p>
                    <p className="mt-2 break-all font-semibold leading-6 text-white/85">
                      {selectedInstall.installPath}
                    </p>
                  </div>
                )}
              </section>

              <section className="hidden min-h-0 flex-1 rounded-[1.75rem] border border-white/10 bg-launcher-panel/80 p-3 shadow-2xl shadow-black/30">
                <div className="grid h-full min-h-0 auto-rows-fr grid-cols-2 gap-1.5">
                  {secondaryActions.map((action) => (
                    <button
                      className="flex min-h-0 items-center justify-between rounded-2xl px-3 py-2 text-left text-xs font-semibold leading-4 text-launcher-muted transition hover:bg-white/[0.065] hover:text-white"
                      disabled={pendingActionId !== null || isLaunching || isGameProcessActive}
                      key={action.id}
                      onClick={() => void handleSecondaryAction(action)}
                      type="button"
                    >
                      <span>{pendingActionId === action.id
                        ? action.id === 'run-remote-update'
                          ? 'Atualizando arquivos...'
                          : 'Processando...'
                        : action.label}</span>
                      <span className="ml-2 shrink-0 text-white/25">›</span>
                    </button>
                  ))}
                </div>
              </section>

              {installFlow?.gameId === selectedGame.id && !['done', 'error'].includes(installFlow.status) && (
                <section className="pointer-events-auto rounded-xl border border-white/10 bg-black/55 px-4 py-2.5 text-xs text-white/70 backdrop-blur-xl">
                  <p className="font-black uppercase tracking-[0.16em]">Preparando jogo</p>
                  <p className="mt-2 leading-5 text-white/75">{installFlow.message}</p>
                </section>
              )}

              <section className="hidden rounded-[1.75rem] border border-white/10 bg-white/[0.055] p-5 backdrop-blur-2xl">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-xs font-black uppercase tracking-[0.2em] text-launcher-muted">
                      Runners
                    </p>
                    <h3 className="mt-1 text-lg font-black">Compatibilidade</h3>
                  </div>
                  <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-xs font-black text-emerald-100 ring-1 ring-emerald-300/15">
                    {availableRunners.length} ativos
                  </span>
                </div>

                <div className="mt-4 space-y-2">
                  {runners.slice(0, 4).map((runner) => (
                    <div
                      className="rounded-2xl bg-black/20 p-3 text-sm ring-1 ring-white/[0.08]"
                      key={runner.id}
                      title={runner.path ?? runner.installHint ?? runner.label}
                    >
                      <div className="flex items-center justify-between gap-3">
                        <p className="font-black text-white/90">{runner.label}</p>
                        <span className={`rounded-full px-2 py-0.5 text-[0.62rem] font-black uppercase tracking-[0.14em] ring-1 ${
                          runner.status === 'available'
                            ? 'bg-emerald-400/10 text-emerald-100 ring-emerald-300/20'
                            : 'bg-amber-400/10 text-amber-100 ring-amber-300/20'
                        }`}
                        >
                          {formatRunnerStatus(runner.status)}
                        </span>
                      </div>
                      <p className="mt-1 text-xs text-launcher-muted">
                        {formatRunner(runner.kind)} • {runner.source}
                      </p>
                    </div>
                  ))}
                </div>

                {installableRunners.length > 0 && (
                  <p className="mt-4 rounded-2xl bg-purple-500/[0.08] p-3 text-xs leading-5 text-purple-100 ring-1 ring-purple-300/15">
                    Se Wine/Proton não estiverem disponíveis no sistema, o launcher já reserva opções gerenciadas para instalação futura.
                  </p>
                )}
              </section>

              {(actionMessage || actionError) && (
                <button className={`pointer-events-auto max-w-full truncate rounded-xl border bg-black/55 px-4 py-2.5 text-xs backdrop-blur-xl ${
                  actionError
                    ? 'border-red-300/25 text-red-100'
                    : 'border-white/10 text-white/65'
                }`}
                onClick={() => setIsDetailsOpen(true)}
                title={actionError ?? actionMessage ?? undefined}
                type="button"
                >
                  {actionError ?? actionMessage}
                </button>
              )}

              <section className="hidden rounded-[1.75rem] border border-purple-300/15 bg-purple-500/[0.065] p-5">
                <p className="text-xs font-black uppercase tracking-[0.2em] text-purple-200">
                  Nota de compatibilidade
                </p>
                <p className="mt-3 text-sm leading-6 text-launcher-muted">
                  Jogos com <strong>runner: proton</strong> no manifesto ficam marcados como Proton obrigatório.
                  Os demais podem manter runner nativo quando o manifesto permitir.
                </p>
              </section>
            </aside>
          </div>
        </section>
      </div>

      {isLibraryOpen && (
        <div className="absolute inset-0 z-40 bg-black/60 backdrop-blur-sm" onClick={() => setIsLibraryOpen(false)} role="presentation">
          <section className="h-full w-[min(420px,90vw)] overflow-y-auto border-r border-white/10 bg-launcher-panel/95 p-5 shadow-2xl" onClick={(event) => event.stopPropagation()}>
            <div className="flex items-center justify-between">
              <div><p className="text-xs font-black uppercase tracking-[0.2em] text-purple-300">Biblioteca</p><h2 className="mt-1 text-2xl font-black">Todos os jogos</h2></div>
              <button className="grid h-10 w-10 place-items-center rounded-xl bg-white/[0.07] text-xl" onClick={() => setIsLibraryOpen(false)} type="button">×</button>
            </div>
            <div className="mt-5 grid gap-2">
              {games.map((game) => (
                <button className={`flex items-center gap-3 rounded-2xl border p-3 text-left transition ${game.id === selectedGame.id ? 'border-purple-300/45 bg-white/10' : 'border-white/[0.07] bg-white/[0.035] hover:bg-white/[0.07]'}`} key={game.id} onClick={() => { setSelectedGameId(game.id); setIsLibraryOpen(false); }} type="button">
                  <span className={`grid h-12 w-12 shrink-0 place-items-center rounded-xl bg-gradient-to-br ${game.accent} text-xs font-black`}>{game.shortName}</span>
                  <span className="min-w-0 flex-1"><strong className="block truncate">{game.name}</strong><span className="mt-1 block truncate text-xs text-launcher-muted">{game.installLabel} • {game.runnerLabel}</span></span><span className="text-white/25">›</span>
                </button>
              ))}
            </div>
          </section>
        </div>
      )}

      {isDetailsOpen && (
        <div className="absolute inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-sm" onClick={() => setIsDetailsOpen(false)} role="presentation">
          <aside className="h-full w-[min(450px,92vw)] overflow-y-auto border-l border-white/10 bg-launcher-panel/95 p-5 shadow-2xl" onClick={(event) => event.stopPropagation()}>
            <div className="flex items-center justify-between gap-3"><div><p className="text-xs font-black uppercase tracking-[0.2em] text-purple-300">Detalhes</p><h2 className="mt-1 text-xl font-black">{selectedGame.name}</h2></div><button className="grid h-10 w-10 place-items-center rounded-xl bg-white/[0.07] text-xl" onClick={() => setIsDetailsOpen(false)} type="button">×</button></div>
            <p className="mt-4 text-sm leading-6 text-launcher-muted">{selectedGame.description}</p>
            {selectedInstall && <p className="mt-3 break-all rounded-xl bg-black/20 p-3 text-xs leading-5 text-white/60 ring-1 ring-white/[0.07]">{selectedInstall.installPath}</p>}
            {selectedGame.status === 'installed' && (
              <section className="mt-4 grid grid-cols-2 gap-2 rounded-2xl border border-emerald-300/15 bg-emerald-500/[0.045] p-3 text-xs">
                <div className="rounded-xl bg-black/20 p-3">
                  <p className="font-black uppercase tracking-[0.14em] text-emerald-200/70">Tempo acumulado</p>
                  <p className="mt-2 text-lg font-black text-white">{formatPlaytime(displayedPlaytimeSeconds)}</p>
                  <p className="mt-1 text-white/35">{gameActivity?.completedSessions ?? 0} sessão(ões) encerrada(s)</p>
                  <p className="mt-1 truncate text-white/30" title={formatLastPlayed(gameActivity?.lastPlayedAt)}>
                    Última: {formatLastPlayed(gameActivity?.lastPlayedAt)}
                  </p>
                </div>
                <div className="rounded-xl bg-black/20 p-3">
                  <p className="font-black uppercase tracking-[0.14em] text-emerald-200/70">Atividade</p>
                  <p className="mt-2 text-lg font-black text-white">{activityStatusLabel}</p>
                  <p className="mt-1 text-white/35">{activeProcess?.processId ? `PID ${activeProcess.processId}` : 'Nenhum processo ativo'}</p>
                </div>
                {activeProcess?.error && (
                  <p className="col-span-2 rounded-xl border border-red-300/15 bg-red-500/[0.06] p-3 text-red-100/80">
                    {activeProcess.error}
                  </p>
                )}
              </section>
            )}
            <section className="mt-5">
              <p className="text-xs font-black uppercase tracking-[0.18em] text-white/40">Ações do jogo</p>
              <div className="mt-3 grid grid-cols-2 gap-2">
                {secondaryActions.map((action) => (
                  <button
                    className="flex min-h-14 items-center justify-between rounded-xl border border-white/[0.08] bg-white/[0.045] px-3 py-2 text-left text-xs font-semibold leading-4 text-white/65 transition hover:bg-white/[0.08] hover:text-white disabled:opacity-45"
                    disabled={pendingActionId !== null || isLaunching || isGameProcessActive}
                    key={action.id}
                    onClick={() => void handleSecondaryAction(action)}
                    type="button"
                  >
                    <span>{pendingActionId === action.id
                      ? action.id === 'run-remote-update' ? 'Atualizando...' : 'Processando...'
                      : action.label}</span>
                    <span className="ml-2 text-white/25">›</span>
                  </button>
                ))}
              </div>
            </section>
            <section className="mt-5 rounded-2xl border border-violet-300/15 bg-violet-500/[0.05] p-4">
              <div className="flex items-start justify-between gap-3">
                <div><p className="text-xs font-black uppercase tracking-[0.16em] text-violet-200">Runners gerenciados</p><p className="mt-1 text-xs leading-5 text-white/50">Instale Proton-GE sem sair do launcher.</p></div>
                <button className="text-xs font-bold text-violet-200/70 hover:text-violet-100 disabled:opacity-40" disabled={runnerActionId !== null} onClick={() => void refreshRunnerCatalog()} type="button">{runnerActionId === 'catalog' ? 'Consultando...' : 'Consultar'}</button>
              </div>
              {runnerRelease ? (
                <div className="mt-3 rounded-xl bg-black/20 p-3 ring-1 ring-white/[0.06]">
                  <div className="flex items-center justify-between gap-3"><div className="min-w-0"><strong className="block truncate text-sm">{runnerRelease.version}</strong><span className="text-[0.68rem] text-white/40">{formatBytes(runnerRelease.size)} • release mais recente</span></div><button className="shrink-0 rounded-lg bg-violet-200 px-3 py-2 text-[0.68rem] font-black text-slate-950 disabled:opacity-45" disabled={runnerActionId !== null || runnerRelease.installed} onClick={() => void installProtonGe()} type="button">{runnerActionId === 'install' ? 'Instalando...' : runnerRelease.installed ? 'Instalado' : 'Instalar'}</button></div>
                </div>
              ) : <button className="mt-3 w-full rounded-xl border border-white/10 bg-white/[0.04] px-3 py-3 text-xs font-bold text-white/60 hover:bg-white/[0.07]" disabled={runnerActionId !== null} onClick={() => void refreshRunnerCatalog()} type="button">Ver Proton-GE mais recente</button>}
              {runnerProgress && (runnerActionId === 'install' || runnerProgress.status === 'error') && (
                <div className="mt-3 text-xs text-white/55"><div className="mb-2 flex justify-between gap-3"><span>{runnerProgress.message}</span><strong>{runnerInstallPercent}%</strong></div><div className="h-1.5 overflow-hidden rounded-full bg-black/30"><div className="h-full bg-gradient-to-r from-violet-300 to-sky-300 transition-all" style={{ width: `${runnerInstallPercent}%` }} /></div>{runnerProgress.error && <p className="mt-2 text-red-200">{runnerProgress.error}</p>}</div>
              )}
              {managedRunners.length > 0 && <div className="mt-3 space-y-2">{managedRunners.map((runner) => <div className="flex items-center justify-between gap-3 rounded-xl border border-white/[0.06] bg-white/[0.025] p-3" key={runner.id}><div className="min-w-0"><strong className="block truncate text-xs">{runner.label}</strong><span className={`text-[0.65rem] ${runner.status === 'available' ? 'text-emerald-200/60' : 'text-amber-200/70'}`}>{formatRunnerStatus(runner.status)}</span></div>{runner.canRemove && <button className="text-[0.68rem] font-bold text-red-200/55 hover:text-red-100 disabled:opacity-40" disabled={runnerActionId !== null} onClick={() => void uninstallManagedRunner(runner)} type="button">{runnerActionId === `remove:${runner.id}` ? 'Removendo...' : 'Remover'}</button>}</div>)}</div>}
            </section>
            {isSettingsOpen && (
              <section className="mt-5 rounded-2xl border border-purple-300/20 bg-purple-500/[0.06] p-4">
                <div className="flex items-start justify-between gap-3">
                  <div><p className="text-xs font-black uppercase tracking-[0.16em] text-purple-200">Configurações locais</p><p className="mt-1 text-xs leading-5 text-white/55">Campos vazios continuam usando o valor do manifesto.</p></div>
                  <button className="text-lg text-white/40 hover:text-white" onClick={() => setIsSettingsOpen(false)} type="button">×</button>
                </div>
                <label className="mt-4 block text-xs font-bold text-white/65">Runner
                  <select className="mt-2 w-full rounded-xl border border-white/10 bg-[#11111b] px-3 py-3 text-sm text-white [color-scheme:dark] outline-none focus:border-purple-300/40" onChange={(event) => setSettingsRunner(event.target.value)} value={settingsRunner}>
                    <option className="bg-[#11111b] text-white" value="">Padrão do manifesto ({formatRunner(selectedGame.launch.runner)})</option>
                    {runners.filter((runner) => runner.status === 'available').map((runner) => <option className="bg-[#11111b] text-white" key={runner.id} value={runner.id}>{runner.label} • {runner.source}</option>)}
                  </select>
                </label>
                <div className="mt-4 space-y-3">
                  {Object.entries(selectedGame.launch.env ?? {}).sort(([left], [right]) => left.localeCompare(right)).map(([key, defaultValue]) => (
                    <label className="block text-xs font-bold text-white/65" key={key}>{key}
                      <input className="mt-2 w-full rounded-xl border border-white/10 bg-black/35 px-3 py-3 font-mono text-xs text-white outline-none placeholder:text-white/25 focus:border-purple-300/40" onChange={(event) => setSettingsEnv((current) => ({ ...current, [key]: event.target.value }))} placeholder={defaultValue} value={settingsEnv[key] ?? ''} />
                      <span className="mt-1 block break-all font-normal text-white/30">Padrão: {defaultValue}</span>
                    </label>
                  ))}
                  {Object.keys(selectedGame.launch.env ?? {}).length === 0 && <p className="rounded-xl bg-black/20 p-3 text-xs text-white/45">Este manifesto não declara variáveis de ambiente ajustáveis.</p>}
                </div>
                <div className="mt-4 grid grid-cols-2 gap-2">
                  <button className="rounded-xl border border-white/10 bg-white/[0.06] px-3 py-3 text-xs font-bold text-white/65 hover:bg-white/10" disabled={pendingActionId !== null} onClick={() => void restoreGameSettings()} type="button">{pendingActionId === 'reset-settings' ? 'Restaurando...' : 'Restaurar padrões'}</button>
                  <button className="rounded-xl bg-white px-3 py-3 text-xs font-black text-slate-950 hover:bg-purple-100" disabled={pendingActionId !== null} onClick={() => void persistGameSettings()} type="button">{pendingActionId === 'save-settings' ? 'Salvando...' : 'Salvar'}</button>
                </div>
                {gameSettings?.updatedAt && <p className="mt-3 text-[0.65rem] text-white/30">Última persistência: {gameSettings.updatedAt}</p>}
              </section>
            )}
            {activeUpdateProgress && (
              <section className="mt-5 rounded-2xl border border-sky-300/20 bg-sky-500/[0.06] p-4 text-xs">
                <div className="flex justify-between gap-3"><div><p className="font-black uppercase tracking-[0.16em] text-sky-200">Diagnóstico do update</p><p className="mt-1 font-semibold">{activeUpdateProgress.message}</p></div><strong className="text-xl">{updatePercent}%</strong></div>
                <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-black/25"><div className="h-full bg-gradient-to-r from-sky-300 to-purple-300" style={{ width: `${updatePercent}%` }} /></div>
                <div className="mt-4 space-y-1 break-all rounded-xl bg-black/20 p-3 leading-5 text-white/65">
                  <p><strong>Etapa:</strong> {activeUpdateProgress.stageLabel ?? '—'}</p><p><strong>Stage:</strong> {activeUpdateProgress.stage ?? '—'}</p><p><strong>Evento:</strong> {lastUpdateEventLabel}</p><p><strong>Fonte:</strong> {updateProgressSource ?? 'aguardando'}</p><p><strong>Arquivo:</strong> {activeUpdateProgress.currentFile ?? '—'}</p><p><strong>Alvo:</strong> {activeUpdateProgress.targetDir ?? '—'}</p><p><strong>Log:</strong> {activeUpdateProgress.logPath ?? '—'}</p>
                </div>
              </section>
            )}
            {verificationResult?.gameId === selectedGame.id && (
              <section className={`mt-5 rounded-2xl border p-4 text-xs ${verificationResult.valid
                ? 'border-emerald-300/20 bg-emerald-500/[0.06]'
                : 'border-amber-300/20 bg-amber-500/[0.06]'
              }`}>
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className={`font-black uppercase tracking-[0.16em] ${verificationResult.valid ? 'text-emerald-200' : 'text-amber-200'}`}>
                      {verificationResult.valid ? 'Instalação íntegra' : 'Instalação requer atenção'}
                    </p>
                    <p className="mt-1 text-white/65">
                      {verificationResult.valid
                        ? 'A pasta, o executável, os arquivos obrigatórios e os checksums configurados estão válidos.'
                        : verificationResult.issues[0] ?? 'Foram encontrados problemas na instalação.'}
                    </p>
                  </div>
                  <strong className="text-lg">{verificationResult.valid ? '✓' : '!'}</strong>
                </div>
                <div className="mt-3 space-y-1 break-all rounded-xl bg-black/20 p-3 leading-5 text-white/60">
                  <p><strong>Pasta:</strong> {verificationResult.installPath}</p>
                  <p><strong>Executável:</strong> {verificationResult.executablePath ?? 'não definido'}</p>
                  <p><strong>Reparo:</strong> {verificationResult.repairStrategy ?? 'manual'}</p>
                  {verificationResult.issues.map((issue) => <p key={issue}>• {issue}</p>)}
                  {verificationResult.missingFiles.map((file) => <p key={file}>• Ausente: {file}</p>)}
                  {verificationResult.checksumResults.map((checksum) => (
                    <p className={checksum.valid ? 'text-emerald-200/75' : 'text-amber-100'} key={`${checksum.algorithm}:${checksum.path}`}>
                      • {checksum.algorithm.toUpperCase()} {checksum.path}: {checksum.valid
                        ? 'válido'
                        : `esperado ${checksum.expected}, obtido ${checksum.actual ?? 'arquivo ausente'}`}
                    </p>
                  ))}
                </div>
                {!verificationResult.valid
                  && verificationResult.installPathExists
                  && verificationResult.repairStrategy === 'remoteManifest' && (
                  <button
                    className="mt-3 w-full rounded-xl border border-sky-300/20 bg-sky-400/10 px-4 py-3 text-xs font-black uppercase tracking-[0.14em] text-sky-100 transition hover:bg-sky-400/15 disabled:cursor-wait disabled:opacity-50"
                    disabled={pendingActionId !== null || isLaunching || isGameProcessActive}
                    onClick={() => void executeRemoteUpdate('repair-files')}
                    type="button"
                  >
                    {pendingActionId === 'repair-files' ? 'Reparando arquivos...' : 'Reparar arquivos pelo manifesto'}
                  </button>
                )}
              </section>
            )}
          </aside>
        </div>
      )}
    </main>
  );
}

export default App;