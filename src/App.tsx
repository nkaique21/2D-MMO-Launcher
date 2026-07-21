import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  listGames,
  listInstalls,
  listRunners,
  downloadAndRunInstaller,
  launchGame,
  locateExistingInstall,
  openInstallFolder,
  removeInstall,
  runGameUpdate,
  runGameRemoteUpdate,
} from './lib/tauri';
import type { GameInstall, GameManifest, GameUpdateProgress, RunnerInfo } from './types/manifest';

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
  zezenia: {
    shortName: 'ZZ',
    accent: 'from-emerald-400 via-teal-500 to-purple-600',
    softAccent: 'bg-emerald-500/15 text-emerald-100 ring-emerald-300/20',
    meta: 'Classic • Persistent world',
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
  const [selectedGameId, setSelectedGameId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);
  const [pendingActionId, setPendingActionId] = useState<string | null>(null);
  const [updateProgress, setUpdateProgress] = useState<GameUpdateProgress | null>(null);
  const [isLaunching, setIsLaunching] = useState(false);
  const [reloadSignal, setReloadSignal] = useState(0);

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

    const unlistenPromise = listen<GameUpdateProgress>('game-update-progress', (event) => {
      if (!isMounted) return;

      setUpdateProgress(event.payload);
    });

    return () => {
      isMounted = false;
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

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

  async function handlePrimaryAction() {
    setActionError(null);
    setActionMessage(null);

    if (selectedGame.status !== 'installed') {
      const windowsInstallerMethod = selectedGame.installation.methods.find(
        (method) => method.type === 'windowsInstaller',
      );

      if (!windowsInstallerMethod) {
        setActionMessage('Use “Localizar instalação existente” para registrar este jogo antes de jogar.');
        return;
      }

      setPendingActionId('primary-install');
      setActionMessage('Baixando instalador e preparando runner...');

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

    setIsLaunching(true);

    try {
      const result = await launchGame(selectedGame.id);
      setActionMessage(formatLaunchMessage('Jogo iniciado', result));
    } catch (error) {
      setActionError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsLaunching(false);
    }
  }

  async function handleSecondaryAction(action: SecondaryAction) {
    setActionError(null);
    setActionMessage(null);

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
      setPendingActionId(action.id);
      setUpdateProgress(null);
      setActionMessage('Preparando atualização dos arquivos...');

      try {
        const result = await runGameRemoteUpdate(selectedGame.id);
        const logMessage = result.logPath ? ` Log: ${result.logPath}` : '';

        setActionMessage(
          `Update concluído: ${result.updatedFiles} arquivo(s) baixado(s), ${result.skippedFiles} já estavam atualizados, ${formatBytes(result.downloadedBytes)} transferidos.${logMessage}`,
        );
      } catch (error) {
        setActionError(error instanceof Error ? error.message : String(error));
      } finally {
        setPendingActionId(null);
      }

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
    <main className="min-h-screen overflow-hidden bg-launcher-bg text-launcher-text">
      <div className="flex min-h-screen bg-[radial-gradient(circle_at_10%_0%,rgba(139,92,246,0.22),transparent_28rem),radial-gradient(circle_at_82%_18%,rgba(14,165,233,0.12),transparent_24rem),linear-gradient(135deg,rgba(255,255,255,0.03),transparent_42%)]">
        <aside className="flex w-[104px] flex-col items-center border-r border-white/10 bg-black/35 px-4 py-5 backdrop-blur-2xl">
          <div className="grid h-14 w-14 place-items-center rounded-[1.35rem] bg-white/10 ring-1 ring-white/15 shadow-glow">
            <span className="bg-gradient-to-br from-white to-purple-200 bg-clip-text text-lg font-black text-transparent">
              2D
            </span>
          </div>

          <div className="mt-9 flex w-full flex-1 flex-col items-center gap-3">
            <p className="mb-1 [writing-mode:vertical-rl] rotate-180 text-[0.63rem] font-black uppercase tracking-[0.28em] text-launcher-muted">
              Instalados
            </p>
            {installedGames.length > 0 ? (
              installedGames.map((game) => {
                const isActive = game.id === selectedGame.id;

                return (
                  <button
                    aria-label={game.name}
                    className={`group relative grid h-16 w-16 place-items-center rounded-3xl border transition duration-200 ${
                      isActive
                        ? 'border-purple-300/70 bg-white/15 shadow-glow'
                        : 'border-white/10 bg-white/[0.055] hover:border-white/25 hover:bg-white/10'
                    }`}
                    key={game.id}
                    onClick={() => setSelectedGameId(game.id)}
                    title={game.name}
                    type="button"
                  >
                    <span className={`absolute inset-2 rounded-[1.15rem] bg-gradient-to-br ${game.accent} opacity-80 blur-[1px]`} />
                    <span className="relative text-sm font-black tracking-tight text-white drop-shadow">
                      {game.shortName}
                    </span>
                    {isActive && <span className="absolute -right-5 h-9 w-1 rounded-full bg-purple-300" />}
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
            className="grid h-12 w-12 place-items-center rounded-2xl border border-white/10 bg-white/[0.055] text-xl text-launcher-muted transition hover:border-purple-300/40 hover:text-white"
            type="button"
          >
            +
          </button>
        </aside>

        <section className="flex min-w-0 flex-1 flex-col">
          <header className="border-b border-white/10 bg-launcher-bg/55 px-8 pb-6 pt-5 backdrop-blur-2xl">
            <div className="flex items-center justify-between gap-6">
              <div>
                <p className="text-xs font-black uppercase tracking-[0.28em] text-purple-300">
                  Catálogo por manifesto
                </p>
                <h1 className="mt-1 text-2xl font-black tracking-tight">2D MMO Launcher</h1>
              </div>
              <div className="flex items-center gap-3 rounded-full border border-white/10 bg-white/[0.045] px-4 py-2 text-xs font-semibold text-launcher-muted">
                <span className={`h-2 w-2 rounded-full ${loadError ? 'bg-amber-300 shadow-[0_0_18px_rgba(252,211,77,0.75)]' : 'bg-emerald-400 shadow-[0_0_18px_rgba(52,211,153,0.75)]'}`} />
                {loadError ? 'Catálogo local em modo degradado' : 'Manifestos locais carregados'}
              </div>
            </div>

            <div className="mt-6 flex gap-3 overflow-x-auto pb-1">
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

          <div className="grid flex-1 grid-cols-[minmax(0,1fr)_320px] gap-6 overflow-auto p-8">
            <section className="min-w-0">
              <article className="relative min-h-[520px] overflow-hidden rounded-[2rem] border border-white/10 bg-launcher-panel shadow-2xl shadow-black/40">
                <div
                  className="absolute inset-0 bg-cover bg-center opacity-70"
                  style={{
                    backgroundImage: `linear-gradient(90deg, rgba(7,7,16,0.92) 0%, rgba(7,7,16,0.58) 46%, rgba(7,7,16,0.26) 100%), url(${selectedGame.assets.banner})`,
                  }}
                />
                <div className={`absolute inset-0 bg-gradient-to-br ${selectedGame.accent} opacity-20`} />
                <div className="absolute inset-x-0 bottom-0 h-48 bg-gradient-to-t from-launcher-bg via-launcher-bg/70 to-transparent" />

                <div className="relative flex min-h-[520px] max-w-3xl flex-col justify-end p-8 lg:p-10">
                  <div className="mb-auto flex flex-wrap items-center gap-3">
                    <span className="rounded-full border border-white/[0.12] bg-black/30 px-3 py-1.5 text-xs font-black uppercase tracking-[0.2em] text-white/85 backdrop-blur-md">
                      {selectedGame.status === 'installed' ? 'Na sua biblioteca' : 'Disponível para instalar'}
                    </span>
                    {selectedGame.protonOnly && (
                      <span className="rounded-full border border-purple-200/20 bg-purple-500/[0.18] px-3 py-1.5 text-xs font-black uppercase tracking-[0.2em] text-purple-100 backdrop-blur-md">
                        Proton obrigatório
                      </span>
                    )}
                  </div>

                  <p className="text-sm font-bold uppercase tracking-[0.24em] text-purple-200/90">
                    {selectedGame.meta}
                  </p>
                  <h2 className="mt-3 max-w-2xl text-6xl font-black leading-[0.92] tracking-[-0.06em] text-white">
                    {selectedGame.name}
                  </h2>
                  <p className="mt-5 max-w-xl text-base leading-7 text-launcher-muted">
                    {selectedGame.description}
                  </p>

                  <div className="mt-8 flex flex-wrap items-center gap-3">
                    <button
                      className="rounded-2xl bg-white px-8 py-4 text-sm font-black uppercase tracking-[0.16em] text-slate-950 shadow-[0_18px_60px_rgba(255,255,255,0.16)] transition hover:-translate-y-0.5 hover:bg-purple-100"
                      disabled={isLaunching || pendingActionId === 'primary-install'}
                      onClick={() => void handlePrimaryAction()}
                      type="button"
                    >
                      {isLaunching
                        ? 'Iniciando...'
                        : pendingActionId === 'primary-install'
                          ? 'Baixando...'
                          : selectedGame.status === 'installed'
                            ? 'Jogar'
                            : 'Baixar e instalar'}
                    </button>
                    <button
                      className="rounded-2xl border border-white/[0.12] bg-white/[0.07] px-5 py-4 text-sm font-bold text-white/78 backdrop-blur-md transition hover:border-white/25 hover:bg-white/[0.12] hover:text-white"
                      type="button"
                    >
                      Ver detalhes
                    </button>
                  </div>
                </div>
              </article>
            </section>

            <aside className="space-y-4">
              <section className="rounded-[1.75rem] border border-white/10 bg-white/[0.055] p-5 backdrop-blur-2xl">
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

                <div className="mt-5 grid grid-cols-2 gap-3 text-sm">
                  <div className="rounded-2xl bg-black/20 p-4 ring-1 ring-white/[0.08]">
                    <p className="text-xs text-launcher-muted">Runner</p>
                    <p className="mt-1 font-black">{selectedGame.runnerLabel}</p>
                  </div>
                  <div className="rounded-2xl bg-black/20 p-4 ring-1 ring-white/[0.08]">
                    <p className="text-xs text-launcher-muted">Estado</p>
                    <p className="mt-1 font-black">{selectedGame.status === 'installed' ? 'Instalado' : 'Catálogo'}</p>
                  </div>
                </div>

                {selectedInstall && (
                  <div className="mt-3 rounded-2xl bg-black/20 p-4 text-sm ring-1 ring-white/[0.08]">
                    <p className="text-xs text-launcher-muted">Caminho da instalação</p>
                    <p className="mt-2 break-all font-semibold leading-6 text-white/85">
                      {selectedInstall.installPath}
                    </p>
                  </div>
                )}
              </section>

              <section className="rounded-[1.75rem] border border-white/10 bg-launcher-panel/80 p-3 shadow-2xl shadow-black/30">
                {secondaryActions.map((action) => (
                  <button
                    className="flex w-full items-center justify-between rounded-2xl px-4 py-3 text-left text-sm font-semibold text-launcher-muted transition hover:bg-white/[0.065] hover:text-white"
                    disabled={pendingActionId === action.id}
                    key={action.id}
                    onClick={() => void handleSecondaryAction(action)}
                    type="button"
                  >
                    {pendingActionId === action.id ? 'Aguardando seleção...' : action.label}
                    <span className="text-white/25">›</span>
                  </button>
                ))}
              </section>

              {updateProgress && updateProgress.gameId === selectedGame.id && (
                <section className="rounded-[1.75rem] border border-sky-300/20 bg-sky-500/[0.08] p-5 text-sm text-sky-100">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="text-xs font-black uppercase tracking-[0.2em] text-sky-200">
                        Atualização
                      </p>
                      <p className="mt-1 font-black">{updateProgress.message}</p>
                    </div>
                    <span className="rounded-full bg-black/20 px-3 py-1 text-xs font-black uppercase tracking-[0.14em] ring-1 ring-white/10">
                      {updateProgress.status}
                    </span>
                  </div>
                  <div className="mt-4 h-2 overflow-hidden rounded-full bg-black/25 ring-1 ring-white/[0.08]">
                    <div
                      className="h-full rounded-full bg-sky-300 transition-all"
                      style={{
                        width: `${updateProgress.totalFiles > 0
                          ? Math.min(100, Math.round((updateProgress.checkedFiles / updateProgress.totalFiles) * 100))
                          : 8}%`,
                      }}
                    />
                  </div>
                  <p className="mt-3 text-xs leading-5 text-sky-100/80">
                    {updateProgress.checkedFiles}/{updateProgress.totalFiles} verificados • {updateProgress.updatedFiles} baixados
                    {updateProgress.currentFile ? ` • ${updateProgress.currentFile}` : ''}
                  </p>
                </section>
              )}

              <section className="rounded-[1.75rem] border border-white/10 bg-white/[0.055] p-5 backdrop-blur-2xl">
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
                <section className={`rounded-[1.75rem] border p-4 text-sm leading-6 ${
                  actionError
                    ? 'border-red-300/20 bg-red-500/[0.08] text-red-100'
                    : 'border-emerald-300/20 bg-emerald-500/[0.08] text-emerald-100'
                }`}
                >
                  {actionError ?? actionMessage}
                </section>
              )}

              <section className="rounded-[1.75rem] border border-purple-300/15 bg-purple-500/[0.065] p-5">
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
    </main>
  );
}

export default App;