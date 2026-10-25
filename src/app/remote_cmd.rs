//! Remote windows: connecting, with ssh's questions (password, unknown
//! host) asked in the footer.

use std::sync::Arc;

use super::*;
use crate::remote::{Address, Connecting, askpass};

impl App {
    /// `L` (or the command line) with a remote address: connects in the
    /// background; the window changes when the connection is made.
    pub(crate) fn connect(&mut self, address: Address) -> Result<(), String> {
        let run = platform::run_dir().map_err(|e| format!("Cannot connect: {e}"))?;
        self.connections += 1;
        let connecting = Connecting::start(address, &run, self.connections)
            .map_err(|e| format!("Cannot connect: {e}"))?;
        self.message = Some(format!(
            "Connecting to {}... (Esc cancels)",
            connecting.address.destination()
        ));
        self.connecting = Some((self.active, connecting));
        Ok(())
    }

    /// Called on every pass of the main loop while connecting: asks ssh's questions and
    /// opens the remote window when the connection is made.
    pub fn poll_connection(&mut self) {
        let Some((_, connecting)) = &mut self.connecting else {
            return;
        };
        let Some(result) = connecting.poll() else {
            let question = connecting.askpass.question();
            let asking = matches!(
                self.prompt.as_ref().map(|p| &p.kind),
                Some(PromptKind::Askpass { .. })
            );
            if let Some(question) = question
                && !asking
                && self.prompt.is_none()
                && self.confirm.is_none()
            {
                let secret = askpass::is_secret(&question);
                self.open_prompt(
                    PromptKind::Askpass { secret },
                    askpass::one_line(&question),
                    "",
                );
            }
            return;
        };
        let (pane, connecting) = self.connecting.take().unwrap();
        let destination = connecting.address.destination();
        if matches!(
            self.prompt.as_ref().map(|p| &p.kind),
            Some(PromptKind::Askpass { .. })
        ) {
            self.prompt = None;
        }
        match result {
            Ok(remote) => {
                let old = &self.panes[pane];
                let (sort, display) = (old.sort, old.display);
                let mut new = Pane::from_remote(Arc::new(remote));
                if new.tree.show_hidden != self.options.show_hidden {
                    new.tree.show_hidden = self.options.show_hidden;
                    new.after_change();
                }
                new.display = display;
                self.panes[pane] = new;
                self.panes[pane].set_sort(sort);
                self.message = Some(format!("Connected to {destination}"));
            }
            Err(e) => self.message = Some(format!("Cannot connect to {destination}: {e}")),
        }
    }

    /// The answer to ssh's question (`None`: the user gave up).
    pub(crate) fn answer_askpass(&mut self, answer: Option<&str>) {
        if let Some((_, connecting)) = &self.connecting
            && connecting.askpass.answer(answer).is_err()
        {
            self.message = Some("ssh is no longer waiting for an answer".into());
        }
    }

    /// Esc while connecting: kills ssh.
    pub(crate) fn cancel_connection(&mut self) {
        if let Some((_, connecting)) = self.connecting.take() {
            self.message = Some(format!(
                "Connection to {} cancelled",
                connecting.address.destination()
            ));
        }
    }
}
