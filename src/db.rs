use thiserror::Error;

use crate::poll::{create_poll_format_from_bytes, Poll, PollData, PollID, PollType};

pub type DbPool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Internal error (database)")]
    Internal,
    #[error("No such poll")]
    NoSuchPoll,
    #[error("Database error: {0:?}")]
    Database(rusqlite::Error),
    #[error("Database query error: {0:?}")]
    Query(rusqlite::Error),
    #[error("Database insert error: {0:?}")]
    Insert(rusqlite::Error),
    #[error("Database connection error: {0:?}")]
    Connection(r2d2::Error),
    #[error("Failed serializing poll data: {0:?}")]
    SerializationError(anyhow::Error),
}

impl actix_web::error::ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            Error::NoSuchPoll => actix_web::http::StatusCode::BAD_REQUEST,
            _ => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Retrieves a single poll using it's unique id from the database
pub async fn get_poll(pool: &DbPool, id: PollID) -> Result<Poll, Error> {
    let conn = pool.get().map_err(Error::Connection)?;

    let mut query = conn
        .prepare("SELECT * FROM polls where id = ?1")
        .map_err(Error::Query)?;

    // TODO: Use column indices instead of names and check map_err-s
    let mut poll_iter = query
        .query_map([id.index()], |row| {
            use rusqlite::types::Type;
            let ptype = PollType::try_parse(&row.get::<_, String>("type")?)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(2, Type::Text, e.into()))?;
            Ok(Poll {
                data: PollData {
                    id,
                    ptype,
                    name: row.get("name")?,
                    date_created: chrono::DateTime::parse_from_rfc3339(
                        &row.get::<_, String>("date_created")?,
                    )
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(3, Type::Text, e.into())
                    })?
                    .into(),
                    admin_link: row.get("admin_link")?,
                    voters: row.get("voters")?,
                },
                format: create_poll_format_from_bytes(ptype, row.get("format_data")?).map_err(
                    |e| rusqlite::Error::FromSqlConversionFailure(7, Type::Blob, e.into()),
                )?,
            })
        })
        .map_err(Error::Query)?;

    let poll = poll_iter
        .next()
        .ok_or(Error::NoSuchPoll)?
        .map_err(Error::Database)?;

    // Verify the randpart
    if poll.data.id.randpart() != id.randpart() {
        return Err(Error::NoSuchPoll);
    }

    Ok(poll)
}

/// Retrieves the last ID assigned to a poll in the database
/// Note: This does not find the last PollID (with random part) only the raw ID part
pub async fn last_id(pool: &DbPool) -> Result<usize, Error> {
    let conn = pool.get().map_err(Error::Connection)?;
    let mut query = conn
        .prepare("SELECT MAX(id) FROM polls")
        .map_err(Error::Query)?;

    let mut i = query
        // If there are no rows, this returns InvalidColumnType, so assume 0
        .query_map([], |row| Ok(row.get(0).unwrap_or(0)))
        .map_err(Error::Database)?;
    // This error (Internal) shouldn't ever happen, but better be safe
    let id = i.next().ok_or(Error::Internal)?.map_err(Error::Database)?;
    Ok(id)
}

/// Inserts a poll into the database
pub async fn insert_poll(pool: &DbPool, poll: Poll) -> Result<(), Error> {
    let conn = pool.get().map_err(Error::Connection)?;

    let params = rusqlite::params![
        // If randpart is > 2^63 this fails; convert to i64 to avoid this
        poll.data.id.randpart() as i64,
        poll.data.ptype.to_string(),
        poll.data.name,
        poll.data.date_created.to_rfc3339(),
        poll.data.admin_link,
        poll.data.voters,
        poll.format
            .save_state()
            .map_err(Error::SerializationError)?,
    ];

    conn
        .execute("INSERT INTO polls (randpart, type, name, date_created, admin_link, voters, format_data) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    params)
        .map_err(Error::Insert)?;

    Ok(())
}

/// Sets the number of voters and updates the format data
/// Returns true if a poll was updated
pub async fn update_poll(pool: &DbPool, poll: &Poll) -> Result<bool, Error> {
    let conn = pool.get().map_err(Error::Connection)?;

    let params = rusqlite::params![
        poll.data.id.index(),
        poll.data.voters,
        poll.format
            .save_state()
            .map_err(Error::SerializationError)?,
    ];

    conn.execute(
        "UPDATE polls SET voters = ?2, format_data = ?3 WHERE id = ?1",
        params,
    )
    .map_err(Error::Query)
    // Safety: id is unique, so the number of rows updated is always 0 or 1
    .map(|u| u == 1)
}

/// Completely clears the polls table,
/// returns number of deleted rows
pub async fn purge(pool: &DbPool) -> Result<usize, Error> {
    pool.get()
        .map_err(Error::Connection)?
        .execute("DELETE FROM polls WHERE id IS NOT NULL", [])
        .map_err(Error::Query)
}

/// Deletes a poll, returns true if a poll was deleted
pub async fn delete_poll(pool: &DbPool, id: PollID) -> Result<bool, Error> {
    pool.get()
        .map_err(Error::Connection)?
        .execute("DELETE FROM polls WHERE id = ?1", [id.index()])
        .map_err(Error::Query)
        // Safety: id is unique, so the number of rows updated is always 0 or 1
        .map(|u| u == 1)
}

/// Retrieves *ALL POLLS*. If there are a lot of polls, this can and will obliterate the server's
/// RAM.
pub async fn list_polls(pool: &DbPool) -> Result<Vec<crate::templates::PollInfo>, Error> {
    let conn = pool.get().map_err(Error::Connection)?;

    let mut query = conn.prepare("SELECT * FROM polls").map_err(Error::Query)?;

    let poll_iter = query
        .query_map([], |row| {
            let id = PollID::new(row.get(0)?, row.get(1)?);
            Ok(crate::templates::PollInfo {
                id,
                name: row.get(3)?,
                poll_type: row.get(2)?,
                admin_token: row.get(5)?,
                voters: row.get(6)?,
                date_created: row.get(4)?,
            })
        })
        .map_err(Error::Query)?;

    let polls: Result<Vec<crate::templates::PollInfo>, rusqlite::Error> = poll_iter.collect();

    polls.map_err(Error::Database)
}
