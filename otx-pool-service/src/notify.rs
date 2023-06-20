use ckb_async_runtime::Handle;
use ckb_stop_handler::{SignalSender, StopHandler};
use ckb_types::H256;
use otx_format::jsonrpc_types::OpenTransaction;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot,
};

use std::collections::HashMap;

pub type RuntimeHandle = Handle;

/// Asynchronous request sent to the service.
pub struct Request<A, R> {
    /// Oneshot channel for the service to send back the response.
    pub responder: oneshot::Sender<R>,
    /// Request arguments.
    pub arguments: A,
}

impl<A, R> Request<A, R> {
    /// Call the service with the arguments and wait for the response.
    pub async fn call(sender: &Sender<Request<A, R>>, arguments: A) -> Option<R> {
        let (responder, response) = oneshot::channel();
        let _ = sender
            .send(Request {
                responder,
                arguments,
            })
            .await;
        response.await.ok()
    }
}

pub const SIGNAL_CHANNEL_SIZE: usize = 1;
pub const REGISTER_CHANNEL_SIZE: usize = 2;
pub const NOTIFY_CHANNEL_SIZE: usize = 128;

pub type NotifyRegister<M> = Sender<Request<String, Receiver<M>>>;

#[derive(Clone)]
pub struct NotifyController {
    stop: StopHandler<()>,
    new_open_tx_register: NotifyRegister<OpenTransaction>,
    new_open_tx_notifier: Sender<OpenTransaction>,
    commit_open_tx_register: NotifyRegister<Vec<H256>>,
    commit_open_tx_notifier: Sender<Vec<H256>>,
    interval_register: NotifyRegister<u64>,
    interval_notifier: Sender<u64>,
    start_register: NotifyRegister<()>,
    start_notifier: Sender<()>,
    stop_register: NotifyRegister<()>,
    stop_notifier: Sender<()>,
    handle: Handle,
}

impl Drop for NotifyController {
    fn drop(&mut self) {
        self.stop.try_send(());
    }
}

pub struct NotifyService {
    new_open_tx_subscribers: HashMap<String, Sender<OpenTransaction>>,
    commit_open_tx_subscribers: HashMap<String, Sender<Vec<H256>>>,
    interval_subscribers: HashMap<String, Sender<u64>>,
    start_subscribers: HashMap<String, Sender<()>>,
    stop_subscribers: HashMap<String, Sender<()>>,
}

impl Default for NotifyService {
    fn default() -> Self {
        Self::new()
    }
}

impl NotifyService {
    pub fn new() -> Self {
        Self {
            new_open_tx_subscribers: HashMap::default(),
            commit_open_tx_subscribers: HashMap::default(),
            interval_subscribers: HashMap::default(),
            start_subscribers: HashMap::default(),
            stop_subscribers: HashMap::default(),
        }
    }

