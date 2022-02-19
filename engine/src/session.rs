//! Game session module.

use std::borrow::Borrow;

use eyre::Context;

use {
    crate::{
        clocks::{TimeSpan, TimeStamp},
        system::DEFAULT_TICK_SPAN,
    },
    eyre::WrapErr as _,
    std::{
        collections::VecDeque,
        future::Future,
        io::{Read as _, Write as _},
        num::NonZeroU64,
    },
};

const DEFAULT_INPUT_QUEUE_CAP: usize = 4;
const CLIENT_INIT_MAGIC: [u8; 16] = *b"arcanaclientinit";
const SERVER_INIT_MAGIC: [u8; 16] = *b"arcanaserverinit";
const PLAYER_JOIN: [u8; 16] = *b"arcanaplayerjoin";
const PLAYER_JOINED: [u8; 16] = *b"arcanajoinedplay";
const PLAYERS_INPUT: [u8; 16] = *b"arcananetidinput";
const STATE_UPDATE: [u8; 16] = *b"arcanastateupdte";

/// Unique network identifier for an entity.
/// Uniqueness can be guaranteed only within one game session.
/// Servers may safely convert `EntityId` to `NetId`.
/// Clients must map their `EntityId` to `NetId`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct NetId {
    value: NonZeroU64,
}

impl NetId {
    pub fn from_server_entity(entity: edict::EntityId) -> Self {
        NetId {
            value: NonZeroU64::new(entity.to_bits() + 1).unwrap(),
        }
    }

    pub fn into_server_entity(self) -> edict::EntityId {
        edict::EntityId::from_bits(self.value.get() - 1)
    }
}

/// Game session on client side in client-server game.
///
/// Network communications are inherently async, but this type hides most of the asynchrony.
pub struct ClientSession<P, J, I, U> {
    /// Current tick played by player.
    current_tick: u64,

    /// Instant of the last tick
    current_tick_stamp: TimeStamp,

    /// Tick span requested by server.
    tick_span: TimeSpan,

    /// Queue with last few inputs.
    input_queue: VecDeque<I>,

    /// UDP connection to the server.
    socket: tokio::net::UdpSocket,
}

enum Request<P, I> {
    Input { player: NetId, input: I },
    PlayerJoin { player_info: P },
}

enum Response<J, U> {
    Update { raw: Vec<u8> },
}

impl<P, J, I, U> ClientSession<P, J, I, U> {
    pub async fn new(addr: std::net::SocketAddr) -> eyre::Result<Self> {
        let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
            .await
            .wrap_err_with(|| "Failed to create udp socket for client session")?;

        socket
            .connect(addr)
            .await
            .wrap_err_with(|| "Failed to connect udp socket to server addr")?;

        socket
            .send(&CLIENT_INIT_MAGIC.to_be_bytes())
            .await
            .wrap_err_with(|| "Failed to send handshake to the server")?;

        let mut magic = [0; 4];
        let len = socket
            .recv(&mut magic)
            .await
            .wrap_err_with(|| "Failed to receive handshake magic from server")?;

        if len != 4 || u32::from_be_bytes(magic) != SERVER_INIT_MAGIC {
            return Err(eyre::eyre!("Server handshake response error"));
        }

        tokio::task::spawn(async move {});

        ClientSession {
            current_tick: 0,
            current_tick_stamp: TimeStamp::ORIGIN,
            tick_span: DEFAULT_TICK_SPAN,
            input_queue: VecDeque::with_capacity(DEFAULT_INPUT_QUEUE_CAP),
            socket,
        }
    }

    pub fn player_join(&mut self, player: P::PlayerInfo) -> impl Future<Output = P::JoinInfo> {
        todo!()
    }

    /// Sends inputs to the server for current tick.
    pub fn send_input(&mut self, player: NetId, input: P::Input) {}
}

pub trait HasPlayerId {
    fn player_id(&self) -> NetId;
}
