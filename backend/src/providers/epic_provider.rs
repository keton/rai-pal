use std::{
	fs::{
		self,
		File,
	},
	io::Read,
	path::PathBuf,
};

use async_trait::async_trait;
use base64::engine::general_purpose;
use glob::GlobError;
use log::error;
use winreg::{
	enums::HKEY_LOCAL_MACHINE,
	RegKey,
};

use super::{
	provider::{
		self,
		ProviderId,
	},
	provider_command::ProviderCommand,
};
use crate::{
	game_engines::game_engine::GameEngine,
	installed_game::InstalledGame,
	owned_game::OwnedGame,
	paths::glob_path,
	pc_gaming_wiki,
	provider::{
		ProviderActions,
		ProviderStatic,
	},
	serializable_struct,
	Result,
};

pub struct Epic {
	app_data_path: PathBuf,
	engine_cache: provider::EngineCache,
}

impl ProviderStatic for Epic {
	const ID: &'static ProviderId = &ProviderId::Epic;

	fn new() -> Result<Self>
	where
		Self: Sized,
	{
		let app_data_path = RegKey::predef(HKEY_LOCAL_MACHINE)
			.open_subkey(r"SOFTWARE\WOW6432Node\Epic Games\EpicGamesLauncher")
			.and_then(|launcher_reg| launcher_reg.get_value::<String, _>("AppDataPath"))
			.map(PathBuf::from)?;

		let engine_cache = Self::try_get_engine_cache();

		Ok(Self {
			app_data_path,
			engine_cache,
		})
	}
}

serializable_struct!(EpicManifest {
	#[serde(rename = "DisplayName")]
	display_name: String,
	#[serde(rename = "LaunchExecutable")]
	launch_executable: String,
	#[serde(rename = "InstallLocation")]
	install_location: String,
	#[serde(rename = "CatalogNamespace")]
	catalog_namespace: String,
	#[serde(rename = "CatalogItemId")]
	catalog_item_id: String,
	#[serde(rename = "AppName")]
	app_name: String,
});

serializable_struct!(EpicCatalogCategory { path: String });

serializable_struct!(EpicCatalogReleaseInfo {
	app_id: String,
	platform: Vec<String>,
	date_added: Option<String>,
});

serializable_struct!(EpicCatalogImage {
	height: i32,
	url: String,
});

serializable_struct!(EpicCatalogItem {
	id: String,
	namespace: String,
	title: String,
	categories: Vec<EpicCatalogCategory>,
	release_info: Vec<EpicCatalogReleaseInfo>,
	key_images: Vec<EpicCatalogImage>,
});

impl EpicCatalogItem {
	fn get_release_date(&self) -> Option<i64> {
		Some(
			self.release_info
				.first()?
				.date_added
				.as_ref()?
				.parse::<chrono::DateTime<chrono::Utc>>()
				.ok()?
				.timestamp(),
		)
	}

	fn get_thumbnail_url(&self) -> Option<String> {
		Some(
			self.key_images
				.iter()
				// The available images here seem to be a bit inconsistent, so we'll just pick the smallest one.
				.min_by_key(|image| image.height)?
				.url
				.clone(),
		)
	}
}

#[async_trait]
impl ProviderActions for Epic {
	fn get_installed_games(&self) -> Result<Vec<InstalledGame>> {
		let manifests = glob_path(&self.app_data_path.join("Manifests").join("*.item"))?;

		Ok(manifests
			.filter_map(
				|manifest_path_result| match read_manifest(manifest_path_result) {
					Ok(manifest) => {
						let path = PathBuf::from(manifest.install_location)
							.join(manifest.launch_executable);

						let mut game =
							InstalledGame::new(&path, &manifest.display_name, Self::ID.to_owned())?;
						game.set_start_command_string(&format!(
							"com.epicgames.launcher://apps/{}?action=launch&silent=true",
							manifest.app_name
						));
						game.set_provider_game_id(&manifest.catalog_item_id);

						Some(game)
					}
					Err(err) => {
						error!("Failed to glob manifest path: {err}");
						None
					}
				},
			)
			.collect())
	}

	async fn get_owned_games(&self) -> Result<Vec<OwnedGame>> {
		let mut file = File::open(self.app_data_path.join("Catalog").join("catcache.bin"))?;

		let mut decoder = base64::read::DecoderReader::new(&mut file, &general_purpose::STANDARD);
		let mut json = String::default();
		decoder.read_to_string(&mut json)?;

		let items = serde_json::from_str::<Vec<EpicCatalogItem>>(&json)?;

		let owned_games = futures::future::join_all(items.iter().map(|catalog_item| async {
			if catalog_item
				.categories
				.iter()
				.all(|category| category.path != "games")
			{
				return None;
			}

			let mut game = OwnedGame::new(&catalog_item.id, *Self::ID, &catalog_item.title);

			game.set_install_command(ProviderCommand::String(format!(
				"com.epicgames.launcher://apps/{}%3A{}%3A{}?action=install",
				catalog_item.namespace,
				catalog_item.id,
				catalog_item
					.release_info
					.first()
					.map(|release_info| release_info.app_id.clone())
					.unwrap_or_default(),
			)));

			if let Some(thumbnail_url) = catalog_item.get_thumbnail_url() {
				game.set_thumbnail_url(&thumbnail_url);
			}

			if let Some(release_date) = catalog_item.get_release_date() {
				game.set_release_date(release_date);
			}

			if let Some(engine) = get_engine(&catalog_item.title, &self.engine_cache).await {
				game.set_engine(engine);
			}

			Some(game)
		}))
		.await
		.into_iter()
		.flatten();

		Self::try_save_engine_cache(
			&owned_games
				.clone()
				.map(|owned_game| (owned_game.name.clone(), owned_game.engine))
				.collect(),
		);

		Ok(owned_games.collect())
	}
}

async fn get_engine(title: &str, cache: &provider::EngineCache) -> Option<GameEngine> {
	if let Some(cached_engine) = cache.get(title) {
		return cached_engine.clone();
	}

	pc_gaming_wiki::get_engine_from_game_title(title).await
}

fn read_manifest(path_result: std::result::Result<PathBuf, GlobError>) -> Result<EpicManifest> {
	let json = fs::read_to_string(path_result?)?;
	let manifest = serde_json::from_str::<EpicManifest>(&json)?;
	Ok(manifest)
}
