import { invoke } from '@tauri-apps/api/core';
import type { GameManifest } from '../types/manifest';

export function listGames() {
  return invoke<GameManifest[]>('list_games');
}
