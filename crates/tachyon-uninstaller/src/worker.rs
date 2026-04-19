pub enum UninstallMessage {
    Log(String),
    Progress,
    ProgressToEnd,
    Done,
    Error(String),
}

pub struct Reporter {
    tx: std::sync::mpsc::Sender<UninstallMessage>,
    notice: nwg::NoticeSender,
}

impl Reporter {
    pub fn new(tx: std::sync::mpsc::Sender<UninstallMessage>, notice: nwg::NoticeSender) -> Self {
        Self { tx, notice }
    }

    pub fn log(&self, msg: String) {
        let _ = self.tx.send(UninstallMessage::Log(msg));
        self.notice.notice();
    }
    pub fn progress(&self) {
        let _ = self.tx.send(UninstallMessage::Progress);
        self.notice.notice();
    }
    pub fn progress_to_end(&self) {
        let _ = self.tx.send(UninstallMessage::ProgressToEnd);
        self.notice.notice();
    }
}