    /// start background tokio spawned task.
    pub fn start(mut self, handle: Handle) -> NotifyController {
        let (signal_sender, mut signal_receiver) = oneshot::channel();

        let (new_open_tx_register, mut new_open_tx_register_receiver) =
            mpsc::channel(REGISTER_CHANNEL_SIZE);
        let (new_open_tx_sender, mut new_open_tx_receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);

        let (commit_open_tx_register, mut commit_open_tx_register_receiver) =
            mpsc::channel(REGISTER_CHANNEL_SIZE);
        let (commit_open_tx_sender, mut commit_open_tx_receiver) =
            mpsc::channel(NOTIFY_CHANNEL_SIZE);

        let (interval_register, mut interval_register_receiver) =
            mpsc::channel(REGISTER_CHANNEL_SIZE);
        let (interval_sender, mut interval_receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);

        let (start_register, mut start_register_receiver) = mpsc::channel(REGISTER_CHANNEL_SIZE);
        let (start_sender, mut start_receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);

        let (stop_register, mut stop_register_receiver) = mpsc::channel(REGISTER_CHANNEL_SIZE);
        let (stop_sender, mut stop_receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);

        handle.spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut signal_receiver => {
                        break;
                    }
                    Some(msg) = new_open_tx_register_receiver.recv() => { self.handle_register_new_open_tx(msg) },
                    Some(msg) = new_open_tx_receiver.recv() => { self.handle_notify_new_open_tx(msg).await },
                    Some(msg) = commit_open_tx_register_receiver.recv() => { self.handle_register_commit_open_tx(msg) },
                    Some(msg) = commit_open_tx_receiver.recv() => { self.handle_notify_commit_open_tx(msg).await },
                    Some(msg) = interval_register_receiver.recv() => { self.handle_register_interval(msg) },
                    Some(msg) = interval_receiver.recv() => { self.handle_notify_interval(msg).await },
                    Some(msg) = start_register_receiver.recv() => { self.handle_register_start(msg) },
                    Some(()) = start_receiver.recv() => { self.handle_notify_start().await },
                    Some(msg) = stop_register_receiver.recv() => { self.handle_register_stop(msg) },
                    Some(()) = stop_receiver.recv() => { self.handle_notify_stop().await },
                    else => break,
                }
            }
        });

        NotifyController {
            new_open_tx_register,
            new_open_tx_notifier: new_open_tx_sender,
            commit_open_tx_register,
            commit_open_tx_notifier: commit_open_tx_sender,
            interval_register,
            interval_notifier: interval_sender,
            start_register,
            start_notifier: start_sender,
            stop_register,
            stop_notifier: stop_sender,
            stop: StopHandler::new(
                SignalSender::Tokio(signal_sender),
                None,
                "notify".to_string(),
            ),
            handle,
        }
    }

    fn handle_register_new_open_tx(&mut self, msg: Request<String, Receiver<OpenTransaction>>) {
        let Request {
            responder,
            arguments: name,
        } = msg;
        log::debug!("Register new_open_tx {:?}", name);
        let (sender, receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);
        self.new_open_tx_subscribers.insert(name, sender);
        let _ = responder.send(receiver);
    }

    async fn handle_notify_new_open_tx(&mut self, otx: OpenTransaction) {
        log::trace!("event new open tx {:?}", otx);
        // notify all subscribers
        for subscriber in self.new_open_tx_subscribers.values() {
            let _ = subscriber.send(otx.clone()).await;
        }
    }

    fn handle_register_commit_open_tx(&mut self, msg: Request<String, Receiver<Vec<H256>>>) {
        let Request {
            responder,
            arguments: name,
        } = msg;
        log::debug!("Register commit_open_tx {:?}", name);
        let (sender, receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);
        self.commit_open_tx_subscribers.insert(name, sender);
        let _ = responder.send(receiver);
    }

    async fn handle_notify_commit_open_tx(&mut self, otx_hashes: Vec<H256>) {
        log::trace!("event commit open tx {:?}", otx_hashes);
        // notify all subscribers
        for subscriber in self.commit_open_tx_subscribers.values() {
            let _ = subscriber.send(otx_hashes.clone()).await;
        }
    }

    fn handle_register_interval(&mut self, msg: Request<String, Receiver<u64>>) {
        let Request {
            responder,
            arguments: name,
        } = msg;
        log::debug!("Register interval event plugin: {:?}", name);
        let (sender, receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);
        self.interval_subscribers.insert(name, sender);
        let _ = responder.send(receiver);
    }

    async fn handle_notify_interval(&mut self, elapsed_secs: u64) {
        log::trace!("event interval");
        // notify all subscribers
        for subscriber in self.interval_subscribers.values() {
            let _ = subscriber.send(elapsed_secs).await;
        }
    }

    fn handle_register_start(&mut self, msg: Request<String, Receiver<()>>) {
        let Request {
            responder,
            arguments: name,
        } = msg;
        log::debug!("Register start {:?}", name);
        let (sender, receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);
        self.start_subscribers.insert(name, sender);
        let _ = responder.send(receiver);
    }

    async fn handle_notify_start(&mut self) {
        log::trace!("event start");
        // notify all subscribers
        for subscriber in self.start_subscribers.values() {
            let _ = subscriber.send(()).await;
        }
    }

    fn handle_register_stop(&mut self, msg: Request<String, Receiver<()>>) {
        let Request {
            responder,
            arguments: name,
        } = msg;
        log::debug!("Register stop {:?}", name);
        let (sender, receiver) = mpsc::channel(NOTIFY_CHANNEL_SIZE);
        self.stop_subscribers.insert(name, sender);
        let _ = responder.send(receiver);
    }

    async fn handle_notify_stop(&mut self) {
        log::trace!("event stop");
        // notify all subscribers
        for subscriber in self.stop_subscribers.values() {
            let _ = subscriber.send(()).await;
        }
    }
}

impl NotifyController {
    pub async fn subscribe_new_open_tx<S: ToString>(&self, name: S) -> Receiver<OpenTransaction> {
        Request::call(&self.new_open_tx_register, name.to_string())
            .await
            .expect("Subscribe new open tx should be OK")
    }

    pub fn notify_new_open_tx(&self, otx: OpenTransaction) {
        let new_open_tx_notifier = self.new_open_tx_notifier.clone();
        self.handle.spawn(async move {
            let _ = new_open_tx_notifier.send(otx).await;
        });
    }

    pub async fn subscribe_commit_open_tx<S: ToString>(&self, name: S) -> Receiver<Vec<H256>> {
        Request::call(&self.commit_open_tx_register, name.to_string())
            .await
            .expect("Subscribe commit open tx should be OK")
    }

    pub fn notify_commit_open_tx(&self, otx_hashes: Vec<H256>) {
        let commit_open_tx_notifier = self.commit_open_tx_notifier.clone();
        self.handle.spawn(async move {
            let _ = commit_open_tx_notifier.send(otx_hashes).await;
        });
    }

    pub async fn subscribe_interval<S: ToString>(&self, name: S) -> Receiver<u64> {
        Request::call(&self.interval_register, name.to_string())
            .await
            .expect("Subscribe interval should be OK")
    }

    pub fn notify_interval(&self, elapsed_secs: u64) {
        let interval_notifier = self.interval_notifier.clone();
        self.handle.spawn(async move {
            let _ = interval_notifier.send(elapsed_secs).await;
        });
    }

    pub async fn subscribe_start<S: ToString>(&self, name: S) -> Receiver<()> {
        Request::call(&self.start_register, name.to_string())
            .await
            .expect("Subscribe start should be OK")
    }

    pub fn notify_start(&self) {
        let start_notifier = self.start_notifier.clone();
        self.handle.spawn(async move {
            let _ = start_notifier.send(()).await;
        });
    }

    pub async fn subscribe_stop<S: ToString>(&self, name: S) -> Receiver<()> {
        Request::call(&self.stop_register, name.to_string())
            .await
            .expect("Subscribe stop should be OK")
    }

    pub fn notify_stop(&self) {
        let stop_notifier = self.stop_notifier.clone();
        self.handle.spawn(async move {
            let _ = stop_notifier.send(()).await;
        });
    }
}
