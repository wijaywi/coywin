use libp2p::{
    gossipsub, mdns, noise, swarm::NetworkBehaviour, tcp, yamux, Swarm, SwarmBuilder
};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockProposed {
    pub hash: [u8; 32],
    pub signature: Vec<u8>,
    pub nonce: u64,
}

#[derive(NetworkBehaviour)]
pub struct CoywinBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
}

pub async fn build_swarm() -> Result<Swarm<CoywinBehaviour>, Box<dyn std::error::Error>> {
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };
            
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(1))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .expect("Valid config");
            
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            ).expect("Valid gossipsub");

            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            ).expect("Valid mdns");

            CoywinBehaviour { gossipsub, mdns }
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    
    let topic = gossipsub::IdentTopic::new("coywin-blocks");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    Ok(swarm)
}
