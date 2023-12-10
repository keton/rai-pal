use std::{
	collections::HashMap,
	fs,
	io::Cursor,
	path::{
		Path,
		PathBuf,
	},
};

use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use zip::ZipArchive;

use super::{
	bepinex::BepInEx,
	mod_database::{self,},
	unreal_vr::UnrealVr,
};
use crate::{
	game_mod::CommonModData,
	installed_game::InstalledGame,
	local_mod::{
		self,
		LocalMod,
		ModKind,
	},
	mod_loaders::mod_database::ModDatabase,
	paths,
	remote_mod::{
		RemoteMod,
		RemoteModData,
	},
	serializable_struct,
	Error,
	Result,
};

serializable_struct!(ModLoaderData {
	pub id: String,
	pub path: PathBuf,
	pub kind: ModKind,
});

#[enum_dispatch]
#[derive(Clone)]
pub enum ModLoader {
	BepInEx,
	UnrealVr,
}

#[async_trait]
#[enum_dispatch(ModLoader)]
pub trait ModLoaderActions {
	fn install(&self, game: &InstalledGame) -> Result;
	async fn install_mod(&self, game: &InstalledGame, game_mod: &LocalMod) -> Result;
	fn get_data(&self) -> &ModLoaderData;
	fn get_mod_path(&self, mod_data: &CommonModData) -> Result<PathBuf>;
	fn get_local_mods(&self) -> Result<HashMap<String, LocalMod>>;

	fn get_installed_mods_path(&self) -> Result<PathBuf> {
		Ok(paths::app_data_path()?
			.join("mod-loaders")
			.join(&self.get_data().id)
			.join("mods"))
	}

	async fn get_remote_mods<F>(&self, error_handler: F) -> HashMap<String, RemoteMod>
	where
		F: Fn(Error) + Send,
	{
		let data = self.get_data();
		let id = &data.id;

		let database = mod_database::get(id).await.unwrap_or_else(|error| {
			error_handler(error);
			ModDatabase {
				mods: HashMap::new(),
			}
		});

		database
			.mods
			.into_iter()
			.map(|(mod_id, database_mod)| {
				(
					mod_id.clone(),
					RemoteMod {
						common: CommonModData {
							id: mod_id,
							engine: database_mod.engine,
							unity_backend: database_mod.unity_backend,
							loader_id: id.clone(),
						},
						data: RemoteModData {
							author: database_mod.author,
							description: database_mod.description,
							source_code: database_mod.source_code,
							title: database_mod.title,
							downloads: database_mod.downloads,
						},
					},
				)
			})
			.collect()
	}

	async fn download_mod(&self, remote_mod: &RemoteMod) -> Result {
		let target_path = self.get_mod_path(&remote_mod.common)?;
		let data = self.get_data();
		let downloads_folder = data.path.join("downloads");
		fs::create_dir_all(&downloads_folder)?;

		if let Some(first_download) = remote_mod.data.downloads.first() {
			let response = reqwest::get(&first_download.url).await?;

			if response.status().is_success() {
				// This keeps the whole zip in memory and only copies the extracted part to disk.
				// If we ever need to support very big mods, we should stream the zip to disk first,
				// and extract it after it's written to disk.
				ZipArchive::new(Cursor::new(response.bytes().await?))?.extract(&target_path)?;

				// Saves the manifest so we know which version of the mod we installed.
				fs::write(
					local_mod::get_manifest_path(&target_path),
					serde_json::to_string_pretty(&local_mod::Manifest {
						version: first_download.version.clone(),
					})?,
				)?;

				Ok(())
			} else {
				Err(Error::ModNotFound(remote_mod.common.id.to_string())) // TODO error
			}
		} else {
			Err(Error::ModNotFound(remote_mod.common.id.to_string())) // TODO error
		}
	}
}

#[async_trait]
pub trait ModLoaderStatic {
	const ID: &'static str;

	async fn new(resources_path: &Path) -> Result<Self>
	where
		Self: Sized;
}

pub type Map = HashMap<String, ModLoader>;
pub type DataMap = HashMap<String, ModLoaderData>;

async fn create_map_entry<TModLoader: ModLoaderActions + ModLoaderStatic>(
	path: &Path,
) -> Result<(String, ModLoader)>
where
	ModLoader: std::convert::From<TModLoader>,
{
	let mod_loader: ModLoader = TModLoader::new(path).await?.into();

	Ok((TModLoader::ID.to_string(), mod_loader))
}

async fn add_entry<TModLoader: ModLoaderActions + ModLoaderStatic>(path: &Path, map: &mut Map)
where
	ModLoader: std::convert::From<TModLoader>,
{
	match create_map_entry::<TModLoader>(path).await {
		Ok((key, value)) => {
			map.insert(key, value);
		}
		Err(err) => eprintln!("Failed to create map entry: {err}"),
	}
}

pub async fn get_map(resources_path: &Path) -> Map {
	let mut map = Map::new();

	add_entry::<BepInEx>(resources_path, &mut map).await;
	add_entry::<UnrealVr>(resources_path, &mut map).await;

	map
}

pub fn get_data_map(map: &Map) -> Result<DataMap> {
	map.values()
		.map(|mod_loader| {
			let data = mod_loader.get_data();
			Ok((data.id.clone(), data.clone()))
		})
		.collect()
}
