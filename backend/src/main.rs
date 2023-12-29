// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![feature(future_join)]

use std::{
	collections::HashMap,
	path::PathBuf,
	sync::Mutex,
};

use app_state::{
	AppState,
	DataValue,
	StateData,
	StatefulHandle,
};
use events::{
	AppEvent,
	EventEmitter,
};
use game_mod::get_common_data_map;
use installed_game::InstalledGame;
use maps::TryGettable;
use mod_loaders::mod_loader::{
	self,
	ModLoaderActions,
};
use paths::{
	hash_path,
	normalize_path,
};
use providers::{
	manual_provider,
	provider::{
		self,
		ProviderActions,
	},
};
use result::{
	Error,
	Result,
};
use steamlocate::SteamDir;
use tauri::{
	api::dialog::message,
	AppHandle,
	Manager,
};

mod analytics;
mod app_state;
mod events;
mod files;
mod game_engines;
mod game_executable;
mod game_mod;
mod game_mode;
mod installed_game;
mod local_mod;
mod macros;
mod maps;
mod mod_loaders;
mod owned_game;
mod paths;
mod providers;
mod remote_mod;
mod result;
mod steam;
mod windows;

#[tauri::command]
#[specta::specta]
async fn get_installed_games(handle: AppHandle) -> Result<installed_game::Map> {
	handle.app_state().installed_games.get_data()
}

#[tauri::command]
#[specta::specta]
async fn get_owned_games(handle: AppHandle) -> Result<owned_game::Map> {
	handle.app_state().owned_games.get_data()
}

#[tauri::command]
#[specta::specta]
async fn get_mod_loaders(handle: AppHandle) -> Result<mod_loader::DataMap> {
	mod_loader::get_data_map(&handle.app_state().mod_loaders.get_data()?)
}

#[tauri::command]
#[specta::specta]
async fn get_local_mods(handle: AppHandle) -> Result<local_mod::Map> {
	handle.app_state().local_mods.get_data()
}

#[tauri::command]
#[specta::specta]
async fn get_remote_mods(handle: AppHandle) -> Result<remote_mod::Map> {
	handle.app_state().remote_mods.get_data()
}

fn update_state<TData>(
	event: AppEvent,
	data: TData,
	mutex: &Mutex<Option<TData>>,
	handle: &AppHandle,
) {
	if let Ok(mut mutex_guard) = mutex.lock() {
		*mutex_guard = Some(data);
	}

	// Sends a signal to make the frontend request an app state refresh.
	// I would have preferred to just send the state with the signal,
	// but it seems like Tauri events are really slow for large data.
	handle.emit_event(event, ());
}

#[tauri::command]
#[specta::specta]
async fn open_game_folder(game_id: &str, handle: AppHandle) -> Result {
	handle
		.app_state()
		.installed_games
		.try_get(game_id)?
		.open_game_folder()
}

#[tauri::command]
#[specta::specta]
async fn open_game_mods_folder(game_id: &str, handle: AppHandle) -> Result {
	handle
		.app_state()
		.installed_games
		.try_get(game_id)?
		.open_mods_folder()
}

#[tauri::command]
#[specta::specta]
async fn open_mods_folder() -> Result {
	Ok(open::that_detached(paths::installed_mods_path()?)?)
}

#[tauri::command]
#[specta::specta]
async fn open_mod_folder(mod_id: &str, handle: AppHandle) -> Result {
	handle.app_state().local_mods.try_get(mod_id)?.open_folder()
}

#[tauri::command]
#[specta::specta]
async fn open_mod_loader_folder(mod_loader_id: &str, handle: AppHandle) -> Result {
	handle
		.app_state()
		.mod_loaders
		.try_get(mod_loader_id)?
		.open_folder()
}

#[tauri::command]
#[specta::specta]
async fn download_mod(mod_id: &str, handle: AppHandle) -> Result {
	let state = handle.app_state();
	let remote_mod = state.remote_mods.try_get(mod_id)?;
	let mod_loaders = state.mod_loaders.get_data()?;

	mod_loaders
		.try_get(&remote_mod.common.loader_id)?
		.download_mod(&remote_mod)
		.await?;

	refresh_local_mods(&mod_loaders, &handle).await;

	Ok(())
}

#[tauri::command]
#[specta::specta]
async fn start_game(game_id: &str, handle: AppHandle) -> Result {
	handle
		.app_state()
		.installed_games
		.try_get(game_id)?
		.start(&handle)
}

#[tauri::command]
#[specta::specta]
async fn start_game_exe(game_id: &str, handle: AppHandle) -> Result {
	handle
		.app_state()
		.installed_games
		.try_get(game_id)?
		.start_exe()
}

