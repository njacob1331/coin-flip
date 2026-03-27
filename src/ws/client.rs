use anyhow::Result;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use kanal::{AsyncReceiver, AsyncSender};

use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use url::Url;

use crate::ws::message::{DepthUpdate, Message, Subscription};

const WS_URL: &str = "wss://ws.gemini.com?snapshot=-1";

pub type Writer = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;
pub type Reader = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;
pub type WsMessage = tokio_tungstenite::tungstenite::Message;

pub struct WsClient;

impl WsClient {
    pub async fn connect() -> Result<(Writer, Reader)> {
        let url = Url::parse(WS_URL).expect("invalid WS_URL constant");
        let (ws_stream, _) = connect_async(url).await?;

        Ok(ws_stream.split())
    }

    pub fn spawn_reader(
        mut reader: Reader,
        feed_tx: AsyncSender<DepthUpdate>,
        sub_tx: AsyncSender<Subscription>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = reader.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => match serde_json::from_str::<Message>(&text) {
                        Ok(parsed) => {
                            match parsed {
                                Message::DepthUpdate(update) => {
                                    if feed_tx.send(update).await.is_err() {
                                        eprintln!("ws feed channel error"); // ← was let _ = ...
                                    }
                                }

                                Message::Subscription(sub) => {
                                    if sub_tx.send(sub).await.is_err() {
                                        eprintln!("sub feed channel closed"); // ← was let _ = ...
                                    }
                                }

                                _ => {}
                            }
                        }
                        Err(e) => eprintln!("ws deserialize error: {e}"),
                    },
                    Ok(WsMessage::Close(frame)) => {
                        eprintln!("ws closed by server: {frame:?}");
                        break;
                    }
                    Err(e) => {
                        eprintln!("ws reader error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            eprintln!("ws reader task exiting");
        })
    }

    pub fn spawn_writer(mut writer: Writer, ws_rx: AsyncReceiver<String>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Ok(req) = ws_rx.recv().await {
                if let Err(e) = writer.send(WsMessage::Text(req)).await {
                    eprintln!("ws writer error: {e}");
                    break;
                }
            }
            eprintln!("ws writer task exiting");
        })
    }
}

// pub struct WsClient {
//     msg_tx: AsyncSender<String>,
//     feed_rx: AsyncReceiver<Message>, // ← was Sender<Message>
//     _stream: JoinHandle<()>,
// }

// impl WsClient {
//     pub async fn connect() -> Result<Self> {
//         let url = Url::parse(WS_URL).expect("invalid WS_URL constant");
//         let (ws_stream, _) = connect_async(url).await?;
//         let (writer, reader) = ws_stream.split();

//         let (msg_tx, msg_rx) = kanal::bounded_async::<String>(8);
//         Self::spawn_writer(writer, msg_rx);

//         let (feed_tx, feed_rx) = kanal::bounded_async::<Message>(1024); // ← changed
//         let _stream = Self::spawn_reader(reader, feed_tx);

//         Ok(Self {
//             msg_tx,
//             feed_rx,
//             _stream,
//         })
//     }

//     pub fn feed(&self) -> AsyncReceiver<Message> {
//         self.feed_rx.clone()
//     }

//     pub fn msg_sender(&self) -> AsyncSender<String> {
//         self.msg_tx.clone()
//     }

//     fn spawn_writer(mut writer: Writer, msg_rx: AsyncReceiver<String>) -> JoinHandle<()> {
//         tokio::spawn(async move {
//             while let Ok(req) = msg_rx.recv().await {
//                 if let Err(e) = writer.send(WsMessage::Text(req)).await {
//                     eprintln!("ws writer error: {e}");
//                     break;
//                 }
//             }
//             eprintln!("ws writer task exiting");
//         })
//     }

//     fn spawn_reader(mut reader: Reader, sender: AsyncSender<Message>) -> JoinHandle<()> {
//         // ← Sender type changed
//         tokio::spawn(async move {
//             while let Some(msg) = reader.next().await {
//                 match msg {
//                     Ok(WsMessage::Text(text)) => match serde_json::from_str::<Message>(&text) {
//                         Ok(parsed) => {
//                             if sender.send(parsed).await.is_err() {
//                                 eprintln!("ws feed channel closed"); // ← was let _ = ...
//                                 break;
//                             }
//                         }
//                         Err(e) => eprintln!("ws deserialize error: {e}"),
//                     },
//                     Ok(WsMessage::Close(frame)) => {
//                         eprintln!("ws closed by server: {frame:?}");
//                         break;
//                     }
//                     Err(e) => {
//                         eprintln!("ws reader error: {e}");
//                         break;
//                     }
//                     _ => {}
//                 }
//             }
//             eprintln!("ws reader task exiting");
//         })
//     }
// }
