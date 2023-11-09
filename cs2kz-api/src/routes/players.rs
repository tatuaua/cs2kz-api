use {
	crate::{
		database,
		res::{player as res, BadRequest},
		util::{Created, Filter, Limit, Offset},
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::NaiveTime,
	cs2kz::{PlayerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

static ROOT_GET_BASE_QUERY: &str = r#"
	SELECT
		p1.*,
		p2.playtime,
		p2.afktime
	FROM
		players p1
		JOIN playtimes p2 ON p2.player_id = p1.id
"#;

/// Query parameters for fetching players.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetPlayersParams {
	/// Name of a player.
	name: Option<String>,

	/// A minimum amount of playtime.
	playtime: Option<NaiveTime>,

	/// Only include (not) banned players.
	is_banned: Option<bool>,

	offset: Offset,
	limit: Limit<500>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Players", context_path = "/api/v0", path = "/players",
	params(GetPlayersParams),
	responses(
		(status = 200, body = Vec<Player>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_players(
	state: State,
	Query(GetPlayersParams { name, playtime, is_banned, offset, limit }): Query<GetPlayersParams>,
) -> Response<Vec<res::Player>> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);
	let mut filter = Filter::new();

	if let Some(ref name) = name {
		query
			.push(filter)
			.push(" p1.name LIKE ")
			.push_bind(format!("%{name}%"));

		filter.switch();
	}

	if let Some(playtime) = playtime {
		query
			.push(filter)
			.push(" p1.playtime >= ")
			.push_bind(playtime);

		filter.switch();
	}

	if let Some(is_banned) = is_banned {
		query
			.push(filter)
			.push(" p1.is_banned = ")
			.push_bind(is_banned);

		filter.switch();
	}

	query
		.push(" LIMIT ")
		.push_bind(offset.value)
		.push(",")
		.push_bind(limit.value);

	let players = query
		.build_query_as::<database::PlayerWithPlaytime>()
		.fetch_all(state.database())
		.await?
		.into_iter()
		.map(Into::into)
		.collect::<Vec<res::Player>>();

	if players.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(players))
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Players", context_path = "/api/v0", path = "/players/{ident}",
	params(("ident" = PlayerIdentifier, Path, description = "The player's `SteamID` or name")),
	responses(
		(status = 200, body = Player),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_player(
	state: State,
	Path(ident): Path<PlayerIdentifier<'_>>,
) -> Response<res::Player> {
	let mut query = QueryBuilder::new(ROOT_GET_BASE_QUERY);

	query.push(" WHERE ");

	match ident {
		PlayerIdentifier::SteamID(steam_id) => {
			query
				.push(" p1.id = ")
				.push_bind(steam_id.as_u32());
		}
		PlayerIdentifier::Name(name) => {
			query
				.push(" p1.name LIKE ")
				.push_bind(format!("%{name}%"));
		}
	};

	let player = query
		.build_query_as::<database::PlayerWithPlaytime>()
		.fetch_optional(state.database())
		.await?
		.ok_or(Error::NoContent)?
		.into();

	Ok(Json(player))
}

/// Information about a new KZ player.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewPlayer {
	/// The player's Steam name.
	name: String,

	/// The player's `SteamID`.
	steam_id: SteamID,

	/// The player's IP address.
	#[schema(value_type = String)]
	ip_address: Ipv4Addr,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Players", context_path = "/api/v0", path = "/players",
	request_body = NewPlayer,
	responses(
		(status = 201, body = ()),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_player(
	state: State,
	Json(NewPlayer { steam_id, name, ip_address }): Json<NewPlayer>,
) -> Result<Created<()>> {
	sqlx::query! {
		r#"
		INSERT INTO
			Players (id, name, ip_address)
		VALUES
			(?, ?, ?)
		"#,
		steam_id.as_u32(),
		name,
		ip_address.to_string(),
	}
	.execute(state.database())
	.await?;

	Ok(Created(()))
}

/// Updated information about a KZ player.
#[rustfmt::skip]
#[derive(Debug, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's new name.
	name: Option<String>,

	/// The player's new IP address.
	#[schema(value_type = String)]
	ip_address: Option<Ipv4Addr>,

	/* TODO(AlphaKeks): figure out what to take here. Probably a `course_id` as well? Maybe
	 * a `course_info` struct?
	 *
	 * /// The additional playtime recorded by the server.
	 * playtime: u32,
	 *
	 */
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(put, tag = "Players", context_path = "/api/v0", path = "/players/{steam_id}",
	params(("steam_id" = SteamID, Path, description = "The player's SteamID")),
	request_body = PlayerUpdate,
	responses(
		(status = 200),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn update_player(
	state: State,
	Path(steam_id): Path<SteamID>,
	Json(PlayerUpdate { name, ip_address }): Json<PlayerUpdate>,
) -> Result<()> {
	// TODO(AlphaKeks): update playtimes as well

	let id = steam_id.as_u32();
	let mut transaction = state.database().begin().await?;

	if let Some(name) = name {
		sqlx::query!("UPDATE Players SET name = ? WHERE id = ?", name, id)
			.execute(transaction.as_mut())
			.await?;
	}

	if let Some(ip_address) = ip_address.map(|ip| ip.to_string()) {
		sqlx::query!("UPDATE Players SET ip_address = ? WHERE id = ?", ip_address, id)
			.execute(transaction.as_mut())
			.await?;
	}

	transaction.commit().await?;

	Ok(())
}
