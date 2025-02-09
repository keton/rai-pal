use std::{
	collections::HashSet,
	fs,
	path::PathBuf,
};

use async_trait::async_trait;
use lazy_regex::BytesRegex;
use steamlocate::SteamDir;

use super::{
	provider::{
		self,
		ProviderId,
	},
	provider_command::ProviderCommand,
};
use crate::{
	game_engines::game_engine::GameEngine,
	game_executable::OperatingSystem,
	game_mode::GameMode,
	installed_game::{
		self,
		InstalledGame,
	},
	owned_game::OwnedGame,
	pc_gaming_wiki,
	provider::{
		ProviderActions,
		ProviderStatic,
	},
	steam::{
		appinfo::{
			self,
			SteamAppInfoFile,
			SteamLaunchOption,
		},
		id_lists,
		thumbnail::get_steam_thumbnail,
	},
	Result,
};

pub struct Steam {
	steam_dir: SteamDir,
	app_info_file: SteamAppInfoFile,
	engine_cache: provider::EngineCache,
}

impl ProviderStatic for Steam {
	const ID: &'static ProviderId = &ProviderId::Steam;

	fn new() -> Result<Self>
	where
		Self: Sized,
	{
		let steam_dir = SteamDir::locate()?;
		let app_info_file = appinfo::read(steam_dir.path())?;
		let engine_cache = Self::try_get_engine_cache();

		Ok(Self {
			steam_dir,
			app_info_file,
			engine_cache,
		})
	}
}

#[async_trait]
impl ProviderActions for Steam {
	fn get_installed_games(&self) -> Result<Vec<InstalledGame>> {
		let mut games: Vec<InstalledGame> = Vec::new();
		let mut used_paths: HashSet<PathBuf> = HashSet::new();
		let mut used_names: HashSet<String> = HashSet::new();

		for library in (self.steam_dir.libraries()?).flatten() {
			for app in library.apps().flatten() {
				if let Some(app_info) = self.app_info_file.apps.get(&app.app_id) {
					let sorted_launch_options = {
						let mut sorted_launch_options = app_info.launch_options.clone();
						sorted_launch_options.sort_by(|a, b| a.launch_id.cmp(&b.launch_id));
						sorted_launch_options
					};

					for launch_option in sorted_launch_options {
						if let Some(executable_path) = launch_option.executable.as_ref() {
							let full_path = &app.path.join(executable_path);

							if used_paths.contains(full_path) {
								continue;
							}

							if let Some(name) = &app.name {
								if let Some(mut game) =
									installed_game::InstalledGame::new(full_path, name, *Self::ID)
								{
									let discriminator_option = if used_names.contains(name) {
										Some(launch_option.description.as_ref().map_or_else(
											|| executable_path.display().to_string(),
											Clone::clone,
										))
									} else {
										None
									};

									if let Some(discriminator) = &discriminator_option {
										game.set_discriminator(discriminator);
									}

									let app_id_string = app.app_id.to_string();

									game.set_provider_game_id(&app_id_string);
									game.set_thumbnail_url(&get_steam_thumbnail(&app_id_string));
									game.set_start_command_string(&get_start_command(
										&launch_option,
										&discriminator_option,
									));

									games.push(game);
									used_names.insert(name.clone());
									used_paths.insert(full_path.clone());
								}
							}
						}
					}
				}
			}
		}

		Ok(games)
	}

