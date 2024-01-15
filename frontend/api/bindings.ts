/* eslint-disable */
// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

declare global {
    interface Window {
        __TAURI_INVOKE__<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
    }
}

// Function avoids 'window not defined' in SSR
const invoke = () => window.__TAURI_INVOKE__;

export function dummyCommand() {
    return invoke()<[InstalledGame, AppEvent]>("dummy_command")
}

export function updateData() {
    return invoke()<null>("update_data")
}

export function getInstalledGames() {
    return invoke()<{ [key: string]: InstalledGame }>("get_installed_games")
}

export function getOwnedGames() {
    return invoke()<{ [key: string]: OwnedGame }>("get_owned_games")
}

export function getModLoaders() {
    return invoke()<{ [key: string]: ModLoaderData }>("get_mod_loaders")
}

export function openGameFolder(gameId: string) {
    return invoke()<null>("open_game_folder", { gameId })
}

export function installMod(gameId: string, modId: string) {
    return invoke()<null>("install_mod", { gameId,modId })
}

export function uninstallMod(gameId: string, modId: string) {
    return invoke()<null>("uninstall_mod", { gameId,modId })
}

export function openGameModsFolder(gameId: string) {
    return invoke()<null>("open_game_mods_folder", { gameId })
}

export function startGame(gameId: string) {
    return invoke()<null>("start_game", { gameId })
}

export function startGameExe(gameId: string) {
    return invoke()<null>("start_game_exe", { gameId })
}

export function openModFolder(modId: string) {
    return invoke()<null>("open_mod_folder", { modId })
}

export function downloadMod(modId: string) {
    return invoke()<null>("download_mod", { modId })
}

export function openModsFolder() {
    return invoke()<null>("open_mods_folder")
}

export function addGame(path: string) {
    return invoke()<null>("add_game", { path })
}

export function removeGame(gameId: string) {
    return invoke()<null>("remove_game", { gameId })
}

export function deleteSteamAppinfoCache() {
    return invoke()<null>("delete_steam_appinfo_cache")
}

export function frontendReady() {
    return invoke()<null>("frontend_ready")
}

export function getLocalMods() {
    return invoke()<{ [key: string]: LocalMod }>("get_local_mods")
}

export function getRemoteMods() {
    return invoke()<{ [key: string]: RemoteMod }>("get_remote_mods")
}

export function openModLoaderFolder(modLoaderId: string) {
    return invoke()<null>("open_mod_loader_folder", { modLoaderId })
}

export function refreshGame(gameId: string) {
    return invoke()<null>("refresh_game", { gameId })
}

export function openLogsFolder() {
    return invoke()<null>("open_logs_folder")
}

export function showGameInLibrary(ownedGameId: string) {
    return invoke()<null>("show_game_in_library", { ownedGameId })
}

export function installGame(ownedGameId: string) {
    return invoke()<null>("install_game", { ownedGameId })
}

export function openGamePage(ownedGameId: string) {
    return invoke()<null>("open_game_page", { ownedGameId })
}

export type GameEngineVersion = { major: number; minor: number; patch: number; suffix: string | null; display: string }
export type Manifest = { version: string; runnable: RunnableModData | null; engine: GameEngineBrand | null; unityBackend: UnityScriptingBackend | null }
export type ProviderId = "Steam" | "Manual" | "Epic" | "Gog" | "Xbox"
export type GameEngineBrand = "Unity" | "Unreal" | "Godot"
export type GameMode = "VR" | "Flat"
export type ModKind = "Installable" | "Runnable"
export type RunnableModData = { path: string; args: string[] }
export type AppEvent = "SyncInstalledGames" | "SyncOwnedGames" | "SyncModLoaders" | "SyncLocalMods" | "SyncRemoteMods" | "ExecutedSteamCommand" | "GameAdded" | "GameRemoved" | "Error"
export type ModDownload = { id: string; url: string; root: string | null; runnable: RunnableModData | null }
export type OwnedGame = { id: string; provider: ProviderId; providerGameId: string; name: string; osList: OperatingSystem[]; engine: GameEngine | null; releaseDate: BigInt; thumbnailUrl: string; gameMode: GameMode | null; uevrScore: UevrScore | null; showLibraryCommand: ProviderCommand | null; openPageCommand: ProviderCommand | null; installCommand: ProviderCommand | null }
export type LocalMod = { data: LocalModData; common: CommonModData }
export type RemoteModData = { title: string; author: string; sourceCode: string; description: string; latestVersion: ModDownload | null }
export type ProviderCommand = { String: string } | { Path: [string, string[]] }
export type LocalModData = { path: string; manifest: Manifest | null }
export type ModLoaderData = { id: string; path: string; kind: ModKind }
export type UnityScriptingBackend = "Il2Cpp" | "Mono"
export type InstalledGame = { id: string; name: string; provider: ProviderId; executable: GameExecutable; installedModVersions: { [key: string]: string | null }; discriminator: string | null; thumbnailUrl: string | null; gameMode: GameMode | null; providerGameId: string | null; startCommand: ProviderCommand | null }
export type GameExecutable = { path: string; name: string; engine: GameEngine | null; architecture: Architecture | null; operatingSystem: OperatingSystem | null; scriptingBackend: UnityScriptingBackend | null }
export type RemoteMod = { common: CommonModData; data: RemoteModData }
export type UevrScore = "A" | "B" | "C" | "D" | "E"
export type GameEngine = { brand: GameEngineBrand; version: GameEngineVersion | null }
export type OperatingSystem = "Linux" | "Windows"
export type CommonModData = { id: string; engine: GameEngineBrand | null; unityBackend: UnityScriptingBackend | null; loaderId: string }
export type Architecture = "X64" | "X86"
