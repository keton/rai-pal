use std::{
	collections::{
		HashMap,
		HashSet,
	},
	fs,
	path::PathBuf,
	string,
};

use async_trait::async_trait;
use lazy_regex::BytesRegex;
use steamlocate::SteamDir;

use super::provider::ProviderId;
use crate::{
	game_executable::OperatingSystem,
	installed_game,
	mod_loaders::mod_loader,
	owned_game::OwnedGame,
	provider::{
		ProviderActions,
		ProviderStatic,
	},
	steam::{
		appinfo::{
			self,
			SteamAppInfoFile,
		},
		id_lists,
		thumbnail::get_steam_thumbnail,
	},
	Result,
};

pub struct SteamProvider {
	steam_dir: SteamDir,
	app_info_file: SteamAppInfoFile,
}

impl ProviderStatic for SteamProvider {
	const ID: &'static ProviderId = &ProviderId::Steam;

	fn new() -> Result<Self>
	where
		Self: Sized,
	{
		let steam_dir = SteamDir::locate()?;
		let app_info_file = appinfo::read(steam_dir.path())?;

		Ok(Self {
			steam_dir,
			app_info_file,
		})
	}
}

#[async_trait]
impl ProviderActions for SteamProvider {
	fn get_installed_games(
		&self,
		mod_loaders: &mod_loader::DataMap,
	) -> Result<installed_game::Map> {
		let mut game_map: installed_game::Map = HashMap::new();
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
						let executable_id =
							format!("{}_{}", launch_option.app_id, launch_option.launch_id);

						if let Some(executable_path) = launch_option.executable.as_ref() {
							let full_path = &app.path.join(executable_path);

							if used_paths.contains(full_path) {
								continue;
							}

							if let Some(name) = &app.name {
								let discriminator = if used_names.contains(name) {
									launch_option.description.as_ref().map_or_else(
										|| {
											executable_path
												.to_str()
												.map(string::ToString::to_string)
										},
										|description| Some(description.clone()),
									)
								} else {
									None
								};

								if let Some(game) = installed_game::InstalledGame::new(
									&executable_id,
									name,
									Self::ID,
									discriminator,
									full_path,
									Some(&launch_option),
									Some(get_steam_thumbnail(&app.app_id.to_string())),
									mod_loaders,
								) {
									game_map.insert(executable_id.clone(), game);
									used_names.insert(name.clone());
									used_paths.insert(full_path.clone());
								}
							}
						}
					}
				}
			}
		}

		Ok(game_map)
	}

	async fn get_owned_games(&self) -> Result<Vec<OwnedGame>> {
		Ok(id_lists::get()
			.await?
			.iter()
			.filter_map(|steam_id_data| {
				let id_number = steam_id_data.id.parse::<u32>().ok()?;

				let app_info = self.app_info_file.apps.get(&id_number)?;

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
						BytesRegex::new(&steam_id_data.id)
							.map_or(false, |regex| regex.is_match(&assets_cache_bytes))
					});

				if !owned {
					return None;
				}

				let installed = self
					.steam_dir
					.app(id_number)
					.map_or(false, |steam_app| steam_app.is_some());

				let release_date = app_info
					.original_release_date
					.or(app_info.steam_release_date)
					.unwrap_or_default();

				Some(OwnedGame {
					id: steam_id_data.id.clone(),
					name: app_info.name.clone(),
					installed,
					os_list,
					engine: steam_id_data.engine,
					release_date,
					thumbnail_url: get_steam_thumbnail(&steam_id_data.id),
				})
			})
			.collect())
	}
}
