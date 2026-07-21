import { invoke } from '@tauri-apps/api/core';
import type { GameInstall, GameManifest } from '../types/manifest';

export function listGames() {
  return invoke<GameManifest[]>('list_games');
}

export function listInstalls() {
  return invoke<GameInstall[]>('list_installs');
}
