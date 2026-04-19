pub enum InstallMessage {
    Log(String),
    Progress,
    ProgressToEnd,
    Done,
    Error(String),
}

pub struct Reporter {
    tx: std::sync::mpsc::Sender<InstallMessage>,
    notice: nwg::NoticeSender,
}

impl Reporter {
    pub fn new(tx: std::sync::mpsc::Sender<InstallMessage>, notice: nwg::NoticeSender) -> Self {
        Self { tx, notice }
    }

    pub fn log(&self, msg: String) {
        let _ = self.tx.send(InstallMessage::Log(msg));
        self.notice.notice();
    }
    pub fn progress(&self) {
        let _ = self.tx.send(InstallMessage::Progress);
        self.notice.notice();
    }
    pub fn progress_to_end(&self) {
        let _ = self.tx.send(InstallMessage::ProgressToEnd);
        self.notice.notice();
    }
}
