use crate::prelude::v1::*;

use std::{
	fmt,
	sync::Arc,
	error::Error
};

use async_trait::async_trait;
use tokio::sync::RwLock;
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FsmError {
	NoTransition,
	Interrupted
}

impl fmt::Display for FsmError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self)
	}
}

impl Error for FsmError {}

#[derive(Error, Clone, Debug)]
#[error(transparent)]
pub struct FsmTransitionError(#[from] pub Arc<anyhow::Error>);

impl From<FsmError> for FsmTransitionError {
	fn from(err: FsmError) -> Self {
		FsmTransitionError(Arc::new(err.into()))
	}
}

pub type FsmTransitionResult<T, E = FsmTransitionError> = std::result::Result<T, E>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FsmQueueStatus {
	Empty,
	MoreEventsQueued
}

pub trait FsmEvent {

}
pub trait FsmEvents<F: Fsm>: Send + Sync {
	fn new_no_event() -> Self;
	fn new_error_event(error: FsmTransitionError) -> Self;
}

#[async_trait]
pub trait FsmState<F: Fsm> {
	async fn on_entry(&self, _event_context: &EventContext<'_, F>) -> FsmTransitionResult<()> { Ok(()) }
	async fn on_exit(&self, _event_context: &EventContext<'_, F>) -> FsmTransitionResult<()> { Ok(()) }
}

#[async_trait]
pub trait FsmInspect<F: Fsm> {
	fn new_from_context(context: &FsmArc<F::C>) -> Self;

	async fn on_state_entry(&self, _state: &F::S, _event_context: &EventContext<'_, F>) { }
	async fn on_state_exit(&self, _state: &F::S, _event_context: &EventContext<'_, F>) { }
	async fn on_action(&self, _state: &F::S, _event_context: &EventContext<'_, F>) { }
	async fn on_transition(&self, _source_state: &F::S, _target_state: &F::S, _event_context: &EventContext<'_, F>) { }
	async fn on_no_transition(&self, _current_state: &F::S, _event_context: &EventContext<'_, F>) { }
}

#[derive(Default)]
pub struct FsmInspectNull<F: Fsm> {
	_fsm_ty: PhantomData<F>
}

impl<F: Fsm> FsmInspect<F> for FsmInspectNull<F> {
	fn new_from_context(_context: &FsmArc<F::C>) -> Self {
		FsmInspectNull {
			_fsm_ty: PhantomData
		}
	}
}

/*
impl<F: Fsm> FsmInspectNull<F> {
	pub fn new(context: &F::C) -> Self {
		FsmInspectNull {
			_fsm_ty: PhantomData
		}
	}
}
*/

/*
impl<F: Fsm> FsmInspect<F> for FsmInspectNull<F> {
	fn on_state_entry(&self, state: &F::S, event_context: &EventContext<F>) { }
	fn on_state_exit(&self, state: &F::S, event_context: &EventContext<F>) { }
	fn on_action(&self, state: &F::S, event_context: &EventContext<F>) { }
	fn on_transition(&self, source_state: &F::S, target_state: &F::S, event_context: &EventContext<F>) { }
	fn on_no_transition(&self, source_state: &F::S, target_state: &F::S) { }
}
*/

// just for the InitialState definition type
impl<F, A, B> FsmState<F> for (A, B) where F: Fsm, A: FsmState<F>, B: FsmState<F> { }
impl<F, A, B, C> FsmState<F> for (A, B, C) where F: Fsm, A: FsmState<F>, B: FsmState<F>, C: FsmState<F> { }
impl<F, A, B, C, D> FsmState<F> for (A, B, C, D) where F: Fsm, A: FsmState<F>, B: FsmState<F>, C: FsmState<F>, D: FsmState<F> { }
impl<F, A, B, C, D, E> FsmState<F> for (A, B, C, D, E) where F: Fsm, A: FsmState<F>, B: FsmState<F>, C: FsmState<F>, D: FsmState<F>, E: FsmState<F> { }

/*
// prevent usage in production, satisfy the compiler
impl<A, B> FsmStateFactory for (A, B) {
	fn new_state<C>(parent_context: &C) -> Self {
		panic!("Not supported for tuple types, just as a helper!");
	}
}
*/


/*
pub trait FsmStateSubMachineTransition<F: Fsm> {
	fn on_entry_internal(&mut self) { }
	fn on_exit_internal(&mut self) { }
}
*/


pub trait FsmStateFactory<Context> {
	fn new_state(parent_context: &FsmArc<Context>) -> Self;
}

impl<S: Default, Context> FsmStateFactory<Context> for S {
	fn new_state(_parent_context: &FsmArc<Context>) -> Self {
		Default::default()
	}
}

pub trait FsmGuard<F: Fsm> {
	fn guard(event_context: &EventContext<F>, states: &F::SS) -> bool;
}

pub struct NoGuard;
impl<F: Fsm> FsmGuard<F> for NoGuard {
	#[inline]
	fn guard(_event_context: &EventContext<F>, _states: &F::SS) -> bool {
		true
	}
}


#[async_trait]
pub trait FsmAction<F: Fsm, S, T> {
	async fn action(event_context: &EventContext<'_, F>, source_state: &S, target_state: &T);
}

#[async_trait]
pub trait FsmActionSelf<F: Fsm, S> {
	async fn action(event_context: &EventContext<'_, F>, state: &S);
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct NoEvent;
impl FsmEvent for NoEvent { }

#[derive(Debug, Clone)]
pub struct FsmErrorEvent(pub FsmTransitionError);
impl FsmEvent for FsmErrorEvent {}

pub struct NoAction;
#[async_trait]
impl<F: Fsm, S: Send + Sync, T: Send + Sync> FsmAction<F, S, T> for NoAction {
	#[inline]
	async fn action(_event_context: &EventContext<'_, F>, _source_state: &S, _target_state: &T) { }
}
#[async_trait]
impl<F: Fsm, S: Send + Sync> FsmActionSelf<F, S> for NoAction {
	#[inline]
	async fn action(_event_context: &EventContext<'_, F>, _state: &S) { }
}


pub struct EventContext<'a, F: Fsm + 'a> {
	pub event: &'a F::E,
	pub queue: FsmArc<dyn FsmEventQueue<F>>,
	pub context: FsmArc<F::C>,
	pub current_state: F::CS,
	//pub states: &'a mut F::SS
}

impl<'a, F: Fsm + 'a> EventContext<'a, F> {
	pub async fn enqueue_event(&self, event: F::E) {
		self
			.queue
			.write()
			.await
			.enqueue_event(event);
	}
}

pub trait FsmEventQueue<F: Fsm>: Send + Sync {
	fn enqueue_event(&mut self, event: F::E);
	fn dequeue_event(&mut self) -> Option<F::E>;
	fn len(&self) -> usize;
}

pub trait FsmRetrieveState<S> {
	fn get_state(&self) -> &S;
	fn get_state_mut(&mut self) -> &mut S;
}

pub struct FsmEventQueueVec<F: Fsm> {
	queue: Vec<F::E>
}

impl<F: Fsm> FsmEventQueueVec<F> {
	pub fn new() -> Self {
		FsmEventQueueVec {
			queue: Vec::new()
		}
	}
}

impl<F: Fsm> FsmEventQueue<F> for FsmEventQueueVec<F> {
	fn enqueue_event(&mut self, event: F::E) {
		self.queue.push(event);
	}

	fn dequeue_event(&mut self) -> Option<F::E> {
		if self.queue.len() > 0 {
			Some(self.queue.remove(0))
		} else {
			None
		}
	}

	fn len(&self) -> usize {
		self.queue.len()
	}
}

pub type FsmArc<T> = Arc<RwLock<T>>;

pub async fn fsm_read_state<S: Copy>(state: &FsmArc<S>) -> S {
	let state = state.read().await;
	*state
}

#[async_trait]
pub trait Fsm where Self: Sized {
	type E: FsmEvents<Self>;
	type S: Send + Sync;
	type C: Send + Sync;
	type CS: Debug + Send + Sync;
	type SS: Send + Sync;

	fn new(context: &FsmArc<Self::C>) -> Self;

	async fn start(&self);
	async fn stop(&self);

	async fn call_on_entry(&self, state: Self::S) -> FsmTransitionResult<()>;
	async fn call_on_exit(&self, state: Self::S) -> FsmTransitionResult<()>;

	fn get_queue(&self) -> &FsmArc<dyn FsmEventQueue<Self>>;

	async fn get_current_state(&self) -> Self::CS;

	fn get_states(&self) -> &Self::SS;

	async fn process_anonymous_transitions(&self) -> Result<(), FsmError> {
		loop {
			match self.process_event(Self::E::new_no_event()).await {
				Ok(_) => { continue; }
				Err(_) => {
					break;
				}
			}
		}

		Ok(())
	}

	async fn process_event(&self, event: Self::E) -> Result<(), FsmError>;

	async fn execute_queued_events(&self) -> FsmQueueStatus {
		{
			let queue = self.get_queue();
			if queue.read().await.len() == 0 { return FsmQueueStatus::Empty; }
		}

		loop {
			let l = self.execute_single_queued_event().await;
			if l == FsmQueueStatus::Empty { break; }
		}

		FsmQueueStatus::Empty
	}

	async fn execute_single_queued_event(&self) -> FsmQueueStatus {
		{
			let ev;

			{
				let queue = self.get_queue();
				let mut queue = queue.write().await;
				ev = queue.dequeue_event();
			}

			if let Some(e) = ev {
				self.process_event(e).await.unwrap(); // should this somehow bubble?
			}
		}

		{
			let queue = self.get_queue();
			if queue.read().await.len() == 0 { FsmQueueStatus::Empty } else { FsmQueueStatus::MoreEventsQueued }
		}
	}

	async fn get_message_queue_size(&self) -> usize {
		self.get_queue().read().await.len()
	}
}


// codegen types

pub struct InitialState<F: Fsm, S: FsmState<F>>(PhantomData<F>, S);
pub struct ErrorState<F: Fsm, S: FsmState<F>>(PhantomData<F>, S);
pub struct ContextType<T>(T);
pub struct InspectionType<F: Fsm, T: FsmInspect<F>>(PhantomData<F>, T);
pub struct SubMachine<F: Fsm>(F);
pub struct ShallowHistory<F: Fsm, E: FsmEvent, StateTarget: FsmState<F> + Fsm>(PhantomData<F>, E, StateTarget);
pub struct InterruptState<F: Fsm, S: FsmState<F>, E: FsmEvent>(PhantomData<F>, S, E);
pub struct CopyableEvents;


pub struct Transition<F: Fsm, StateSource: FsmState<F>, E: FsmEvent, StateTarget: FsmState<F>, A: FsmAction<F, StateSource, StateTarget>>(PhantomData<F>, StateSource, E, StateTarget, A);
pub struct TransitionSelf<F: Fsm, State: FsmState<F>, E: FsmEvent, A: FsmActionSelf<F, State>>(PhantomData<F>, State, E, A);
pub struct TransitionInternal<F: Fsm, State: FsmState<F>, E: FsmEvent, A: FsmActionSelf<F, State>>(PhantomData<F>, State, E, A);

pub struct TransitionGuard<F: Fsm, StateSource: FsmState<F>, E: FsmEvent, StateTarget: FsmState<F>, A: FsmAction<F, StateSource, StateTarget>, G: FsmGuard<F>>(PhantomData<F>, StateSource, E, StateTarget, A, G);
pub struct TransitionSelfGuard<F: Fsm, State: FsmState<F>, E: FsmEvent, A: FsmActionSelf<F, State>, G: FsmGuard<F>>(PhantomData<F>, State, E, A, G);
pub struct TransitionInternalGuard<F: Fsm, State: FsmState<F>, E: FsmEvent, A: FsmActionSelf<F, State>, G: FsmGuard<F>>(PhantomData<F>, State, E, A, G);
