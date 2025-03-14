//! # Konarr Agent Websocket

use konarr::models::Projects;
use rocket::{
    State,
    futures::{SinkExt, StreamExt},
};
use ws::Message;

use crate::{AppState, guards::Session};

pub fn routes() -> Vec<rocket::Route> {
    routes![agent]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentRequest {
    pub project: u32,
}

#[get("/ws?agent")]
pub async fn agent<'a>(
    state: &'a State<AppState>,
    _session: Session,
    ws: ws::WebSocket,
) -> ws::Channel<'a> {
    ws.channel(move |mut stream| {
        Box::pin(async move {
            let connection = state.connection().await;
            let mut projects = vec![];

            while let Some(Ok(message)) = stream.next().await {
                let agent_req = match message {
                    Message::Text(ref text) => {
                        let agent_req: AgentRequest = match serde_json::from_str(&text) {
                            Ok(req) => req,
                            Err(e) => {
                                let _ = stream
                                    .send(Message::Text(
                                        serde_json::to_string(&e.to_string()).unwrap(),
                                    ))
                                    .await;
                                continue;
                            }
                        };

                        agent_req
                    }
                    _ => {
                        let _ = stream
                            .send(Message::Text(
                                serde_json::to_string(&"Invalid message".to_string()).unwrap(),
                            ))
                            .await;
                        continue;
                    }
                };

                if let Ok(mut project) =
                    Projects::fetch_by_primary_key(&connection, agent_req.project as i32).await
                {
                    if !projects.contains(&project.id) {
                        projects.push(project.id);
                    }

                    if let Ok(Some(mut snapshot)) = project.fetch_latest_snapshot(&connection).await
                    {
                        snapshot.fetch_metadata(&connection).await.unwrap();

                        log::info!(
                            "Agent connected, setting project '{}' online (snapshot: {})",
                            project.id,
                            snapshot.id
                        );

                        snapshot
                            .set_metadata(&connection, "status", "online")
                            .await
                            .unwrap();
                    }
                }
            }

            log::info!("Agent disconnected, setting projects offline");

            for project_id in projects.iter() {
                if let Ok(mut project) =
                    Projects::fetch_by_primary_key(&connection, project_id).await
                {
                    if let Ok(Some(mut snap)) = project.fetch_latest_snapshot(&connection).await {
                        snap.fetch_metadata(&connection).await.unwrap();
                        log::info!("Setting project '{}' offline", project.id);

                        snap.set_metadata(&connection, "status", "offline")
                            .await
                            .unwrap();
                    }
                }
            }

            Ok(())
        })
    })
}
