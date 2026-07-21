import { invoke } from '@tauri-apps/api/core';
import type { GameInstall, GameManifest, GameUpdateProgress, GameUpdateResult, LaunchResult, RunnerInfo } from '../types/manifest';

export function listGames() {
  return invoke<GameManifest[]>('list_games');
}

export function listInstalls() {
  return invoke<GameInstall[]>('list_installs');
}

export function listRunners() {
  return invoke<RunnerInfo[]>('list_runners');
}

export function locateExistingInstall(gameId: string) {
  return invoke<GameInstall | null>('locate_existing_install', { gameId });
}

export function downloadAndRunInstaller(gameId: string) {
  return invoke<LaunchResult>('download_and_run_installer', { gameId });
}

export function runGameUpdate(gameId: string) {
  return invoke<LaunchResult>('run_game_update', { gameId });
}

export function runGameRemoteUpdate(gameId: string) {
  return invoke<GameUpdateResult>('run_game_remote_update', { gameId });
}

export function getGameUpdateProgress(gameId: string) {
  return invoke<GameUpdateProgress | null>('get_game_update_progress', { gameId });
}

export function openInstallFolder(gameId: string) {
  return invoke<void>('open_install_folder', { gameId });
}

export function removeInstall(gameId: string) {
  return invoke<boolean>('remove_install', { gameId });
}

export function launchGame(gameId: string) {
  return invoke<LaunchResult>('launch_game', { gameId });
}
