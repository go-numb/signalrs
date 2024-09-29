use log::{error, info};
use serde::de::DeserializeOwned;
use serde_json::Error;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    sync::mpsc::{Receiver, Sender},
};

pub struct TcpClient<T>
where
    T: DeserializeOwned + Send + 'static,
{
    tx: Sender<Result<T, Error>>,
    addr: String,
}

impl<T> TcpClient<T>
where
    T: DeserializeOwned + Send + 'static,
{
    pub fn new(addr: String) -> (Self, Receiver<Result<T, Error>>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (TcpClient { tx, addr }, rx)
    }

    #[allow(unused)]
    pub fn connect(&self) {
        let addr = self.addr.clone();
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            info!("connect to {}", addr);
            let mut stream = match TcpStream::connect(addr) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to connect: {}", e);
                    return;
                }
            };
            let mut buffer = [0; 128];

            loop {
                match stream.read(&mut buffer) {
                    Ok(0) => {
                        info!("Client disconnected");
                        break;
                    }
                    Ok(size) => {
                        let data = &buffer[..size];
                        match serde_json::from_slice::<T>(data) {
                            Ok(t) => {
                                tx.send(Ok(t)).unwrap();
                            }
                            Err(e) => {
                                tx.send(Err(e)).unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from socket: {}", e);
                        break;
                    }
                }
            }
        });
    }

    pub fn received_server(&self) {
        let addr = self.addr.clone();
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            info!("Starting server on {}", addr);

            let listener = TcpListener::bind(&addr).expect("Could not bind");

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let tx = tx.clone();
                        std::thread::spawn(move || {
                            let mut buffer = [0; 128]; // バッファをスレッドごとに定義

                            loop {
                                match stream.read(&mut buffer) {
                                    Ok(0) => {
                                        info!("Client disconnected");
                                        break; // クライアント切断時にはループを抜ける
                                    }
                                    Ok(size) => {
                                        match serde_json::from_slice::<T>(&buffer[..size]) {
                                            Ok(t) => {
                                                match tx.send(Ok(t)) {
                                                    Ok(_) => {}
                                                    Err(e) => {
                                                        error!("Failed to send to channel, cause by blocked: {}", e);
                                                    }
                                                };
                                            }
                                            Err(e) => {
                                                error!("Failed to read from socket: {}", e);
                                                tx.send(Err(e)).unwrap();
                                            }
                                        }
                                        // バッファのクリア
                                        unsafe {
                                            std::ptr::write_bytes(
                                                buffer.as_mut_ptr(),
                                                0,
                                                buffer.len(),
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to read from socket: {}", e);
                                        break; // エラー時にはループを抜ける
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Connection failed: {}", e);
                    }
                }
            }
        });
    }
}
