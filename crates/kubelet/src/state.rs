//! Used to define a state machine.
//!
//! Example Pod state machine:
//! ```
//! use kubelet::state::prelude::*;
//! use kubelet::pod::Pod;
//!
//! #[derive(Debug, TransitionTo)]
//! #[transition_to(TestState)]
//! struct TestState;
//!
//! // Example of manual trait implementation
//! // impl TransitionTo<TestState> for TestState {}
//!
//! struct PodState;
//!
//! impl ResourceState for PodState {
//!     type Manifest = Pod;
//! }
//!
//! #[async_trait::async_trait]
//! impl State<PodState> for TestState {
//!     async fn next(
//!         self: Box<Self>,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> Transition<PodState> {
//!         Transition::next(self, TestState)
//!     }
//!
//!     async fn json_status(
//!         &self,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<serde_json::Value> {
//!         Ok(serde_json::json!(null))
//!     }
//! }
//! ```
//!
//! The next state must also be State<PodState>, if it is not State, it fails to compile:
//! ```compile_fail
//! use kubelet::state::{Transition, State, TransitionTo};
//! use kubelet::pod::Pod;
//!
//! #[derive(Debug, TransitionTo)]
//! #[transition_to(NotState)]
//! struct TestState;
//!
//! struct PodState;
//!
//! #[derive(Debug)]
//! struct NotState;
//!
//! #[async_trait::async_trait]
//! impl State<PodState> for TestState {
//!     async fn next(
//!         self: Box<Self>,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<Transition<PodState>> {
//!         // This fails because NotState is not State
//!         Ok(Transition::next(self, NotState))
//!     }
//!
//!     async fn json_status(
//!         &self,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<serde_json::Value> {
//!         Ok(serde_json::json!(null))
//!     }
//! }
//! ```
//!
//! Edges must be defined, even for self-transition, with edge removed, compilation fails:
//! ```compile_fail
//! use kubelet::state::{Transition, State};
//! use kubelet::pod::Pod;
//!
//! #[derive(Debug)]
//! struct TestState;
//!
//! // impl TransitionTo<TestState> for TestState {}
//!
//! struct PodState;
//!
//! #[async_trait::async_trait]
//! impl State<PodState> for TestState {
//!     async fn next(
//!         self: Box<Self>,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<Transition<PodState>> {
//!         // This fails because TestState is not TransitionTo<TestState>
//!         Ok(Transition::next(self, TestState))
//!     }
//!
//!     async fn json_status(
//!         &self,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<serde_json::Value> {
//!         Ok(serde_json::json!(null))
//!     }
//! }
//! ```
//!
//! The next state must have the same PodState type, otherwise compilation will fail:
//! ```compile_fail
//! use kubelet::state::{Transition, State, TransitionTo};
//! use kubelet::pod::Pod;
//!
//! #[derive(Debug, TransitionTo)]
//! #[transition_to(OtherState)]
//! struct TestState;
//!
//! struct PodState;
//!
//! #[derive(Debug)]
//! struct OtherState;
//!
//! struct OtherPodState;
//!
//! #[async_trait::async_trait]
//! impl State<PodState> for TestState {
//!     async fn next(
//!         self: Box<Self>,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<Transition<PodState>> {
//!         // This fails because OtherState is State<OtherPodState>
//!         Ok(Transition::next(self, OtherState))
//!     }
//!
//!     async fn json_status(
//!         &self,
//!         _pod_state: &mut PodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<serde_json::Value> {
//!         Ok(serde_json::json!(null))
//!     }
//! }
//!
//! #[async_trait::async_trait]
//! impl State<OtherPodState> for OtherState {
//!     async fn next(
//!         self: Box<Self>,
//!         _pod_state: &mut OtherPodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<Transition<OtherPodState>> {
//!         Ok(Transition::Complete(Ok(())))
//!     }
//!
//!     async fn json_status(
//!         &self,
//!         _pod_state: &mut OtherPodState,
//!         _pod: &Pod,
//!     ) -> anyhow::Result<serde_json::Value> {
//!         Ok(serde_json::json!(null))
//!     }
//! }
//! ```

pub mod prelude;

#[cfg(feature = "derive")]
#[doc(hidden)]
pub use kubelet_derive::*;

/// Holds arbitrary State objects in Box, and prevents manual construction of Transition::Next
///
/// ```compile_fail
/// use kubelet::state::{Transition, StateHolder, Stub};
///
/// struct PodState;
///
/// // This fails because `state` is a private field. Use Transition::next classmethod instead.
/// let _transition = Transition::<PodState>::Next(StateHolder {
///     state: Box::new(Stub),
/// });
/// ```
pub struct StateHolder<S: ResourceState> {
    // This is private, preventing manual construction of Transition::Next
    pub(crate) state: Box<dyn State<S>>,
}

/// Represents result of state execution and which state to transition to next.
pub enum Transition<S: ResourceState> {
    /// Transition to new state.
    Next(StateHolder<S>),
    /// Stop executing the state machine and report the result of the execution.
    Complete(anyhow::Result<()>),
}

/// Mark an edge exists between two states.
pub trait TransitionTo<S> {}

impl<S: ResourceState> Transition<S> {
    // This prevents user from having to box everything AND allows us to enforce edge constraint.
    /// Construct Transition::Next from old state and new state. Both states must be State<PodState>
    /// with matching PodState. Input state must implement TransitionTo<OutputState>, which can be
    /// done manually or with the `TransitionTo` derive macro (requires the `derive` feature to be
    /// enabled)
    #[allow(clippy::boxed_local)]
    pub fn next<I: State<S>, O: State<S>>(_i: Box<I>, o: O) -> Transition<S>
    where
        I: TransitionTo<O>,
    {
        Transition::Next(StateHolder { state: Box::new(o) })
    }
}

#[async_trait::async_trait]
/// Allow for asynchronous cleanup up of PodState.
pub trait AsyncDrop: Sized {
    /// Clean up PodState.
    async fn async_drop(self);
}

/// Defines a type which represents a state for a given resource which is passed between its
/// state handlers.
pub trait ResourceState {
    /// The manifest / definition of the resource. Pod, Container, etc.
    type Manifest;
}

#[async_trait::async_trait]
/// A trait representing a node in the state graph.
pub trait State<S: ResourceState>: Sync + Send + 'static + std::fmt::Debug {
    /// Provider supplies method to be executed when in this state.
    async fn next(self: Box<Self>, state: &mut S, manifest: &S::Manifest) -> Transition<S>;

    /// Provider supplies JSON status patch to apply when entering this state.
    async fn json_status(
        &self,
        pod_state: &mut S,
        pod: &S::Manifest,
    ) -> anyhow::Result<serde_json::Value>;
}
