import { invoke } from '@tauri-apps/api/core';
import type { GameInstall, GameManifest, LaunchResult, RunnerInfo } from '../types/manifest';

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

export function openInstallFolder(gameId: string) {
  return invoke<void>('open_install_folder', { gameId });
}

export function removeInstall(gameId: string) {
  return invoke<boolean>('remove_install', { gameId });
}

export function launchGame(gameId: string) {
  return invoke<LaunchResult>('launch_game', { gameId });
}
