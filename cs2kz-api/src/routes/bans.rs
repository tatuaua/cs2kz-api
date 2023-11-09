use {
	crate::{
		res::{bans as res, bans::BanReason, BadRequest},
		util::{Created, Filter, Limit, Offset},
		Error, Response, Result, State,
	},
	axum::{
		extract::{Path, Query},
		Json,
	},
	chrono::{DateTime, Utc},
	cs2kz::{PlayerIdentifier, ServerIdentifier, SteamID},
	serde::{Deserialize, Serialize},
	sqlx::QueryBuilder,
	std::net::Ipv4Addr,
	utoipa::{IntoParams, ToSchema},
};

/// Query parameters for fetching bans.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GetBansParams<'a> {
	/// `SteamID` or name of a player.
	player: Option<PlayerIdentifier<'a>>,

	/// A ban reason.
	reason: Option<BanReason>,

	/// The ID or name of a server.
	server: Option<ServerIdentifier<'a>>,

	/// Only include (non) expired bans.
	expired: Option<bool>,

	/// Only include bans that were issued after a certain date.
	created_after: Option<DateTime<Utc>>,

	/// Only include bans that were issued before a certain date.
	created_before: Option<DateTime<Utc>>,

	offset: Offset,
	limit: Limit<500>,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Bans", context_path = "/api/v0", path = "/bans",
	params(GetBansParams),
	responses(
		(status = 200, body = Vec<Ban>),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_bans(
	state: State,
	Query(GetBansParams {
		player,
		reason,
		server,
		expired,
		created_after,
		created_before,
		offset,
		limit,
	}): Query<GetBansParams<'_>>,
) -> Response<Vec<res::Ban>> {
	let mut query = QueryBuilder::new(
		r#"
		SELECT
			b.id,
			p.id steam_id,
			p.name,
			b.reason,
			b.created_on
		FROM
			Players p
			JOIN Bans b ON b.player_id = p.id
		"#,
	);

	let mut filter = Filter::new();

	if let Some(player) = player {
		query.push(filter);

		match player {
			PlayerIdentifier::SteamID(steam_id) => {
				query
					.push(" p.id = ")
					.push_bind(steam_id.as_u32());
			}
			PlayerIdentifier::Name(name) => {
				query
					.push(" p.name LIKE ")
					.push_bind(format!("%{name}%"));
			}
		};

		filter.switch();
	}

	if let Some(ref reason) = reason {
		query
			.push(filter)
			.push(" b.reason = ")
			.push_bind(reason);

		filter.switch();
	}

	if let Some(server) = server {
		let server_id = match server {
			ServerIdentifier::ID(id) => id,
			ServerIdentifier::Name(name) => {
				sqlx::query!("SELECT id FROM Servers WHERE name = ?", name)
					.fetch_one(state.database())
					.await?
					.id
			}
		};

		query
			.push(filter)
			.push(" b.server_id = ")
			.push_bind(server_id);

		filter.switch();
	}

	if let Some(expired) = expired {
		let now = Utc::now();

		query
			.push(filter)
			.push(" b.expires_on ")
			.push(if expired { " < " } else { " > " })
			.push_bind(now);

		filter.switch();
	}

	if let Some(created_after) = created_after {
		query
			.push(filter)
			.push(" b.created_on > ")
			.push_bind(created_after);

		filter.switch();
	}

	if let Some(created_before) = created_before {
		query
			.push(filter)
			.push(" b.created_on < ")
			.push_bind(created_before);

		filter.switch();
	}

	query
		.push(" LIMIT ")
		.push_bind(offset.value)
		.push(",")
		.push_bind(limit.value);

	let bans = query
		.build_query_as::<res::Ban>()
		.fetch_all(state.database())
		.await?;

	if bans.is_empty() {
		return Err(Error::NoContent);
	}

	Ok(Json(bans))
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(get, tag = "Bans", context_path = "/api/v0", path = "/bans/{id}/replay",
	params(("id" = u32, Path, description = "The ban's ID")),
	responses(
		(status = 200, body = ()),
		(status = 204),
		(status = 400, response = BadRequest),
		(status = 500, body = Error),
	),
)]
pub async fn get_replay(state: State, Path(ban_id): Path<u32>) -> Response<()> {
	todo!();
}

/// Submissions for a player ban.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NewBan {
	/// The player's `SteamID`.
	steam_id: SteamID,

	/// The player's IP address at the time of the ban.
	#[schema(value_type = String)]
	ip_address: Option<Ipv4Addr>,

	/// The reason for the ban.
	reason: BanReason,

	/// The `SteamID` of the admin who issued this ban.
	banned_by: Option<SteamID>,

	/// Information about the server this ban occurred on.
	server_info: Option<BanServerInfo>,

	/// Timestamp of when this ban expires.
	expires_on: Option<DateTime<Utc>>,
}

/// Information about the server this ban occurred on.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BanServerInfo {
	/// The ID of the server.
	id: u16,

	/// The cs2kz plugin version.
	plugin_version: u16,
}

/// Information about a newly created ban.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatedBan {
	/// The ban's ID.
	id: u32,
}

#[tracing::instrument(level = "DEBUG")]
#[utoipa::path(post, tag = "Bans", context_path = "/api/v0", path = "/bans",
	request_body = NewBan,
	responses(
		(status = 201, body = CreatedBan),
		(status = 400, response = BadRequest),
		(status = 401, body = Error),
		(status = 500, body = Error),
	),
)]
pub async fn create_ban(
	state: State,
	Json(NewBan { steam_id, ip_address, reason, banned_by, server_info, expires_on }): Json<NewBan>,
) -> Result<Created<Json<CreatedBan>>> {
	let mut transaction = state.database().begin().await?;

	sqlx::query! {
		r#"
		INSERT INTO
			Bans (
				player_id,
				player_ip,
				server_id,
				reason,
				banned_by,
				plugin_version,
				expires_on
			)
		VALUES
			(?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id.as_u32(),
		ip_address.map(|ip| ip.to_string()),
		server_info.as_ref().map(|info| info.id),
		reason,
		banned_by.map(|steam_id| steam_id.as_u32()),
		server_info.as_ref().map(|info| info.plugin_version),
		expires_on,
	}
	.execute(transaction.as_mut())
	.await?;

	let id = sqlx::query!("SELECT MAX(id) id FROM Bans")
		.fetch_one(transaction.as_mut())
		.await?
		.id
		.expect("ban was just inserted");

	transaction.commit().await?;

	Ok(Created(Json(CreatedBan { id })))
}
