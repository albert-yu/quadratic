use anyhow::{anyhow, Result};
use axum::extract::ws::{Message, WebSocket};
use chrono::{DateTime, Utc};
use futures_util::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::state::State;
use crate::{get_mut_room, get_room};

#[derive(Serialize, Debug, Clone)]
pub(crate) struct User {
    pub session_id: Uuid,
    pub user_id: String,
    pub first_name: String,
    pub last_name: String,
    pub image: String,
    #[serde(flatten)]
    pub state: UserState,
    #[serde(skip_serializing)]
    pub socket: Option<Arc<Mutex<SplitSink<WebSocket, Message>>>>,
    #[serde(skip_serializing)]
    pub last_heartbeat: DateTime<Utc>,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.session_id == other.session_id
            && self.user_id == other.user_id
            && self.first_name == other.first_name
            && self.last_name == other.last_name
            && self.image == other.image
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub(crate) struct UserState {
    pub sheet_id: Option<Uuid>,
    pub selection: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

impl Default for UserState {
    fn default() -> Self {
        UserState {
            sheet_id: None,
            selection: None,
            x: None,
            y: None,
        }
    }
}

impl State {
    /// Retrieves a copy of a user in a room
    pub(crate) async fn _get_user_in_room(
        &self,
        file_id: &Uuid,
        session_id: &Uuid,
    ) -> Result<User> {
        let user = get_room!(self, file_id)?
            .users
            .get(session_id)
            .ok_or(anyhow!("User {} not found in Room {}", session_id, file_id))?
            .to_owned();

        Ok(user)
    }

    /// Remove stale users in a room.  Remove the number of users removed in the room.
    pub(crate) async fn remove_stale_users_in_room(
        &self,
        file_id: Uuid,
        heartbeat_timeout_s: i64,
    ) -> Result<usize> {
        let stale_users = get_room!(self, file_id)?
            .users
            .iter()
            .filter(|(_, user)| {
                user.last_heartbeat.timestamp() + heartbeat_timeout_s < Utc::now().timestamp()
            })
            .map(|(user_id, _)| user_id.to_owned())
            .collect::<Vec<Uuid>>();

        for user_id in stale_users.iter() {
            tracing::info!("Removing stale user {} from room {}", user_id, file_id);

            self.leave_room(file_id, user_id).await?;
        }

        Ok(stale_users.len())
    }

    /// Updates a user's heartbeat in a room
    pub(crate) async fn update_heartbeat(&self, file_id: Uuid, session_id: &Uuid) -> Result<()> {
        get_mut_room!(self, file_id)?
            .users
            .entry(session_id.to_owned())
            .and_modify(|user| {
                user.last_heartbeat = Utc::now();
                tracing::trace!("Updating heartbeat for {session_id}");
            });

        Ok(())
    }

    /// updates a user's state in a room
    pub(crate) async fn update_user_state(
        &self,
        file_id: &Uuid,
        session_id: &Uuid,
        update: &UserState,
    ) -> Result<()> {
        get_mut_room!(self, file_id)?
            .users
            .entry(session_id.to_owned())
            .and_modify(|user| {
                if let Some(sheet_id) = &update.sheet_id {
                    user.state.sheet_id = Some(sheet_id.to_owned());
                }
                if let Some(selection) = &update.selection {
                    user.state.selection = Some(selection.to_owned());
                }
                if let Some(x) = &update.x {
                    user.state.x = Some(*x);
                }
                if let Some(y) = &update.y {
                    user.state.y = Some(*y);
                }
                user.last_heartbeat = Utc::now();
                tracing::trace!("Updating sheet_id for {session_id}");
            });

        Ok(())
    }

    /// Updates a user's selection in a room
    pub(crate) async fn update_selection(
        &self,
        file_id: Uuid,
        session_id: &Uuid,
        selection: &String,
    ) -> Result<()> {
        get_mut_room!(self, file_id)?
            .users
            .entry(session_id.to_owned())
            .and_modify(|user| {
                user.state.selection = Some(selection.to_owned());
                user.last_heartbeat = Utc::now();
                tracing::trace!("Updating selection for {session_id}");
            });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::new_user;

    use super::*;

    #[tokio::test]
    async fn updates_a_users_heartbeat() {
        let state = State::new();
        let internal_session_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();
        let user = new_user();

        state.enter_room(file_id, &user, internal_session_id).await;
        let old_heartbeat = state
            ._get_user_in_room(&file_id, &user.session_id)
            .await
            .unwrap()
            .last_heartbeat;

        state
            .update_heartbeat(file_id, &user.session_id)
            .await
            .unwrap();
        let new_heartbeat = state
            ._get_user_in_room(&file_id, &user.session_id)
            .await
            .unwrap()
            .last_heartbeat;

        assert!(old_heartbeat < new_heartbeat);
    }
}
