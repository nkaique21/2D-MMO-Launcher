import { useEffect, useMemo, useState } from 'react';
import { listGames } from './lib/tauri';
import type { GameManifest } from './types/manifest';

const cardAccents = [
  'from-violet-500 to-fuchsia-500',
  'from-purple-500 to-indigo-500',
  'from-sky-500 to-violet-500',
  'from-emerald-400 to-purple-500',
  'from-rose-500 to-violet-500',
  'from-amber-400 to-purple-600',
];

const navItems = ['Library', 'Downloads', 'Settings'];

function App() {
  const [games, setGames] = useState<GameManifest[]>([]);
  const [selectedGameId, setSelectedGameId] = useState<string | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);

  useEffect(() => {
    listGames()
      .then((loadedGames) => {
        setGames(loadedGames);
        setSelectedGameId(loadedGames[0]?.id ?? null);
        setCatalogError(null);
      })
      .catch((error) => {
        setCatalogError(error instanceof Error ? error.message : String(error));
      });
  }, []);

  const selectedGame = useMemo(
    () => games.find((game) => game.id === selectedGameId) ?? games[0],
    [games, selectedGameId],
  );

  return (
    <main className="min-h-screen bg-launcher-bg text-launcher-text">
      <div className="flex min-h-screen bg-[radial-gradient(circle_at_top_left,rgba(139,92,246,0.24),transparent_32rem),radial-gradient(circle_at_bottom_right,rgba(91,33,182,0.24),transparent_34rem)]">
        <aside className="flex w-72 flex-col border-r border-launcher-border bg-black/30 px-5 py-6 backdrop-blur-xl">
          <div className="mb-10 flex items-center gap-3">
            <div className="grid h-11 w-11 place-items-center rounded-2xl bg-purple-600 shadow-glow">
              <span className="text-lg font-black">2D</span>
            </div>
            <div>
              <p className="text-sm font-semibold uppercase tracking-[0.24em] text-purple-300">MMO</p>
              <h1 className="text-xl font-bold">Launcher</h1>
            </div>
          </div>

          <nav className="space-y-2">
            {navItems.map((item) => (
              <button
                className={`w-full rounded-2xl px-4 py-3 text-left text-sm font-semibold transition ${
                  item === 'Library'
                    ? 'bg-purple-600 text-white shadow-glow'
                    : 'text-launcher-muted hover:bg-white/5 hover:text-white'
                }`}
                key={item}
                type="button"
              >
                {item}
              </button>
            ))}
          </nav>

          <div className="mt-auto rounded-3xl border border-launcher-border bg-launcher-panelSoft p-4">
            <p className="text-xs font-bold uppercase tracking-[0.2em] text-purple-300">MVP</p>
            <p className="mt-2 text-sm text-launcher-muted">
              Etapa 2: shell visual, tema escuro e áreas base do launcher.
            </p>
          </div>
        </aside>

        <section className="flex flex-1 flex-col overflow-hidden">
          <header className="border-b border-launcher-border bg-launcher-bg/70 px-8 py-6 backdrop-blur-xl">
            <p className="text-sm font-semibold uppercase tracking-[0.24em] text-purple-300">
              Biblioteca
            </p>
            <div className="mt-2 flex items-end justify-between gap-6">
              <div>
                <h2 className="text-4xl font-black tracking-tight">Seus MMORPGs 2D</h2>
                <p className="mt-2 max-w-2xl text-launcher-muted">
                  Uma base extensível onde os jogos serão carregados por manifestos JSON, sem
                  lógica específica por jogo.
                </p>
              </div>
              <button
                className="rounded-2xl border border-purple-400/40 bg-purple-500/10 px-5 py-3 text-sm font-bold text-purple-100 transition hover:bg-purple-500/20"
                type="button"
              >
                {games.length} manifestos
              </button>
            </div>
          </header>

          <div className="grid flex-1 grid-cols-[1fr_380px] gap-6 overflow-auto p-8">
            <section>
              {catalogError ? (
                <div className="rounded-3xl border border-red-400/30 bg-red-950/30 p-6 text-red-100">
                  <p className="font-bold">Erro ao carregar catálogo</p>
                  <p className="mt-2 text-sm opacity-80">{catalogError}</p>
                </div>
              ) : null}

              <div className="grid grid-cols-2 gap-5">
                {games.map((game, index) => (
                  <article
                    className={`group overflow-hidden rounded-3xl border bg-launcher-panel shadow-2xl shadow-black/30 transition hover:-translate-y-1 hover:border-purple-400/60 ${
                      selectedGame?.id === game.id ? 'border-purple-400/70' : 'border-launcher-border'
                    }`}
                    key={game.id}
                    onClick={() => setSelectedGameId(game.id)}
                  >
                    <div className={`h-36 bg-gradient-to-br ${cardAccents[index % cardAccents.length]} opacity-90`} />
                    <div className="p-5">
                      <p className="text-xs font-bold uppercase tracking-[0.18em] text-purple-300">
                        MMORPG 2D
                      </p>
                      <h3 className="mt-2 text-2xl font-black">{game.name}</h3>
                      <p className="mt-2 line-clamp-2 text-sm text-launcher-muted">
                        {game.description}
                      </p>
                    </div>
                  </article>
                ))}
              </div>
            </section>

            <aside className="rounded-3xl border border-launcher-border bg-launcher-panel/90 p-6 shadow-2xl shadow-black/40">
              <div className="rounded-3xl bg-gradient-to-br from-purple-600 via-violet-700 to-slate-950 p-6 shadow-glow">
                <p className="text-xs font-bold uppercase tracking-[0.22em] text-purple-100/80">
                  Selecionado
                </p>
                <h3 className="mt-20 text-3xl font-black">{selectedGame?.name ?? 'Catálogo'}</h3>
                <p className="mt-2 text-sm text-purple-100/80">
                  {selectedGame?.description ?? 'Nenhum manifesto carregado.'}
                </p>
              </div>

              <div className="mt-6 space-y-3">
                <button className="w-full rounded-2xl bg-purple-600 px-5 py-4 font-bold shadow-glow transition hover:bg-purple-500" type="button">
                  Jogar
                </button>
                <button className="w-full rounded-2xl border border-launcher-border bg-white/5 px-5 py-4 font-bold text-launcher-muted transition hover:bg-white/10 hover:text-white" type="button">
                  Localizar instalação
                </button>
                <button className="w-full rounded-2xl border border-launcher-border bg-white/5 px-5 py-4 font-bold text-launcher-muted transition hover:bg-white/10 hover:text-white" type="button">
                  Configurações do jogo
                </button>
              </div>

              <div className="mt-6 grid grid-cols-2 gap-3 text-sm">
                <div className="rounded-2xl bg-white/5 p-4">
                  <p className="text-launcher-muted">Runner</p>
                  <p className="mt-1 font-bold capitalize">{selectedGame?.launch.runner ?? '-'}</p>
                </div>
                <div className="rounded-2xl bg-white/5 p-4">
                  <p className="text-launcher-muted">Update</p>
                  <p className="mt-1 font-bold capitalize">{selectedGame?.update.strategy ?? '-'}</p>
                </div>
              </div>
            </aside>
          </div>
        </section>
      </div>
    </main>
  );
}

export default App;