	async fn get_owned_games(&self) -> Result<Vec<OwnedGame>> {
		let steam_games = id_lists::get().await?;
		let owned_games = futures::future::join_all(self.app_info_file.apps.iter().map(
			|(steam_id, app_info)| async {
				let id_string = steam_id.to_string();
				let os_list: HashSet<_> = app_info
					.launch_options
					.iter()
					.filter_map(|launch| {
						launch
							.os_list
							.as_ref()
							.and_then(|os_list| match os_list.as_str() {
								"linux" => Some(OperatingSystem::Linux),
								"windows" => Some(OperatingSystem::Windows),
								_ => None,
							})
					})
					.collect();

				// Games in appinfo.vdf aren't necessarily owned.
				// Most of them are, but there are also a bunch of other games that Steam needs to reference for one reason or another.
				// assets.vdf is another cache file, and from my (not very extensive) tests, it to really only include owned files.
				// Free games are some times not there though, so I'm presuming that any free game found in appinfo.vdf is owned.
				// appinfo.vdf is also still needed since most of the game data we want is there, so we can't just read everything from assets.vdf.
				let owned = app_info.is_free
					|| fs::read(
						self.steam_dir
							.path()
							.join("appcache/librarycache/assets.vdf"),
					)
					.map_or(false, |assets_cache_bytes| {
						// Would be smarter to actually parse assets.vdf and extract all the ids,
						// but I didn't feel like figuring out how to parse another binary vdf.
						// Maybe later. But most likely never.
						BytesRegex::new(&id_string)
							.map_or(false, |regex| regex.is_match(&assets_cache_bytes))
					});

				if !owned {
					return None;
				}

				let game_mode = if app_info
					.launch_options
					.iter()
					.any(|launch| launch.get_game_mode() == GameMode::VR)
				{
					GameMode::VR
				} else {
					GameMode::Flat
				};

				// TODO: cache the whole thing, not just the engine version.
				let steam_game_option = steam_games.get(&id_string);

				let mut game = OwnedGame::new(&id_string, *Self::ID, &app_info.name);

				game.set_thumbnail_url(&get_steam_thumbnail(&id_string))
					.set_os_list(os_list)
					.set_game_mode(game_mode)
					.set_show_library_command(ProviderCommand::String(format!(
						"steam://nav/games/details/{id_string}"
					)))
					.set_open_page_command(ProviderCommand::String(format!(
						"steam://store/{id_string}"
					)))
					.set_install_command(ProviderCommand::String(format!(
						"steam://install/{id_string}"
					)));

				if let Some(release_date) = app_info
					.original_release_date
					.or(app_info.steam_release_date)
				{
					game.set_release_date(release_date.into());
				}

				if let Some(steam_game) = steam_game_option {
					if let Some(uevr_score) = steam_game.uevr_score {
						game.set_uevr_score(uevr_score);
					}

					game.set_engine(GameEngine {
						brand: steam_game.engine,
						version: get_engine(&id_string, &self.engine_cache)
							.await
							.and_then(|info| info.version),
					});
				}

				Some(game)
			},
		))
		.await
		.into_iter()
		.flatten();

		Self::try_save_engine_cache(
			&owned_games
				.clone()
				.map(|owned_game| (owned_game.id.clone(), owned_game.engine))
				.collect(),
		);

		Ok(owned_games.collect())
	}
}

async fn get_engine(steam_id: &str, cache: &provider::EngineCache) -> Option<GameEngine> {
	if let Some(cached_engine) = cache.get(steam_id) {
		return cached_engine.clone();
	}

	pc_gaming_wiki::get_engine(&format!("Steam_AppID%20HOLDS%20%22{steam_id}%22")).await
}

pub fn get_start_command(
	steam_launch: &SteamLaunchOption,
	discriminator: &Option<String>,
) -> String {
	if discriminator.is_none() {
		// If a game has no discriminator, it means we're probably using the default launch option.
		// For those, we use the steam://rungameid command, since that one will make steam show a nice
		// loading popup, wait for game updates, etc.

		format!("steam://rungameid/{}", steam_launch.app_id)
	} else {
		// For the few cases where we're showing an alternative launch option, we use the steam://launch command.
		// This one will show an error if the game needs an update, and doesn't show the nice loading popup,
		// but it allows us to specify the specific launch option to run.
		// This one also supports passing "dialog" instead of the app_type, (steam://launch/{app_id}/dialog)
		// which makes Steam show the launch selection dialog, but that dialog stops showing if the user
		// selects the "don't ask again" checkbox.
		format!(
			"steam://launch/{}/{}",
			steam_launch.app_id,
			steam_launch.launch_type.as_deref().unwrap_or(""),
		)
	}
}
