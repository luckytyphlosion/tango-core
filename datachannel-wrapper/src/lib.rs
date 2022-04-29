pub struct PeerConnection {
    peer_conn: Box<datachannel::RtcPeerConnection<PeerConnectionHandler>>,
    data_channel_rx: tokio::sync::mpsc::Receiver<DataChannel>,
    signal_receiver: PeerConnectionSignalReceiver,
}

#[derive(Clone)]
pub struct PeerConnectionSignalReceiver {
    inner: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<PeerConnectionSignal>>>,
}

impl PeerConnectionSignalReceiver {
    pub async fn recv(&self) -> Option<PeerConnectionSignal> {
        self.inner.lock().await.recv().await
    }
}

impl PeerConnection {
    pub fn new(config: RtcConfig) -> anyhow::Result<Self> {
        let (signal_tx, signal_rx) = tokio::sync::mpsc::channel(1);
        let (data_channel_tx, data_channel_rx) = tokio::sync::mpsc::channel(1);
        let pch = PeerConnectionHandler {
            signal_tx,
            pending_dc_receiver: None,
            data_channel_tx,
        };
        let peer_conn = datachannel::RtcPeerConnection::new(&config, pch)?;
        Ok(PeerConnection {
            peer_conn,
            data_channel_rx,
            signal_receiver: PeerConnectionSignalReceiver {
                inner: std::sync::Arc::new(tokio::sync::Mutex::new(signal_rx)),
            },
        })
    }

    pub fn signal_receiver(&self) -> PeerConnectionSignalReceiver {
        self.signal_receiver.clone()
    }

    pub fn create_data_channel(
        &mut self,
        label: &str,
        dc_init: DataChannelInit,
    ) -> anyhow::Result<DataChannel> {
        let (message_tx, message_rx) = tokio::sync::mpsc::channel(1);
        let (open_tx, open_rx) = tokio::sync::oneshot::channel();
        let error_cell = std::sync::Arc::new(tokio::sync::OnceCell::new());
        let dch = DataChannelHandler {
            message_tx: Some(message_tx),
            open_tx: Some(open_tx),
            error_cell: error_cell.clone(),
        };
        let dc = self
            .peer_conn
            .create_data_channel_ex(label, dch, &dc_init)?;
        Ok(DataChannel {
            dc,
            state: tokio::sync::Mutex::new(DataChannelState::Pending(open_rx)),
            receiver: DataChannelReceiver {
                inner: std::sync::Arc::new(tokio::sync::Mutex::new(message_rx)),
            },
            error_cell,
        })
    }

    pub async fn accept(&mut self) -> Option<DataChannel> {
        self.data_channel_rx.recv().await
    }

    pub fn set_local_description(&mut self, sdp_type: SdpType) -> anyhow::Result<()> {
        self.peer_conn.set_local_description(sdp_type)?;
        Ok(())
    }

    pub fn set_remote_description(&mut self, sess_desc: SessionDescription) -> anyhow::Result<()> {
        self.peer_conn.set_remote_description(&sess_desc)?;
        Ok(())
    }

    pub fn local_description(&self) -> Option<datachannel::SessionDescription> {
        self.peer_conn.local_description()
    }

    pub fn remote_description(&self) -> Option<datachannel::SessionDescription> {
        self.peer_conn.remote_description()
    }

    pub fn add_remote_candidate(&mut self, cand: &datachannel::IceCandidate) -> anyhow::Result<()> {
        self.peer_conn.add_remote_candidate(cand)?;
        Ok(())
    }
}

struct PeerConnectionHandler {
    signal_tx: tokio::sync::mpsc::Sender<PeerConnectionSignal>,
    pending_dc_receiver: Option<(
        DataChannelReceiver,
        tokio::sync::oneshot::Receiver<()>,
        std::sync::Arc<tokio::sync::OnceCell<DataChannelError>>,
    )>,
    data_channel_tx: tokio::sync::mpsc::Sender<DataChannel>,
}

pub enum PeerConnectionSignal {
    SessionDescription(SessionDescription),
    IceCandidate(IceCandidate),
}

impl datachannel::PeerConnectionHandler for PeerConnectionHandler {
    type DCH = DataChannelHandler;

