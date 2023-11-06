use {
	cs2kz::SteamID,
	serde::Serialize,
	sqlx::FromRow,
	utoipa::{ToResponse, ToSchema},
};

pub mod player;
pub mod bans;
pub mod maps;
pub mod servers;
pub mod records;

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct PlayerInfo {
	pub name: String,
	pub steam_id: SteamID,
}

#[derive(ToResponse)]
#[response(description = "Request body is malformed in some way.")]
pub struct BadRequest;