#[tauri::command]
#[specta::specta]
async fn install_mod(game_id: &str, mod_id: &str, handle: AppHandle) -> Result {
	let state = handle.app_state();

	let mut installed_games = state.installed_games.get_data()?;
	let game = installed_games.try_get_mut(game_id)?;
	let mod_loaders = state.mod_loaders.get_data()?;
	let remote_mod = state.remote_mods.try_get(mod_id)?;
	let mod_loader = mod_loaders.try_get(&remote_mod.common.loader_id)?;

	let local_mods = {
		let state_local_mods = state.local_mods.get_data()?;
		if state_local_mods.contains_key(mod_id) {
			state_local_mods
		} else {
			// Local mod wasn't in app state,
			// so let's sync app state to local files in case some file was manually changed.
			let disk_local_mods = refresh_local_mods(&mod_loaders, &handle).await;

			if state_local_mods.contains_key(mod_id) {
				disk_local_mods
			} else {
				if remote_mod.data.latest_version.is_some() {
					// If local mod still can't be found on disk,
					// we try to download it from the database.
					mod_loader
						.download_mod(&state.remote_mods.try_get(mod_id)?)
						.await?;
				} else {
					// If downloading from the database isn't possible,
					// we just open the mod loader folder so the user can install it themselves.
					mod_loader.open_folder()?;
				}

				refresh_local_mods(&mod_loaders, &handle).await
			}
		}
	};

	mod_loader
		.install_mod(game, local_mods.try_get(mod_id)?)
		.await?;

	refresh_game_mods_and_exe(&game.id, &handle)?;

	analytics::send_event(analytics::Event::InstallOrRunMod, mod_id).await;

	Ok(())
}

fn refresh_game_mods_and_exe(game_id: &str, handle: &AppHandle) -> Result {
	let state = handle.app_state();

	let mod_data_map = game_mod::get_common_data_map(
		&handle.app_state().local_mods.get_data()?,
		&handle.app_state().remote_mods.get_data()?,
	);

	let mut installed_games = state.installed_games.get_data()?;

	let game = installed_games.try_get_mut(game_id)?;

	game.refresh_mods(&mod_data_map);
	game.refresh_executable();

	update_state(
		AppEvent::SyncInstalledGames,
		installed_games,
		&handle.app_state().installed_games,
		handle,
	);

	Ok(())
}

#[tauri::command]
#[specta::specta]
async fn refresh_game(game_id: &str, handle: AppHandle) -> Result {
	refresh_game_mods_and_exe(game_id, &handle)
}

#[tauri::command]
#[specta::specta]
async fn uninstall_mod(game_id: &str, mod_id: &str, handle: AppHandle) -> Result {
	let state = handle.app_state();
	let mut installed_games = state.installed_games.get_data()?;
	let game = installed_games.try_get_mut(game_id)?;
	game.uninstall_mod(mod_id)?;

	refresh_game_mods_and_exe(&game.id, &handle)?;

	Ok(())
}

async fn refresh_local_mods(mod_loaders: &mod_loader::Map, handle: &AppHandle) -> local_mod::Map {
	let local_mods: HashMap<_, _> = mod_loaders
		.values()
		.filter_map(|mod_loader| {
			mod_loader.get_local_mods().ok() // don't swallow error.
		})
		.flatten()
		.collect();

	update_state(
		AppEvent::SyncLocalMods,
		local_mods.clone(),
		&handle.app_state().local_mods,
		handle,
	);

	local_mods
}

async fn refresh_remote_mods(mod_loaders: &mod_loader::Map, handle: &AppHandle) -> remote_mod::Map {
	let mut remote_mods = remote_mod::Map::default();

	for mod_loader in mod_loaders.values() {
		for (mod_id, remote_mod) in mod_loader
			.get_remote_mods(|error| {
				handle.emit_error(format!("Failed to get remote mods: {error}"));
			})
			.await
		{
			remote_mods.insert(mod_id.clone(), remote_mod.clone());
		}
	}

	update_state(
		AppEvent::SyncRemoteMods,
		remote_mods.clone(),
		&handle.app_state().remote_mods,
		handle,
	);

	remote_mods
}

