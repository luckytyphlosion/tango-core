use super::protocol;
use futures_util::SinkExt;
use futures_util::TryStreamExt;

pub async fn connect(
    addr: &str,
    peer_conn: &mut datachannel_wrapper::PeerConnection,
    session_id: &str,
) -> Result<(), anyhow::Error>
where
{
    let (mut stream, _) = tokio_tungstenite::connect_async(addr).await?;

    log::info!("negotiation started");

    let signal_receiver = peer_conn.signal_receiver();
    loop {
        if let Some(datachannel_wrapper::PeerConnectionSignal::GatheringStateChange(
            datachannel_wrapper::GatheringState::Complete,
        )) = signal_receiver.recv().await
        {
            break;
        }
    }

    let local_description = peer_conn.local_description().unwrap();
    stream
        .send(tokio_tungstenite::tungstenite::Message::Binary(
            protocol::Packet::Start(protocol::Start {
                protocol_version: protocol::VERSION,
                session_id: session_id.to_string(),
                offer_sdp: local_description.sdp.to_string(),
            })
            .serialize()?,
        ))
        .await?;
    log::info!("negotiation start sent");

    loop {
        tokio::select! {
            signal_msg = signal_receiver.recv() => {
                let cand = if let Some(datachannel_wrapper::PeerConnectionSignal::IceCandidate(cand)) = signal_msg {
                    cand
                } else {
                    anyhow::bail!("ice candidate not received")
                };

                stream
                    .send(tokio_tungstenite::tungstenite::Message::Binary(
                        protocol::Packet::ICECandidate(protocol::ICECandidate {
                            candidate: cand.candidate,
                            mid: cand.mid,
                        })
                        .serialize()?,
                    ))
                    .await?;
            }
            ws_msg = stream.try_next() => {
                let raw = if let Some(raw) = ws_msg? {
                    raw
                } else {
                    anyhow::bail!("stream ended early");
                };

                let packet = if let tokio_tungstenite::tungstenite::Message::Binary(d) = raw {
                    protocol::Packet::deserialize(&d)?
                } else {
                    anyhow::bail!("invalid packet");
                };

                match packet {
                    protocol::Packet::Start(_) => {
                        anyhow::bail!("unexpected start");
                    }
                    protocol::Packet::Offer(offer) => {
                        log::info!("received an offer, this is the polite side. rolling back our local description and switching to answer");

                        peer_conn.set_local_description(datachannel_wrapper::SdpType::Rollback)?;
                        peer_conn.set_remote_description(datachannel_wrapper::SessionDescription {
                            sdp_type: datachannel_wrapper::SdpType::Offer,
                            sdp: datachannel_wrapper::parse_sdp(&offer.sdp.to_string(), false)?,
                        })?;

                        let local_description = peer_conn.local_description().unwrap();
                        stream
                            .send(tokio_tungstenite::tungstenite::Message::Binary(
                                protocol::Packet::Answer(protocol::Answer {
                                    sdp: local_description.sdp.to_string(),
                                })
                                .serialize()?,
                            ))
                            .await?;
                        log::info!("sent answer to impolite side");
                        break;
                    }
                    protocol::Packet::Answer(answer) => {
                        log::info!("received an answer, this is the impolite side");

                        peer_conn.set_remote_description(datachannel_wrapper::SessionDescription {
                            sdp_type: datachannel_wrapper::SdpType::Answer,
                            sdp: datachannel_wrapper::parse_sdp(&answer.sdp.to_string(), false)?,
                        })?;
                        break;
                    }
                    protocol::Packet::ICECandidate(_ice_candidate) => {
                        anyhow::bail!("ice candidates not supported");
                    }
                }
            }
        };
    }

    stream.close(None).await?;

    Ok(())
}
