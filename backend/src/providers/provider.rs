use std::{
	collections::HashMap,
	time::Instant,
};

use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use log::error;

use super::{
	epic_provider::EpicProvider,
	gog_provider::GogProvider,
	xbox_provider::XboxProvider,
};
use crate::{
	debug::LoggableInstant,
	installed_game::InstalledGame,
	owned_game::OwnedGame,
	providers::{
		manual_provider::ManualProvider,
		steam_provider::SteamProvider,
	},
	serializable_enum,
	Result,
};

serializable_enum!(ProviderId {
	Steam,
	Manual,
	Epic,
	Gog,
	Xbox,
});

#[enum_dispatch]
pub enum Provider {
	SteamProvider,
	ManualProvider,
	EpicProvider,
	GogProvider,
	XboxProvider,
}

#[async_trait]
#[enum_dispatch(Provider)]
pub trait ProviderActions {
	fn get_installed_games(&self) -> Result<Vec<InstalledGame>>;

	async fn get_owned_games(&self) -> Result<Vec<OwnedGame>>;
}

pub trait ProviderStatic {
	const ID: &'static ProviderId;

	fn new() -> Result<Self>
	where
		Self: Sized;
}

type Map = HashMap<String, Provider>;

fn create_map_entry<TProvider: ProviderActions + ProviderStatic>() -> Result<(String, Provider)>
where
	Provider: From<TProvider>,
{
	let mod_loader: Provider = TProvider::new()?.into();

	Ok((TProvider::ID.to_string(), mod_loader))
}

fn add_entry<TProvider: ProviderActions + ProviderStatic>(map: &mut Map)
where
	Provider: From<TProvider>,
{
	match create_map_entry::<TProvider>() {
		Ok((key, value)) => {
			map.insert(key, value);
		}
		Err(error) => error!("Failed to set up provider: {error}"),
	}
}

pub fn get_map() -> Map {
	let mut map = Map::new();
	let now = &mut Instant::now();

	add_entry::<SteamProvider>(&mut map);
	now.log_next("set up provider (Steam)");

	add_entry::<EpicProvider>(&mut map);
	now.log_next("set up provider (Epic)");

	add_entry::<GogProvider>(&mut map);
	now.log_next("set up provider (Gog)");

	add_entry::<XboxProvider>(&mut map);
	now.log_next("set up provider (Xbox)");

	add_entry::<ManualProvider>(&mut map);
	now.log_next("set up provider (Manual)");

	map
}