#[tauri::command]
#[specta::specta]
async fn update_data(handle: AppHandle) -> Result {
	let resources_path = paths::resources_path(&handle)?;

	let mod_loaders = mod_loader::get_map(&resources_path).await;
	update_state(
		AppEvent::SyncModLoaders,
		mod_loaders.clone(),
		&handle.app_state().mod_loaders,
		&handle,
	);

	let local_mods = refresh_local_mods(&mod_loaders, &handle).await;

	let provider_map = provider::get_map(|error| {
		handle.emit_error(format!("Failed to set up provider: {error}"));
	});

	let mut installed_games: HashMap<_, _> = provider_map
		.values()
		.flat_map(|provider| match provider.get_installed_games() {
			Ok(games) => games,
			Err(err) => {
				handle.emit_error(format!("Error getting installed games for provider: {err}"));
				Vec::default()
			}
		})
		.map(|mut game| {
			game.update_available_mods(&get_common_data_map(&local_mods, &HashMap::default()));
			(game.id.clone(), game)
		})
		.collect();

	update_state(
		AppEvent::SyncInstalledGames,
		installed_games.clone(),
		&handle.app_state().installed_games,
		&handle,
	);

	let remote_mods = refresh_remote_mods(&mod_loaders, &handle).await;

	for game in installed_games.values_mut() {
		game.update_available_mods(&get_common_data_map(&local_mods, &remote_mods));
	}

	update_state(
		AppEvent::SyncInstalledGames,
		installed_games.clone(),
		&handle.app_state().installed_games,
		&handle,
	);

	let owned_games: owned_game::Map = futures::future::join_all(
		provider_map
			.values()
			.map(provider::ProviderActions::get_owned_games),
	)
	.await
	.into_iter()
	.flat_map(result::Result::unwrap_or_default)
	.map(|owned_game| (owned_game.id.clone(), owned_game))
	.collect();

	update_state(
		AppEvent::SyncOwnedGames,
		owned_games,
		&handle.app_state().owned_games,
		&handle,
	);

	Ok(())
}

#[tauri::command]
#[specta::specta]
async fn add_game(path: PathBuf, handle: AppHandle) -> Result {
	let state = handle.app_state();
	let normalized_path = normalize_path(&path);
	let game_id = hash_path(&normalized_path);

	if state.installed_games.try_get(&game_id).is_ok() {
		return Err(Error::GameAlreadyAdded(normalized_path));
	}

	let mut game = manual_provider::add_game(&normalized_path)?;
	game.update_available_mods(&get_common_data_map(
		&state.local_mods.get_data()?,
		&state.remote_mods.get_data()?,
	));
	let game_name = game.name.clone();

	let mut installed_games = state.installed_games.get_data()?.clone();
	installed_games.insert(game.id.clone(), game);

	update_state(
		AppEvent::SyncInstalledGames,
		installed_games,
		&state.installed_games,
		&handle,
	);

	handle.emit_event(AppEvent::GameAdded, game_name.clone());

	analytics::send_event(analytics::Event::ManuallyAddGame, &game_name).await;

	Ok(())
}

#[tauri::command]
#[specta::specta]
async fn remove_game(game_id: &str, handle: AppHandle) -> Result {
	let state = handle.app_state();
	let game = state.installed_games.try_get(game_id)?;
	manual_provider::remove_game(&game.executable.path)?;

	let mut installed_games = state.installed_games.get_data()?;
	installed_games.remove(game_id);

	update_state(
		AppEvent::SyncInstalledGames,
		installed_games,
		&handle.app_state().installed_games,
		&handle,
	);

	handle.emit_event(AppEvent::GameRemoved, game.name);

	Ok(())
}

#[tauri::command]
#[specta::specta]
async fn delete_steam_appinfo_cache() -> Result {
	let steam_dir = SteamDir::locate()?;
	steam::appinfo::delete(steam_dir.path())
}

#[tauri::command]
#[specta::specta]
async fn frontend_ready() -> Result {
	analytics::send_event(analytics::Event::StartApp, "").await;

	Ok(())
}

