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
    let local_description =
        if let Some(datachannel_wrapper::PeerConnectionSignal::SessionDescription(sess_desc)) =
            signal_receiver.recv().await
        {
            sess_desc
        } else {
            anyhow::bail!("session description not received")
        };

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

    match match stream
        .try_next()
        .await?
        .ok_or(anyhow::format_err!("stream ended early"))?
    {
        tokio_tungstenite::tungstenite::Message::Binary(d) => protocol::Packet::deserialize(&d)?,
        _ => anyhow::bail!("unexpected message format"),
    } {
        protocol::Packet::Start(_) => {
            anyhow::bail!("unexpected start");
        }
        protocol::Packet::Offer(offer) => {
            log::info!("received an offer, this is the polite side");

            peer_conn.set_local_description(datachannel_wrapper::SdpType::Rollback)?;
            peer_conn.set_remote_description(datachannel_wrapper::SessionDescription {
                sdp_type: datachannel_wrapper::SdpType::Offer,
                sdp: datachannel_wrapper::parse_sdp(&offer.sdp.to_string(), false)?,
            })?;

            let local_description = if let Some(
                datachannel_wrapper::PeerConnectionSignal::SessionDescription(sess_desc),
            ) = signal_receiver.recv().await
            {
                sess_desc
            } else {
                anyhow::bail!("session description not received")
            };

            stream
                .send(tokio_tungstenite::tungstenite::Message::Binary(
                    protocol::Packet::Answer(protocol::Answer {
                        sdp: local_description.sdp.to_string(),
                    })
                    .serialize()?,
                ))
                .await?;
            log::info!("sent answer to impolite side");
        }
        protocol::Packet::Answer(answer) => {
            log::info!("received an answer, this is the impolite side");

            peer_conn.set_remote_description(datachannel_wrapper::SessionDescription {
                sdp_type: datachannel_wrapper::SdpType::Answer,
                sdp: datachannel_wrapper::parse_sdp(&answer.sdp.to_string(), false)?,
            })?;
        }
        protocol::Packet::ICECandidate(ice_candidate) => {
            peer_conn.add_remote_candidate(datachannel_wrapper::IceCandidate {
                candidate: ice_candidate.candidate,
                mid: ice_candidate.mid,
            })?;
        }
    }

    stream.close(None).await?;

    Ok(())
}