    fn data_channel_handler(&mut self) -> Self::DCH {
        let (message_tx, message_rx) = tokio::sync::mpsc::channel(1);
        let (open_tx, open_rx) = tokio::sync::oneshot::channel();
        let error_cell = std::sync::Arc::new(tokio::sync::OnceCell::new());
        let dch = DataChannelHandler {
            message_tx: Some(message_tx),
            open_tx: Some(open_tx),
            error_cell: error_cell.clone(),
        };
        self.pending_dc_receiver = Some((
            DataChannelReceiver {
                inner: std::sync::Arc::new(tokio::sync::Mutex::new(message_rx)),
            },
            open_rx,
            error_cell,
        ));
        dch
    }

    fn on_description(&mut self, sess_desc: SessionDescription) {
        let _ = self
            .signal_tx
            .blocking_send(PeerConnectionSignal::SessionDescription(sess_desc));
    }

    fn on_candidate(&mut self, cand: IceCandidate) {
        let _ = self
            .signal_tx
            .blocking_send(PeerConnectionSignal::IceCandidate(cand));
    }

    fn on_connection_state_change(&mut self, _state: datachannel::ConnectionState) {}

    fn on_gathering_state_change(&mut self, _state: datachannel::GatheringState) {}

    fn on_signaling_state_change(&mut self, _state: datachannel::SignalingState) {}

    fn on_data_channel(&mut self, dc: Box<datachannel::RtcDataChannel<Self::DCH>>) {
        let (dcr, open_rx, error_cell) = self.pending_dc_receiver.take().unwrap();
        let _ = self.data_channel_tx.blocking_send(DataChannel {
            dc,
            error_cell,
            receiver: dcr,
            state: tokio::sync::Mutex::new(DataChannelState::Pending(open_rx)),
        });
    }
}

#[derive(Clone)]
pub struct DataChannelReceiver {
    inner: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<Vec<u8>>>>,
}

impl DataChannelReceiver {
    pub async fn receive(&mut self) -> Option<Vec<u8>> {
        self.inner.lock().await.recv().await
    }
}

enum DataChannelState {
    Pending(tokio::sync::oneshot::Receiver<()>),
    Open,
}

pub struct DataChannel {
    state: tokio::sync::Mutex<DataChannelState>,
    error_cell: std::sync::Arc<tokio::sync::OnceCell<DataChannelError>>,
    receiver: DataChannelReceiver,
    dc: Box<datachannel::RtcDataChannel<DataChannelHandler>>,
}

impl DataChannel {
    pub async fn send(&mut self, msg: &[u8]) -> Result<(), DataChannelError> {
        if let Some(err) = self.error_cell.get() {
            return Err(err.clone());
        }

        let mut state = self.state.lock().await;
        match &mut *state {
            DataChannelState::Pending(r) => {
                r.await.map_err(|_| DataChannelError::Closed)?;
                *state = DataChannelState::Open;
            }
            DataChannelState::Open => {}
        }
        self.dc
            .send(msg)
            .map_err(|e| DataChannelError::UnderlyingError(format!("{:?}", e)))?;
        Ok(())
    }

    pub fn receiver(&self) -> DataChannelReceiver {
        self.receiver.clone()
    }
}

struct DataChannelHandler {
    error_cell: std::sync::Arc<tokio::sync::OnceCell<DataChannelError>>,
    open_tx: Option<tokio::sync::oneshot::Sender<()>>,
    message_tx: Option<tokio::sync::mpsc::Sender<Vec<u8>>>,
}

#[derive(Debug, Clone)]
pub enum DataChannelError {
    Closed,
    UnderlyingError(String),
}

impl std::error::Error for DataChannelError {}

impl std::fmt::Display for DataChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl datachannel::DataChannelHandler for DataChannelHandler {
    fn on_open(&mut self) {
        let _ = self.open_tx.take().unwrap().send(());
    }

    fn on_closed(&mut self) {
        self.message_tx = None;
    }

    fn on_error(&mut self, err: &str) {
        let _ = self
            .error_cell
            .set(DataChannelError::UnderlyingError(err.to_owned()));
    }

    fn on_message(&mut self, msg: &[u8]) {
        let _ = self
            .message_tx
            .as_mut()
            .unwrap()
            .blocking_send(msg.to_vec());
    }

    fn on_buffered_amount_low(&mut self) {}

    fn on_available(&mut self) {}
}

pub type SessionDescription = datachannel::SessionDescription;
pub type IceCandidate = datachannel::IceCandidate;
pub type SdpSession = datachannel::sdp::SdpSession;
pub type SdpType = datachannel::SdpType;
pub type RtcConfig = datachannel::RtcConfig;
pub type DataChannelInit = datachannel::DataChannelInit;
pub type Reliability = datachannel::Reliability;
