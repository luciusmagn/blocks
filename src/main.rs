use barrage::{self, Receiver, Sender};
use ureq;
use ws::{self, util::Token, Handshake, Result as WsResult};

use std::{
    error::Error,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

struct Ticker {
    sender: ws::Sender,
    secs_sent: bool,
    receiver: Receiver<usize>,
}

impl ws::Handler for Ticker {
    fn on_open(&mut self, _: Handshake) -> WsResult<()> {
        println!("connected");
        self.sender.timeout(1000, Token(1))
    }

    fn on_timeout(&mut self, _: Token) -> WsResult<()> {
        if !self.secs_sent {
            let _ = self.sender.send(SECS.load(Ordering::Relaxed).to_string());
            self.secs_sent = true;
        }
        match self.receiver.try_recv() {
            Ok(Some(n)) => {
                if let e @ Err(_) = self.sender.send(n.to_string()) {
                    return e;
                }
            }
            Ok(None) => (),
            Err(_) => return Ok(()),
        };
        self.sender.timeout(1000, Token(1))
    }
}

static BLOCK_HEIGHT: AtomicUsize = AtomicUsize::new(0);
static SECS: AtomicUsize = AtomicUsize::new(0);
fn watcher(block_sender: Sender<usize>) {
    let mut block_height = 0;
    let mut secs = 0;

    loop {
        thread::sleep(Duration::from_millis(1000));

        let new_height = match ureq::get("https://blockchain.info/q/getblockcount").call() {
            Ok(v) => v
                .into_string()
                .unwrap_or_default()
                .parse()
                .unwrap_or_default(),
            Err(_) => continue,
        };

        if block_height != new_height {
            let _ = block_sender.send(new_height);
            println!("update block height");

            block_height = new_height;
            BLOCK_HEIGHT.store(new_height, Ordering::Relaxed);
            secs = 0;
            SECS.store(0, Ordering::Relaxed);
        } else {
            secs += 1;
            SECS.store(secs, Ordering::Relaxed);
        }
    }
}

vial::routes! {
    GET "/blocks" => |_| BLOCK_HEIGHT.load(Ordering::Relaxed).to_string();
    GET "/" => |_| Response::from_file("blocks-frontend/dist/index.html");
    GET "/blocks-frontend.js" => |_| Response::from_file("blocks-frontend/dist/blocks-frontend.js");
    GET "/blocks-frontend_bg.wasm" => |_| Response::from_file("blocks-frontend/dist/blocks-frontend_bg.wasm");
}

fn main() -> Result<(), Box<dyn Error>> {
    let (block_sender, block_receiver) = barrage::unbounded();

    thread::spawn(move || watcher(block_sender));
    thread::spawn(move || {
        ws::listen("0.0.0.0:3012", |out| Ticker {
            sender: out,
            secs_sent: false,
            receiver: block_receiver.clone(),
        })
    });

    vial::asset_dir!("static/");
    vial::run!()?;

    Ok(())
}
