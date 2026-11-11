use crate::result::Result;

use protocol::{
    self,
    ServerPacket,
    kattauth::UserDetails,
    User,
    UserId,
    Member,
    Channel,
    ChannelId,
    MessageId
};

use sqlx::postgres::PgPool;

/// # Errors
/// Accumulates the errors from database and client interactions when building and sending a
/// `ServerData` packet.
pub async fn send_server_data(
    pool:   PgPool,
    output: tokio::sync::mpsc::Sender<ServerPacket>,
    user:   &UserDetails,
) -> Result<()> {
    // TODO move to Connected
    let _ = sqlx::query!(
        "
            INSERT INTO users (user_id, username)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO NOTHING
        ",
        user.user_id,
        user.username,
    )
        .execute(&pool)
        .await?;

    let packet = get_server_data(pool, user.user_id).await?;

    let res = output.send(packet).await;

    if let Err(e) = res {
        eprintln!("An error occured while transmitting data internally to the client handler: {e} (recipient: {user:?})");
        return Ok(())
    }
    
    Ok(())
}

/// # Errors
/// Accumulates the errors from database and client interactions when building and sending a
/// `ServerData` packet.
pub async fn get_server_data(
    pool:    PgPool,
    user_id: UserId,
) -> Result<ServerPacket> {

    let channels = sqlx::query_as!(
        Channel,
        "
            SELECT c.id, c.name
            FROM channels c
            WHERE c.private = false
            OR EXISTS (
                SELECT 1
                FROM channel_members cm
                WHERE cm.channel_id = c.id
                AND   cm.user_id = $1
                AND   cm.access = true
            )
        ",
        user_id
    )
        .fetch_all(&pool)
        .await?;

    let members = sqlx::query_as!(
        Member,
        "
            SELECT users.user_id, users.username
            FROM users
        "
    )
        .fetch_all(&pool)
        .await?;

    let packet = ServerPacket::ServerData {
        name: String::from("Cicadas Server"),
        channels,
        members,
    };

    return Ok(packet);
}

pub async fn send_messages_before(
    pool:       PgPool,
    reply_tx:   tokio::sync::mpsc::Sender<ServerPacket>,
    channel_id: ChannelId,
    before:     Option<MessageId>,
    user:       &UserDetails,
) {
    // querys for 50 latest messages, optionally starting from a message with the id `before`, in
    // the channel with the id `channel_id` if the user either has specific permissions or the
    // channel is not public.
    let (messages_res, access_res) = tokio::join! {
        sqlx::query!(
            r#"
                WITH recent_messages AS (
                    SELECT 
                        msg.id, 
                        u.username AS name, 
                        msg.channel_id, 
                        msg.timestamp, 
                        msg.content
                    FROM messages AS msg
                    INNER JOIN channels AS ch ON ch.id = msg.channel_id
                    LEFT JOIN channel_members AS ch_members 
                        ON ch_members.channel_id = ch.id
                        AND ch_members.user_id = $1
                    INNER JOIN users AS u ON u.user_id = msg.author_id
                    WHERE msg.channel_id = $2
                    AND (ch_members.access = true OR ch.private = false)
                    AND ($3::bigint IS NULL OR msg.timestamp < (
                        SELECT timestamp FROM messages WHERE id = $3
                    ))
                    ORDER BY msg.timestamp DESC
                    LIMIT 50
                )
                SELECT *
                FROM recent_messages
                ORDER BY timestamp ASC
            "#,
            user.user_id,
            channel_id,
            before as Option<i64>,
        )
        .fetch_all(&pool),

        sqlx::query_scalar!(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM channels AS ch
                    LEFT JOIN channel_members AS ch_members
                        ON ch_members.channel_id = ch.id
                        AND ch_members.user_id = $1
                    WHERE ch.id = $2
                    AND (ch_members.access = true OR ch.private = false)
                ) AS "has_access!"
            "#,
            user.user_id,
            channel_id,
        )
        .fetch_one(&pool)
    };

    match access_res {
        Ok(true) => {} // authorized => proceed
        Ok(false) => {
            let _ = reply_tx.send(ServerPacket::Error {
                code: protocol::Error::Unauthorized,
                reason: "You are either not allowed in this channel or it does not exist.".to_string(),
            }).await;

            return;
        }
        Err(e) => {
            eprintln!("Database error while checking access for channel '{channel_id}': {e}");
            return;
        }
    }

    match messages_res {
        Ok(rows) => {
            for row in rows {
                let packet = ServerPacket::NewMessage {
                    id: row.id,
                    user: User { name: row.name },
                    channel_id: row.channel_id,
                    timestamp: row.timestamp,
                    content: row.content,
                };

                if let Err(e) = reply_tx.send(packet).await {
                    eprintln!("Unable to send NewMessage data to client {user:?}: {e}");
                    break;
                }
            }
        }
        Err(e) => {
            eprintln!("Error while querying for messages in channel with id '{channel_id}': {e}");
        }
    }
}
