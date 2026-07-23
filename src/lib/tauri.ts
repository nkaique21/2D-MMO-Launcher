import { invoke } from '@tauri-apps/api/core';
import type { GameActivity, GameInstall, GameManifest, GameSettings, GameUpdateProgress, GameUpdateResult, InstallVerificationResult, LaunchResult, ManagedRunner, ManagedRunnerRelease, PlaytimeSession, RunnerInfo } from '../types/manifest';

export function listGames() {
  return invoke<GameManifest[]>('list_games');
}

export function listInstalls() {
  return invoke<GameInstall[]>('list_installs');
}

export function getGameActivity(gameId: string) {
  return invoke<GameActivity>('get_game_activity', { gameId });
}

export function listGamePlaytimeSessions(gameId: string) {
  return invoke<PlaytimeSession[]>('list_game_playtime_sessions', { gameId });
}

export function listRunners() {
  return invoke<RunnerInfo[]>('list_runners');
}

export function getLatestProtonGeRelease() {
  return invoke<ManagedRunnerRelease>('get_latest_proton_ge_release');
}

export function installLatestProtonGe() {
  return invoke<ManagedRunner>('install_latest_proton_ge');
}

export function removeManagedRunner(runnerId: string) {
  return invoke<boolean>('remove_managed_runner', { runnerId });
}

export function getGameSettings(gameId: string) {
  return invoke<GameSettings>('get_game_settings', { gameId });
}

export function saveGameSettings(gameId: string, runnerOverride: string | null, envOverrides: Record<string, string>) {
  return invoke<GameSettings>('save_game_settings', { gameId, runnerOverride, envOverrides });
}

export function resetGameSettings(gameId: string) {
  return invoke<GameSettings>('reset_game_settings', { gameId });
}

export function locateExistingInstall(gameId: string) {
  return invoke<GameInstall | null>('locate_existing_install', { gameId });
}

export function downloadAndRunInstaller(gameId: string) {
  return invoke<LaunchResult>('download_and_run_installer', { gameId });
}

export function downloadAndInstallArchive(gameId: string) {
  return invoke<LaunchResult>('download_and_install_archive', { gameId });
}

export function runGameUpdate(gameId: string) {
  return invoke<LaunchResult>('run_game_update', { gameId });
}

export function runGameRemoteUpdate(gameId: string) {
  return invoke<GameUpdateResult>('run_game_remote_update', { gameId });
}

export function installGameFromRemoteManifest(gameId: string) {
  return invoke<LaunchResult>('install_game_from_remote_manifest', { gameId });
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

export function verifyGameInstall(gameId: string) {
  return invoke<InstallVerificationResult>('verify_game_install', { gameId });
}

export function launchGame(gameId: string) {
  return invoke<LaunchResult>('launch_game', { gameId });
}
