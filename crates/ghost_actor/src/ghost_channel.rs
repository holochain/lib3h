use crate::*;

pub trait GhostEndpoint<P: GhostProtocol> {
    fn send(&mut self, request_id: Option<RequestId>, message: P) -> GhostResult<()>;
    fn recv(&mut self) -> GhostResult<(Option<RequestId>, P)>;
}

pub struct GhostUnboundedEndpoint<P: GhostProtocol> {
    sender: crossbeam_channel::Sender<(Option<RequestId>, P)>,
    receiver: crossbeam_channel::Receiver<(Option<RequestId>, P)>,
}

impl<P: GhostProtocol> GhostEndpoint<P> for GhostUnboundedEndpoint<P> {
    fn send(&mut self, request_id: Option<RequestId>, message: P) -> GhostResult<()> {
        self.sender.send((request_id, message))?;
        Ok(())
    }

    fn recv(&mut self) -> GhostResult<(Option<RequestId>, P)> {
        Ok(self.receiver.try_recv()?)
    }
}
