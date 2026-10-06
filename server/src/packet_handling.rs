use sqlx::postgres::{ PgPool };
// use anyhow::{ anyhow, Context };

use crate::{
    result::Result,
    protocol::{
        self,
        ServerPacket,
        UserDetails, User, Member,
        Channel, ChannelId,
        MessageId
    },
};

/// # Errors
/// Accumulates the errors from database and client interactions when building and sending a
/// `ServerData` packet.
pub async fn just_connected(
    pool:   PgPool,
    output: tokio::sync::mpsc::Sender<ServerPacket>,
    user:   &UserDetails,
) -> Result<()> {
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
        user.user_id
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

    let res = output.send(ServerPacket::ServerData {
        name: String::from("Cicadas Server"),
        channels,
        members,
    })
        .await;
        // .inspect_err(|e| eprintln!("Unable to send Server data to client {user:?}: {e}"));

    if let Err(e) = res {
        eprintln!("An error occured while transmitting data internally to the client handler: {e} (recipient: {user:?})");
        return Ok(())
    }

    // let mut rows = sqlx::query!(
    //     "
    //         SELECT 
    //             m.id, 
    //             u.username AS name, 
    //             m.channel_id, 
    //             m.timestamp, 
    //             m.content
    //         FROM messages m
    //         INNER JOIN channel_members cm ON cm.channel_id = m.channel_id
    //         INNER JOIN users u ON m.author_id = u.user_id
    //         WHERE cm.user_id = $1
    //         ORDER BY m.timestamp DESC
    //         LIMIT 150
    //     ",
    //     user.user_id
    // )
    //     .fetch_all(&pool)
    //     .await?;
    //
    // rows.reverse();
    //
    // for row in rows {
    //     let packet = ServerPacket::NewMessage {
    //         id: row.id,
    //         user: User { name: row.name },
    //         channel_id: row.channel_id,
    //         timestamp: row.timestamp,
    //         content: row.content,
    //     }; 
    //     if let Err(e) = output.send(packet).await {
    //         eprintln!("Unable to send NewMessage data to client {user:?}: {e}");
    //         break;
    //     }
    // }
    
    Ok(())
}

pub async fn send_messages_before(
    pool:       PgPool,
    reply_tx:   tokio::sync::mpsc::Sender<ServerPacket>,
    channel_id: ChannelId,
    before:     Option<MessageId>,
    user:       &UserDetails,
) {
    let query_result = sqlx::query!(
        "
            WITH recent_messages AS (
                SELECT 
                    msg.id, 
                    u.username AS name, 
                    msg.channel_id, 
                    msg.timestamp, 
                    msg.content
                FROM messages AS msg
                LEFT JOIN channels AS ch ON ch.id = msg.channel_id
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
        ",
        user.user_id,
        channel_id,
        before,
    )
        .fetch_all(&pool)
        .await;

    match query_result {
        Ok(rows) => {
            // if let Some(reply_tx) = active_users.write().await.get(&user.user_id) {
            if rows.is_empty() {
                let _ = reply_tx.send(ServerPacket::Error {
                    code: protocol::Error::Unauthorized,
                    reason: "You are either not allowed in this channel or it does not exists.".to_string()
                }).await;

                return;
            }

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
            eprintln!("Error while querying for messages in channel with id \'{channel_id}\': {e}");
        }
    }
}
