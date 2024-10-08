use std::time::Duration;

use models::error;
use snafu::prelude::*;
use snafu::Location;

use models::error::Ffmpeg as Error;

use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    SinkExt,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::TcpListener,
    process::Child,
};

use crate::{FfmpegBuilder, Parameter};

type Result<T> = std::result::Result<T, Error>;

/// A running instance of ffmpeg.
#[derive(Debug)]
pub struct Ffmpeg {
    /// The stream of progress events emitted by ffmpeg.
    pub progress: UnboundedReceiver<Result<Progress>>,
    /// The actual ffmpeg process.
    pub process: Child,
}

/// A progress event emitted by ffmpeg.
///
/// Names of the fields directly correspond to the names in the output of ffmpeg's `-progress`.  
/// Everything is wrapped in an option because this has no docs I can find, so I can't guarantee
/// that they will all be in the data ffmpeg sends.
/// Note that bitrate is ignored because I'm not sure of the exact format it's in. Blame ffmpeg.  
#[derive(Debug, Default)]
pub struct Progress {
    /// What frame ffmpeg is on.
    pub frame: Option<u64>,
    /// What framerate ffmpeg is processing at.
    pub fps: Option<f64>,
    /// How much data ffmpeg has output so far, in bytes.
    pub total_size: Option<u64>,
    /// How far ffmpeg has processed.
    pub out_time: Option<Duration>,
    /// How many frames were duplicated? The meaning is unclear.
    pub dup_frames: Option<u64>,
    /// How many frames were dropped.
    pub drop_frames: Option<u64>,
    /// How fast it is processing, relative to 1x playback speed.
    pub speed: Option<f64>,
    /// What ffmpeg will do now.
    pub status: Status,
}

/// What ffmpeg is going to do next.
#[derive(Debug)]
pub enum Status {
    /// Ffmpeg will continue emitting progress events.
    Continue,
    /// Ffmpeg has finished processing.
    ///
    /// After emitting this, the stream will end.
    End,
}

impl Default for Status {
    fn default() -> Self {
        Self::Continue
    }
}

impl<'a> FfmpegBuilder<'a> {
    /// Spawns a new ffmpeg process and records the output, consuming the builder
    ///
    /// This has to consume the builder for stdin, etc to work
    pub async fn run(mut self) -> Result<Ffmpeg> {
        let listener = TcpListener::bind("127.0.0.1:0").await.context(error::FfIoSnafu)?;
        let port = listener.local_addr().context(error::FfIoSnafu)?.port();
        let prog_url = format!("tcp://127.0.0.1:{}", port);

        self = self.option(Parameter::key_value("progress", &prog_url));
        let mut command = self.to_command();
        dbg!(&command);
        let mut child = command.spawn().context(error::FfIoSnafu)?;

        let conn = listener.accept();
        let status = child.wait();

        let conn = tokio::select! {
            conn = conn => {
                conn.context(error::FfIoSnafu)?.0
            }
            s = status => {
                let b = s.unwrap();
                return Err(Error::Exit { status: b, location: snafu::location!() })
            }
        };

        let (mut tx, rx) = mpsc::unbounded();

        tokio::spawn(async move {
            let mut reader = BufReader::new(conn);
            let mut progress: Progress = Default::default();

            loop {
                let mut line = String::new();
                let read = reader.read_line(&mut line).await;
                match read {
                    Ok(n) => {
                        if n == 0 {
                            tx.close_channel();
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e).context(error::FfIoSnafu)).await;
                        tx.close_channel();
                    }
                }

                if let Some((key, value)) = parse_line(&line) {
                    match key {
                        "frame" => match value.parse() {
                            Ok(x) => progress.frame = Some(x),
                            Err(e) => handle_parse_error(&mut tx, e, value).await,
                        },
                        "fps" => match value.parse() {
                            Ok(x) => progress.fps = Some(x),
                            Err(e) => handle_parse_error(&mut tx, e, value).await,
                        },
                        // TOOD: bitrate
                        "total_size" => match value.parse() {
                            Ok(x) => progress.total_size = Some(x),
                            Err(e) => handle_parse_error(&mut tx, e, value).await,
                        },
                        "out_time_us" => match value.parse() {
                            Ok(us) => progress.out_time = Some(Duration::from_micros(us)),
                            Err(e) => handle_parse_error(&mut tx, e, value).await,
                        },
                        "dup_frames" => match value.parse() {
                            Ok(x) => progress.dup_frames = Some(x),
                            Err(e) => handle_parse_error(&mut tx, e, value).await,
                        },
                        "drop_frames" => match value.parse() {
                            Ok(x) => progress.drop_frames = Some(x),
                            Err(e) => handle_parse_error(&mut tx, e, value).await,
                        },
                        "speed" => {
                            let num = &value[..(value.len() - 1)];
                            match num.parse() {
                                Ok(x) => progress.speed = Some(x),
                                Err(e) => handle_parse_error(&mut tx, e, num).await,
                            }
                        }
                        "progress" => {
                            progress.status = match value {
                                "continue" => Status::Continue,
                                "end" => Status::End,
                                x => {
                                    // This causes feeding the next thing to error
                                    // However, we don't care
                                    // We just ignore the error
                                    let _ = tx.feed(Err(Error::UnknownStatus { status: x.to_owned() }));
                                    tx.close_channel();

                                    // Just give it a status so it compiles
                                    Status::End
                                }
                            };
                            match tx.feed(Ok(progress)).await {
                                Ok(_) => {}
                                Err(e) => {
                                    if e.is_disconnected() {
                                        tx.close_channel();
                                    }
                                }
                            }
                            progress = Default::default();
                        }
                        _ => {}
                    }
                } else {
                    let _ = tx.send(Err(Error::KeyValueParse { key: line }));
                    tx.close_channel();
                }
            }
        });

        Ok(Ffmpeg {
            progress: rx,
            process: child,
        })
    }
}

fn parse_line<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
    let trimmed = line.trim();
    let mut iter = trimmed.splitn(2, '=');

    let mut key = iter.next()?;
    key = key.trim_end();

    let mut value = iter.next()?;
    // Ffmpeg was putting in random spaces for some reason?
    value = value.trim_start();

    Some((key, value))
}

async fn handle_parse_error(
    tx: &mut UnboundedSender<Result<Progress>>,
    e: impl std::error::Error + Send + 'static,
    x: &str,
) {
    // Ignore the error because we're closing the channel anyway
    let _ = tx
        .send(Err(Error::OtherParse { source: Box::new(e), msg: x.to_owned() }))
        .await;
    tx.close_channel();
}