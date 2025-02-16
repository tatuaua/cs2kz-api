/**
 * The different types of jumpstats.
 *
 * The rows in this table are represented as an enum in Rust.
 * See `cs2kz::Jumpstat` for more information.
 */
CREATE TABLE IF NOT EXISTS JumpstatTypes (
	`id` INT1 UNSIGNED NOT NULL AUTO_INCREMENT,
	`name` VARCHAR(16),
	`created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (`id`)
);

INSERT INTO
	JumpstatTypes (`name`)
VALUES
	("longjump");

INSERT INTO
	JumpstatTypes (`name`)
VALUES
	("single_bhop");

INSERT INTO
	JumpstatTypes (`name`)
VALUES
	("multi_bhop");

INSERT INTO
	JumpstatTypes (`name`)
VALUES
	("weirdjump");

INSERT INTO
	JumpstatTypes (`name`)
VALUES
	("ladderjump");

INSERT INTO
	JumpstatTypes (`name`)
VALUES
	("ladderhop");

/**
 * Jumpstats track distance records for different types of jumps in different modes and styles.
 */
CREATE TABLE IF NOT EXISTS Jumpstats (
	`id` INT8 UNSIGNED NOT NULL AUTO_INCREMENT,
	`type` INT1 UNSIGNED NOT NULL,
	`mode_id` INT1 UNSIGNED NOT NULL,
	`style_id` INT1 UNSIGNED NOT NULL,
	`strafes` INT1 UNSIGNED NOT NULL,
	`distance` DECIMAL(7, 4) UNSIGNED NOT NULL,
	`sync` DECIMAL(5, 2) UNSIGNED NOT NULL,
	`pre` DECIMAL(7, 4) UNSIGNED NOT NULL,
	`max` DECIMAL(7, 4) UNSIGNED NOT NULL,
	`overlap` DECIMAL(3, 2) UNSIGNED NOT NULL,
	`bad_air` DECIMAL(3, 2) UNSIGNED NOT NULL,
	`dead_air` DECIMAL(3, 2) UNSIGNED NOT NULL,
	`height` DECIMAL(5, 2) UNSIGNED NOT NULL,
	`airpath` DECIMAL(5, 4) UNSIGNED NOT NULL,
	`deviation` DECIMAL(7, 4) UNSIGNED NOT NULL,
	`average_width` DECIMAL(5, 2) UNSIGNED NOT NULL,
	`airtime` DECIMAL(5, 4) UNSIGNED NOT NULL,
	`player_id` INT4 UNSIGNED NOT NULL,
	`server_id` INT2 UNSIGNED NOT NULL,
	`plugin_version_id` INT2 UNSIGNED NOT NULL,
	`created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	PRIMARY KEY (`id`),
	FOREIGN KEY (`type`) REFERENCES JumpstatTypes (`id`),
	FOREIGN KEY (`mode_id`) REFERENCES Modes (`id`),
	FOREIGN KEY (`style_id`) REFERENCES Styles (`id`),
	FOREIGN KEY (`player_id`) REFERENCES Players (`steam_id`),
	FOREIGN KEY (`server_id`) REFERENCES Servers (`id`),
	FOREIGN KEY (`plugin_version_id`) REFERENCES PluginVersions (`id`)
);

CREATE TABLE IF NOT EXISTS SuspiciousJumpstats AS
SELECT
	*
FROM
	Jumpstats;

CREATE TABLE IF NOT EXISTS CheatedJumpstats AS
SELECT
	*
FROM
	Jumpstats;
