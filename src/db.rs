use crate::poll::{create_poll_format_from_bytes, Poll, PollData, PollID, PollType};

pub type DbPool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type _DbConnection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

pub enum Error {
    Internal,
    NoSuchPoll,
}

pub async fn get_poll(pool: &DbPool, id: PollID) -> Result<Poll, Error> {
    let pool = pool.clone();
    let conn = actix_web::web::block(move || pool.get())
        .await
        .map_err(|_| Error::Internal)?;

    let mut query = conn
        .prepare("SELECT * FROM polls where id = ?1")
        .map_err(|_| Error::Internal)?;

    let mut poll_iter = query
        .query_map([id.index()], |row| {
            let ptype = PollType::try_parse(&row.get::<_, String>("type")?).unwrap();
            Ok(Poll {
                data: PollData {
                    id,
                    ptype,
                    name: row.get("name")?,
                    date_created: chrono::DateTime::parse_from_rfc3339(
                        &row.get::<_, String>("date_created")?,
                    )
                    .unwrap()
                    .into(),
                    admin_link: row.get("admin_link")?,
                    voters: row.get("voters")?,
                },
                format: create_poll_format_from_bytes(ptype, row.get("format_data")?),
            })
        })
        .map_err(|e| {
            log::error!("Error while retrieving poll: {:?}", e);
            Error::Internal
        })?;

    let poll = poll_iter.next().ok_or(Error::NoSuchPoll)?.map_err(|e| {
        log::error!("Error while retrieving poll: {:?}", e);
        Error::Internal
    })?;
    if poll.data.id.randpart() != id.randpart() {
        return Err(Error::NoSuchPoll);
    }

    Ok(poll)
}

pub async fn last_id(pool: &DbPool) -> Result<i64, Error> {
    let pool = pool.clone();
    let conn = actix_web::web::block(move || pool.get())
        .await
        .map_err(|_| Error::Internal)?;
    Ok(conn.last_insert_rowid())
}

pub async fn insert_poll(pool: &DbPool, poll: Poll) -> Result<(), Error> {
    let pool = pool.clone();
    let conn = actix_web::web::block(move || pool.get())
        .await
        .map_err(|_| Error::Internal)?;

    let params = rusqlite::params![
        poll.data.id.randpart() as i64, // BUG: If randpart is > 2^63 this fails with ToSqlConversionFailure(TryFromIntError(())); convert to i64 to avoid this
        poll.data.ptype.to_string(),
        poll.data.name,
        poll.data.date_created.to_rfc3339(),
        poll.data.admin_link,
        poll.data.voters,
        poll.format.save_state(),
    ];

    conn
        .execute("INSERT INTO polls (randpart, type, name, date_created, admin_link, voters, format_data) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    params)
        .map_err(|e| {
            log::error!("Error when inserting poll: {:?}", e);
            log::error!("params: {:?}", (
                poll.data.id.randpart(),
                poll.data.ptype.to_string(),
                poll.data.name,
                poll.data.date_created.to_rfc3339(),
                poll.data.admin_link,
                poll.data.voters,
            ));
            Error::Internal
        })?;

    Ok(())
}

/// increments the number of voters and updates the format data
pub async fn update_poll(pool: &DbPool, poll: &Poll) -> Result<(), Error> {
    let pool = pool.clone();
    let conn = actix_web::web::block(move || pool.get())
        .await
        .map_err(|_| Error::Internal)?;

    let params = rusqlite::params![poll.data.id.index(), poll.format.save_state(),];

    conn.execute(
        "UPDATE polls SET voters = voters + 1, format_data = ?2 WHERE id = ?1",
        params,
    )
    .map_err(|_| Error::Internal)?;

    Ok(())
}
