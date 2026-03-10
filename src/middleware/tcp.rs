use log::{error, info};
use serde::de::DeserializeOwned;
use serde_json::Error;
use std::{
    io::BufRead,
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
            let stream = match TcpStream::connect(addr) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to connect: {}", e);
                    return;
                }
            };

            let reader = std::io::BufReader::new(&stream);
            for line in reader.lines() {
                match line {
                    Ok(line) if line.is_empty() => continue,
                    Ok(line) => {
                        match serde_json::from_str::<T>(&line) {
                            Ok(t) => {
                                tx.send(Ok(t)).unwrap();
                            }
                            Err(e) => {
                                error!("Failed to parse JSON: {}", e);
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
                    Ok(stream) => {
                        let tx = tx.clone();
                        std::thread::spawn(move || {
                            let reader = std::io::BufReader::new(&stream);
                            for line in reader.lines() {
                                match line {
                                    Ok(line) if line.is_empty() => continue,
                                    Ok(line) => {
                                        match serde_json::from_str::<T>(&line) {
                                            Ok(t) => {
                                                tx.send(Ok(t)).unwrap();
                                            }
                                            Err(e) => {
                                                error!("Failed to parse JSON: {}", e);
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
                    Err(e) => {
                        error!("Connection failed: {}", e);
                    }
                }
            }
        });
    }
}
