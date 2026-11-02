pub enum AVCommand {
    Play,
    Pause,
    Repeat(bool),
    Seek(f64),
    Dispose,
}

pub enum StreamStateData {
    Ready { length: f64 },
    Playing { position: f64 },
    Buffering { position: f64 },
    Seeking {},
    Paused { position: f64 },
}
