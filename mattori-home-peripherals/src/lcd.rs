use std::{array, sync::mpsc, thread::sleep, time::Duration};

use crate::I2cError;
use rppal::i2c::I2c;
use thiserror::Error;
use tokio::{
    sync::watch,
    task::{spawn_blocking, JoinHandle},
};

const LCD_SLAVE_ADDR: u16 = 0x3e;

#[derive(Debug, Clone)]
enum LcdMessage {
    Char(u8),
    Cmd(u8, u8),
    Wait(Duration),
    Stop,
}

#[derive(Error, Clone, Debug)]
pub enum LcdError {
    #[error(transparent)]
    I2cError(#[from] I2cError),
    #[error("Could not send message to lcd thread")]
    Send,
    #[error("Could not wait for lcd thread to stop")]
    ThreadWait,
    #[error("Could not wait for processing notification")]
    ProcessingWait,
}

pub type Result<T> = std::result::Result<T, LcdError>;

#[derive(Debug)]
pub struct Lcd {
    col: u8,
    row: u8,
    write_handle: JoinHandle<()>,
    write_sender: mpsc::Sender<LcdMessage>,
    processing_receiver: watch::Receiver<bool>,
}

impl Lcd {
    const INIT_SEQ: [LcdMessage; 9] = [
        LcdMessage::Cmd(0, 0x38),
        LcdMessage::Cmd(0, 0x39),
        LcdMessage::Cmd(0, 0x14),
        LcdMessage::Cmd(0, 0x70),
        LcdMessage::Cmd(0, 0x56),
        LcdMessage::Cmd(0, 0x6c),
        LcdMessage::Wait(Duration::from_millis(250)),
        LcdMessage::Cmd(0, 0x38),
        LcdMessage::Cmd(0, 0x0c),
    ];

    pub fn new(slave_addr: u16) -> Result<Lcd> {
        let mut i2c = I2c::new().map_err(|_| I2cError::Initialization)?;
        i2c.set_slave_address(slave_addr)
            .map_err(|_| I2cError::SlaveAddr(slave_addr))?;
        let (write_sender, write_receiver) = mpsc::channel();
        let (processing_sender, processing_receiver) = watch::channel(false);
        let write_handle = {
            spawn_blocking(move || {
                info!("starting lcd messaging thread, slave addr {}", slave_addr);
                loop {
                    let next_msg = match write_receiver.try_recv() {
                        Ok(msg) => {
                            trace!("next message was already queued");
                            msg
                        }
                        Err(e) => {
                            trace!("no message queued");
                            // notify if no message in queue
                            if let Err(e) = processing_sender.send(false) {
                                error!("error in lcd messaging thread while trying to set processing status to false: {}", e);
                                break;
                            }
                            match e {
                                mpsc::TryRecvError::Disconnected => {
                                    info!("lcd messaging channel disconnected");
                                    break;
                                }
                                mpsc::TryRecvError::Empty => match write_receiver.recv() {
                                    Ok(msg) => msg,
                                    Err(_) => {
                                        info!("lcd messaging channel had no more messages");
                                        break;
                                    }
                                },
                            }
                        }
                    };
                    if let Err(e) = processing_sender.send(true) {
                        error!("error in lcd messaging thread while trying to set processing status to false: {}", e);
                        break;
                    }
                    match next_msg {
                        LcdMessage::Char(c) => {
                            trace!("writing char {} to lcd", c);
                            i2c.write(&[0x40, c]).map_err(|_| LcdError::Send).unwrap();
                        }
                        LcdMessage::Cmd(ctrl, data) => {
                            trace!("writing cmd {} with data {} to lcd", ctrl, data);
                            i2c.write(&[ctrl, data])
                                .map_err(|_| LcdError::Send)
                                .unwrap();
                        }
                        LcdMessage::Wait(duration) => {
                            trace!("sleeping lcd messaging thread for {:?}", duration);
                            sleep(duration)
                        }
                        LcdMessage::Stop => {
                            trace!("stopping lcd messaging thread");
                            break;
                        }
                    };
                }
                info!("lcd messaging thread stopping");
            })
        };
        let mut lcd = Lcd {
            col: 0,
            row: 1,
            write_handle,
            write_sender,
            processing_receiver,
        };
        lcd.init()?;
        Ok(lcd)
    }

    pub fn default_addr() -> Result<Self> {
        Self::new(LCD_SLAVE_ADDR)
    }

    pub fn init(&mut self) -> Result<()> {
        trace!("initializing lcd");
        array::IntoIter::new(Lcd::INIT_SEQ)
            .try_for_each(|msg| self.write_sender.send(msg))
            .map_err(|_| LcdError::Send)?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        trace!("clearing lcd");
        self.write_sender
            .send(LcdMessage::Cmd(0, 0x01))
            .map_err(|_| LcdError::Send)?;
        self.write_sender
            .send(LcdMessage::Wait(Duration::from_millis(2)))
            .map_err(|_| LcdError::Send)?;
        Ok(())
    }

    pub fn first_line_head(&mut self) -> Result<()> {
        trace!("moving to head of first line of lcd");
        self.col = 0;
        self.row = 1;
        self.write_sender
            .send(LcdMessage::Cmd(0, 0x2))
            .map_err(|_| LcdError::Send)?;
        self.write_sender
            .send(LcdMessage::Wait(Duration::from_millis(2)))
            .map_err(|_| LcdError::Send)?;
        Ok(())
    }

    pub fn second_line_head(&mut self) -> Result<()> {
        trace!("moving to head of second line of lcd");
        self.col = 0;
        self.row = 2;
        self.write_sender
            .send(LcdMessage::Cmd(0, 0xc0))
            .map_err(|_| LcdError::Send)?;
        self.write_sender
            .send(LcdMessage::Wait(Duration::from_millis(2)))
            .map_err(|_| LcdError::Send)?;
        Ok(())
    }

    pub fn push_char(&mut self, char: u8) -> Result<()> {
        trace!("pushing char {} to lcd messaging thread", char);
        self.col += 1;
        if self.col > 8 {
            if self.row == 2 {
                trace!("at end of second line of lcd");
                self.first_line_head()?;
            } else {
                trace!("at end of first line of lcd");
                self.second_line_head()?;
            }
        }
        self.write_sender
            .send(LcdMessage::Char(char))
            .map_err(|_| LcdError::Send)?;
        self.write_sender
            .send(LcdMessage::Wait(Duration::from_micros(50)))
            .map_err(|_| LcdError::Send)?;
        Ok(())
    }

    pub fn push_str(&mut self, s: &str) -> Result<()> {
        s.bytes().try_for_each(|c| self.push_char(c))
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        trace!("shutting down lcd");
        self.clear()?;
        self.write_sender
            .send(LcdMessage::Stop)
            .map_err(|_| LcdError::Send)?;
        (&mut self.write_handle)
            .await
            .map_err(|_| LcdError::ThreadWait)?;
        Ok(())
    }

    pub fn is_write_processing(&self) -> bool {
        *self.processing_receiver.borrow()
    }

    pub async fn wait_for_processing(&mut self) -> Result<()> {
        if self.is_write_processing() {
            self.processing_receiver
                .changed()
                .await
                .map_err(|_| LcdError::ProcessingWait)?;
        }
        Ok(())
    }
}