#[tauri::command]
#[specta::specta]
async fn get_other_games() -> Result<HashMap<String, Vec<String>>> {
	let games = HashMap::from([
		(
			"Epic".to_string(),
			game_scanner::epicgames::games()
				.unwrap_or_default()
				.iter()
				.map(|game| {
					format!(
						"{}: {}. {} ({})",
						game.id,
						game.name,
						if game.state.installed {
							"installed"
						} else {
							"owned"
						},
						game.path.clone().unwrap_or_default().to_string_lossy()
					)
				})
				.collect(),
		),
		(
			"GOG".to_string(),
			game_scanner::gog::games()
				.unwrap_or_default()
				.iter()
				.map(|game| {
					format!(
						"{}: {}. {} ({})",
						game.id,
						game.name,
						if game.state.installed {
							"installed"
						} else {
							"owned"
						},
						game.path.clone().unwrap_or_default().to_string_lossy()
					)
				})
				.collect(),
		),
		(
			"Origin".to_string(),
			game_scanner::origin::games()
				.unwrap_or_default()
				.iter()
				.map(|game| {
					format!(
						"{}: {}. {} ({})",
						game.id,
						game.name,
						if game.state.installed {
							"installed"
						} else {
							"owned"
						},
						game.path.clone().unwrap_or_default().to_string_lossy()
					)
				})
				.collect(),
		),
		(
			"Ubisoft".to_string(),
			game_scanner::ubisoft::games()
				.unwrap_or_default()
				.iter()
				.map(|game| {
					format!(
						"{}: {}. {} ({})",
						game.id,
						game.name,
						if game.state.installed {
							"installed"
						} else {
							"owned"
						},
						game.path.clone().unwrap_or_default().to_string_lossy()
					)
				})
				.collect(),
		),
		(
			"Amazon".to_string(),
			game_scanner::amazon::games()
				.unwrap_or_default()
				.iter()
				.map(|game| {
					format!(
						"{}: {}. {} ({})",
						game.id,
						game.name,
						if game.state.installed {
							"installed"
						} else {
							"owned"
						},
						game.path.clone().unwrap_or_default().to_string_lossy()
					)
				})
				.collect(),
		),
		(
			"Steam".to_string(),
			game_scanner::steam::games()
				.unwrap_or_default()
				.iter()
				.map(|game| {
					format!(
						"{}: {}. {} ({})",
						game.id,
						game.name,
						if game.state.installed {
							"installed"
						} else {
							"owned"
						},
						game.path.clone().unwrap_or_default().to_string_lossy()
					)
				})
				.collect(),
		),
	]);

	Ok(games)
}

#[tauri::command]
#[specta::specta]
async fn dummy_command() -> Result<(InstalledGame, AppEvent)> {
	// This command is here just so tauri_specta exports these types.
	// This should stop being needed once tauri_specta starts supporting events.
	Err(Error::NotImplemented)
}

fn main() {
	// Since I'm making all exposed functions async, panics won't crash anything important, I think.
	// So I can just catch panics here and show a system message with the error.
	std::panic::set_hook(Box::new(|info| {
		eprintln!("Panic: {info}");
		message(
			None::<&tauri::Window>,
			"Failed to execute command",
			info.to_string(),
		);
	}));

	let tauri_builder = tauri::Builder::default()
		.plugin(tauri_plugin_window_state::Builder::default().build())
		.manage(AppState {
			installed_games: Mutex::default(),
			owned_games: Mutex::default(),
			mod_loaders: Mutex::default(),
			local_mods: Mutex::default(),
			remote_mods: Mutex::default(),
		})
		.setup(|app| {
			// This prevents/reduces the white flashbang on app start.
			// Unfortunately, it will still show the default window color for the system for a bit,
			// which can some times be white.
			if let Some(window) = app.get_window("main") {
				window.set_title(&format!("Rai Pal {}", env!("CARGO_PKG_VERSION")))?;

				#[cfg(target_os = "linux")]
				{
					window.with_webview(|webview| {
						use webkit2gtk::traits::WebViewExt;
						let mut color = webview.inner().background_color();
						color.set_red(0.102);
						color.set_green(0.106);
						color.set_blue(0.118);
						webview.inner().set_background_color(&color);
					})?;
				}
			}

			Ok(())
		});

	let (tauri_builder, types_result) = set_up_api!(
		tauri_builder,
		[
			dummy_command,
			update_data,
			get_installed_games,
			get_owned_games,
			get_mod_loaders,
			open_game_folder,
			install_mod,
			uninstall_mod,
			open_game_mods_folder,
			start_game,
			start_game_exe,
			open_mod_folder,
			download_mod,
			open_mods_folder,
			add_game,
			remove_game,
			delete_steam_appinfo_cache,
			frontend_ready,
			get_local_mods,
			get_remote_mods,
			open_mod_loader_folder,
			refresh_game,
			get_other_games,
		]
	);

	match types_result {
		Ok(types) => {
			#[cfg(debug_assertions)]
			if let Err(err) = tauri_specta::ts::export_with_cfg(
				types,
				specta::ts::ExportConfiguration::default()
					.bigint(specta::ts::BigIntExportBehavior::BigInt),
				"../frontend/api/bindings.ts",
			) {
				eprintln!("Failed to generate TypeScript bindings: {err}");
			}
		}
		Err(err) => {
			eprintln!("Failed to generate api bindings: {err}");
		}
	}
	tauri_builder
		.run(tauri::generate_context!())
		.unwrap_or_else(|err| eprintln!("Failed to run Tauri application: {err}"));
}
