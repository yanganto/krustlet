use crate::PodState;
use kubelet::pod::state::prelude::*;

/// Pod was deleted.
#[derive(Default, Debug)]
pub struct Completed;

#[async_trait::async_trait]
impl State<PodState, PodStatus> for Completed {
    async fn next(self: Box<Self>, _pod_state: &mut PodState, _pod: &Pod) -> Transition<PodState> {
        Transition::Complete(Ok(()))
    }

    async fn json_status(
        &self,
        _pod_state: &mut PodState,
        _pod: &Pod,
    ) -> anyhow::Result<PodStatus> {
        Ok(make_status(Phase::Succeeded, "Completed"))
    }
}
