use crate::{Error, Ingest, Ledger, Mutation, Observation, Outcome, Result};
use std::{
    sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    thread::{self, JoinHandle},
};

const CONTROL_CAPACITY: usize = 128;
const OBSERVATION_CAPACITY: usize = 64;

enum Job {
    Apply(Mutation, SyncSender<Result<Outcome>>),
    Ingest(Observation, SyncSender<Result<Ingest>>),
    S(
        Box<crate::s::SRequest>,
        SyncSender<Result<crate::s::SOutcome>>,
    ),
    Take(
        crate::s::DurableCommitReceipt,
        agentguard_typed_authorization::PeerBinding,
        SyncSender<Result<crate::s::SResponse>>,
    ),
}

#[derive(Clone)]
pub struct Client {
    control: SyncSender<Job>,
    observations: SyncSender<Job>,
    wake: SyncSender<()>,
}

impl Client {
    pub fn submit_s(
        &self,
        request: crate::s::SRequest,
    ) -> Result<Receiver<Result<crate::s::SOutcome>>> {
        let (send, receive) = sync_channel(1);
        self.enqueue(&self.control, Job::S(Box::new(request), send))?;
        Ok(receive)
    }
    pub fn take_s(
        &self,
        receipt: crate::s::DurableCommitReceipt,
        peer: agentguard_typed_authorization::PeerBinding,
    ) -> Result<Receiver<Result<crate::s::SResponse>>> {
        let (send, receive) = sync_channel(1);
        self.enqueue(&self.control, Job::Take(receipt, peer, send))?;
        Ok(receive)
    }
    fn enqueue(&self, channel: &SyncSender<Job>, job: Job) -> Result<()> {
        channel.try_send(job).map_err(|error| match error {
            TrySendError::Full(_) => Error::QueueFull,
            TrySendError::Disconnected(_) => Error::Fenced,
        })?;
        let _ = self.wake.try_send(());
        Ok(())
    }

    pub fn submit(&self, mutation: Mutation) -> Result<Receiver<Result<Outcome>>> {
        let (send, receive) = sync_channel(1);
        self.enqueue(&self.control, Job::Apply(mutation, send))?;
        Ok(receive)
    }

    pub fn submit_observation(&self, observation: Observation) -> Result<Receiver<Result<Ingest>>> {
        let (send, receive) = sync_channel(1);
        self.enqueue(&self.observations, Job::Ingest(observation, send))?;
        Ok(receive)
    }
}

pub struct Writer {
    client: Client,
    thread: JoinHandle<()>,
}

impl Writer {
    pub fn start(mut ledger: Ledger) -> Result<Self> {
        let (control, controls) = sync_channel(CONTROL_CAPACITY);
        let (observations, samples) = sync_channel(OBSERVATION_CAPACITY);
        let (wake, wakes) = sync_channel(1);
        let thread = thread::Builder::new()
            .name("i05-synthetic-writer".into())
            .spawn(move || {
                loop {
                    let Ok(job) = controls.try_recv().or_else(|_| samples.try_recv()) else {
                        if wakes.recv().is_err() {
                            break;
                        }
                        continue;
                    };
                    match job {
                        Job::Apply(mutation, response) => {
                            let _ = response.send(ledger.apply(mutation));
                        }
                        Job::Ingest(observation, response) => {
                            let _ = response.send(ledger.ingest(observation));
                        }
                        Job::S(request, response) => {
                            if response.send(ledger.apply_s(*request)).is_err()
                                && let Some(runtime) = ledger.s.as_mut()
                            {
                                runtime.reap_abandoned_slots();
                            }
                        }
                        Job::Take(receipt, peer, response) => {
                            let _ = response.send(ledger.take_s(receipt, peer));
                        }
                    }
                }
            })
            .map_err(|_| Error::Storage)?;
        Ok(Self {
            client: Client {
                control,
                observations,
                wake,
            },
            thread,
        })
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    /// The caller releases its cloned clients before joining.
    pub fn join(self) -> Result<()> {
        drop(self.client);
        self.thread.join().map_err(|_| Error::UnknownCommit)
    }
}
