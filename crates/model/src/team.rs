use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, SqlitePool};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::connection::{Connection, ConnectionExt};

use super::{types::Key, user::User};

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Team {
    pub id: Key<Team>,
    pub name: String,
    pub slug: String,
    pub limit: Option<i64>,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
    pub created_by: Option<Key<User>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum TeamPermission {
    Edit,
    Delete,
    Config,
}

impl Team {
    pub async fn create(&self, connection: &Connection) -> anyhow::Result<()> {
        connection
            .execute(
                "INSERT INTO teams (id, name, slug, \"limit\", enabled, created_at, created_by) \
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    self.id,
                    self.name.clone(),
                    self.slug.clone(),
                    self.limit,
                    self.enabled,
                    self.created_at
                        .format(&Rfc3339)
                        .context("failed to format date")?,
                    self.created_by,
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn update(
        &mut self,
        connection: &Connection,
        name: &str,
        slug: &str,
        limit: Option<i64>,
        enabled: bool,
    ) -> anyhow::Result<()> {
        connection
            .execute(
                "UPDATE teams SET name = ?1, slug = ?2, \"limit\" = ?3, enabled = ?4 WHERE id = ?5",
                (
                    String::from(name),
                    String::from(slug),
                    limit,
                    enabled,
                    self.id,
                ),
            )
            .await?;

        self.name = name.to_string();
        self.slug = slug.to_string();
        self.limit = limit;
        self.enabled = enabled;

        Ok(())
    }

    pub async fn get(connection: &Connection, id: Key<Team>) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional("SELECT * FROM teams WHERE id = ?1", [id])
            .await
    }

    pub async fn get_by_slug(connection: &Connection, slug: &str) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional("SELECT * FROM teams WHERE slug = ?1", [String::from(slug)])
            .await
    }

    pub async fn get_list(connection: &Connection) -> anyhow::Result<Vec<Self>> {
        connection
            .query("SELECT * FROM teams ORDER BY name ASC", ())
            .await
    }

    pub async fn get_for_user(
        connection: &Connection,
        user: Key<User>,
    ) -> anyhow::Result<Vec<Self>> {
        connection
            .query(
                "SELECT teams.* FROM teams \
            LEFT JOIN team_members ON team_members.team = teams.id \
            WHERE team_members.user = ?1",
                [user],
            )
            .await
    }

    pub async fn delete(&self, connection: &Connection) -> anyhow::Result<()> {
        connection
            .execute("DELETE FROM team_members WHERE team = ?1", [self.id])
            .await?;

        connection
            .execute("DELETE FROM teams WHERE id = ?1", [self.id])
            .await?;

        Ok(())
    }

    pub async fn slug_exists(
        connection: &Connection,
        existing: Option<Key<Team>>,
        slug: &str,
    ) -> anyhow::Result<bool> {
        if let Some(existing) = existing {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM teams WHERE slug = ?1 AND id != ?2)",
                    (String::from(slug), existing),
                )
                .await
        } else {
            connection
                .query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM teams WHERE slug = ?1)",
                    [String::from(slug)],
                )
                .await
        }
    }

    pub async fn is_member(
        &self,
        connection: &Connection,
        user: Key<User>,
    ) -> anyhow::Result<bool> {
        connection
            .query_scalar(
                "SELECT EXISTS (SELECT 1 FROM team_members WHERE team = ?1 AND user = ?2)",
                (self.id, user),
            )
            .await
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct TeamMember {
    pub team: Key<Team>,
    pub user: Key<User>,
    pub name: String,
    pub can_edit: bool,
    pub can_delete: bool,
    pub can_config: bool,
}

impl TeamMember {
    pub async fn get_for_user(
        connection: &Connection,
        user: Key<User>,
    ) -> anyhow::Result<Vec<TeamMember>> {
        connection
            .query(
                "SELECT team_members.team, \
                team_members.user, \
                users.name, \
                team_members.can_edit, \
                team_members.can_delete, \
                team_members.can_config \
                FROM team_members \
                LEFT JOIN users ON users.id = team_members.user \
                WHERE team_members.user = ?1",
                [user],
            )
            .await
    }

    pub async fn get_for_team(
        connection: &Connection,
        team: Key<Team>,
    ) -> anyhow::Result<Vec<Self>> {
        connection
            .query(
                "SELECT team_members.team, \
                team_members.user, \
                users.name, \
                team_members.can_edit, \
                team_members.can_delete, \
                team_members.can_config \
                FROM team_members \
                LEFT JOIN users ON users.id = team_members.user \
                WHERE team_members.team = ?1",
                [team],
            )
            .await
    }

    pub async fn get_for_user_and_team(
        connection: &Connection,
        user: Key<User>,
        team: Key<Team>,
    ) -> anyhow::Result<Option<Self>> {
        connection
            .query_optional(
                "SELECT team_members.team, \
            team_members.user, \
            users.name, \
            team_members.can_edit, \
            team_members.can_delete, \
            team_members.can_config \
            FROM team_members \
            LEFT JOIN users ON users.id = team_members.user \
            WHERE team_members.user = ?1 AND team_members.team = ?2",
                (user, team),
            )
            .await
    }

    pub async fn set_user_permissions(
        connection: &Connection,
        team: Key<Team>,
        user: Key<User>,
        can_edit: bool,
        can_delete: bool,
        can_config: bool,
    ) -> anyhow::Result<()> {
        connection
            .execute(
                "INSERT INTO team_members (team, user, can_edit, can_delete, can_config) \
                VALUES (?1, ?2, ?3, ?4, ?5) \
                ON CONFLICT (team, user) DO UPDATE SET \
                can_edit = ?3, can_delete = ?4, can_config = ?5",
                (team, user, can_edit, can_delete, can_config),
            )
            .await?;

        Ok(())
    }

    /// Batch update permissions for multiple team members.
    /// Each tuple is (user_id, can_edit, can_delete, can_config).
    /// Only updates existing members (does not insert new ones).
    ///
    /// TODO: Since changeover to `libsql`, this is no longer one statement in a nice batch.
    pub async fn batch_update_permissions(
        connection: &Connection,
        team: Key<Team>,
        members: &[(Key<User>, bool, bool, bool)],
    ) -> anyhow::Result<()> {
        for (user_id, can_edit, can_delete, can_config) in members {
            connection
                .execute(
                    "UPDATE team_members SET \
                    can_edit = ?1, \
                    can_delete = ?2, \
                    can_config = ?3 \
                    WHERE team = ?4 AND user = ?5",
                    (*can_edit, *can_delete, *can_config, team, *user_id),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct TeamStats {
    pub total: i32,
}

impl TeamStats {
    pub async fn get(connection: &Connection) -> anyhow::Result<TeamStats> {
        connection
            .query_one("SELECT COUNT(*) AS total FROM teams", ())
            .await
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct TeamTab {
    pub id: Key<Team>,
    pub name: String,
    pub slug: String,
    pub count: i64,
}

impl TeamTab {
    pub async fn get_for_user(
        connection: &Connection,
        user: Key<User>,
    ) -> anyhow::Result<Vec<Self>> {
        connection
            .query(
                "SELECT teams.id, teams.name, teams.slug, COUNT(uploads.id) AS count \
                FROM teams \
                LEFT JOIN uploads ON uploads.owner_team = teams.id \
                LEFT JOIN team_members ON team_members.team = teams.id \
                WHERE team_members.user = ?1 \
                GROUP BY teams.id",
                [user],
            )
            .await
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct HomeTab {
    pub count: i64,
}

impl HomeTab {
    pub async fn get_for_user(connection: &Connection, user: Key<User>) -> anyhow::Result<Self> {
        connection
            .query_one(
                "SELECT COUNT(uploads.id) AS count \
                FROM uploads \
                WHERE uploads.owner_user = $1",
                [user],
            )
            .await
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct TeamList {
    pub id: Key<Team>,
    pub name: String,
    pub enabled: bool,
    pub limit: Option<i64>,
    pub upload_count: i64,
    pub upload_total: i64,
    pub member_count: i64,
    pub created_at: OffsetDateTime,
}

impl TeamList {
    pub async fn get(connection: &Connection) -> anyhow::Result<Vec<TeamList>> {
        Self::get_with_pagination(connection, 0, 50).await
    }

    pub async fn get_with_pagination(
        connection: &Connection,
        offset: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<TeamList>> {
        connection.query(
            "SELECT \
            teams.id, teams.name, teams.enabled, teams.\"limit\", teams.created_at, \
            (SELECT COUNT(*) FROM team_members WHERE team_members.team = teams.id) AS member_count, \
            COUNT(uploads.id) AS upload_count, \
            SUM(uploads.size) AS upload_total \
            FROM teams \
            LEFT JOIN uploads ON uploads.owner_team = teams.id \
            GROUP BY teams.id \
            ORDER BY teams.name ASC \
            LIMIT $? OFFSET $?",
            (limit as i64, offset as i64),
        )
        .await
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct TeamSelect {
    pub value: Key<Team>,
    pub label: String,
    pub enabled: bool,
}

impl TeamSelect {
    pub async fn get(connection: &Connection) -> anyhow::Result<Vec<Self>> {
        connection
            .query(
                "SELECT id as value, name as label, enabled FROM teams ORDER BY name ASC",
                (),
            )
            .await
    }
}
