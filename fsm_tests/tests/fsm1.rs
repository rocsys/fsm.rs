extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use async_trait::async_trait;

use fsm::*;


// events

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Event1;
impl FsmEvent for Event1 {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Event2;
impl FsmEvent for Event2 {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Event3;
impl FsmEvent for Event3 {}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct MagicEvent(u32);
impl FsmEvent for MagicEvent {}

// guards

pub struct MagicGuard;
impl FsmGuard<FsmOne> for MagicGuard {
	fn guard(event_context: &EventContext<FsmOne>, _: &FsmOneStatesStore) -> bool {
		match event_context.event {
			&FsmOneEvents::MagicEvent(MagicEvent(n)) if n == 42 => {
				true
			},
			_ => false
		}
	}
}

// states

#[derive(Clone, Default)]
pub struct Initial {
	entry: FsmArc<usize>,
	exit: FsmArc<usize>
}
#[async_trait]
impl FsmState<FsmOne> for Initial {
	async fn on_entry(&self,_event_context: &EventContext<'_, FsmOne>) {
		*self.entry.write().await += 1;
	}

	async fn on_exit(&self,_event_context: &EventContext<'_, FsmOne>) {
		*self.exit.write().await += 1;
	}
}

#[derive(Clone, Default)]
pub struct State1 {
	entry: FsmArc<usize>,
	exit: FsmArc<usize>,
	internal_action: FsmArc<usize>
}
#[async_trait]
impl FsmState<FsmOne> for State1  {
	async fn on_entry(&self,_event_context: &EventContext<'_, FsmOne>) {
		println!("State1 Entry!");
		*self.entry.write().await += 1;
	}

	async fn on_exit(&self,_event_context: &EventContext<'_, FsmOne>) {
		println!("State1 Exit!");
		*self.exit.write().await += 1;
	}
}

#[derive(Clone, PartialEq, Default)]
pub struct State2;
#[async_trait]
impl FsmState<FsmOne> for State2 {

}


// actions

pub struct InitAction;
#[async_trait]
impl FsmAction<FsmOne, Initial, State1> for InitAction {
	async fn action(_event_context: &EventContext<'_, FsmOne>, _source_state: &Initial, _target_state: &State1) {
		println!("Init action!");
	}
}

pub struct State1InternalAction;
#[async_trait]
impl FsmActionSelf<FsmOne, State1> for State1InternalAction {
	async fn action(_event_context: &EventContext<'_, FsmOne>, state: &State1) {
		*state.internal_action.write().await += 1;
	}
}

pub struct InternalTrigger;
#[async_trait]
impl FsmActionSelf<FsmOne, State1> for InternalTrigger {
	async fn action(event_context: &EventContext<'_, FsmOne>, _state: &State1) {
		event_context.queue.write().await.enqueue_event(FsmOneEvents::Event2(Event2)).unwrap();
	}
}

#[derive(Default)]
pub struct FsmOneContext {
	_guard1_exec: usize
}


#[derive(Fsm)]
#[allow(dead_code)]
struct FsmOneDefinition(
	InitialState<FsmOne, Initial>,
	ContextType<FsmOneContext>,

	Transition        < FsmOne, Initial, NoEvent,    State1, InitAction >,
	Transition        < FsmOne, State1,  Event1,     State1, NoAction   >,
	TransitionInternal< FsmOne, State1,  Event2,             State1InternalAction>,
	TransitionInternal< FsmOne, State1,  Event3,             InternalTrigger>,

	TransitionGuard   < FsmOne, State1,  MagicEvent, State2, NoAction,               MagicGuard>,
);


#[cfg(test)]
#[tokio::test]
async fn test_machine1() {
	let fsm1 = FsmOne::new(&Default::default());

	assert_eq!(fsm1.get_current_state().await, FsmOneStates::Initial);
	{
		let initial: &Initial = fsm1.get_state();
		assert_eq!(*initial.entry.read().await, 0);
		assert_eq!(*initial.exit.read().await, 0);
	}

	fsm1.start().await;

	assert_eq!(fsm1.get_current_state().await, FsmOneStates::State1);

	{
		let initial: &Initial = fsm1.get_state();
		assert_eq!(*initial.entry.read().await, 1);
		assert_eq!(*initial.exit.read().await, 1);

		let state1: &State1 = fsm1.get_state();
		assert_eq!(*state1.entry.read().await, 1);
	}

	fsm1.process_event(FsmOneEvents::Event1(Event1)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(*state1.exit.read().await, 1);
		assert_eq!(*state1.entry.read().await, 2);
	}

	fsm1.process_event(FsmOneEvents::Event2(Event2)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(*state1.exit.read().await, 1);
		assert_eq!(*state1.entry.read().await, 2);

		assert_eq!(*state1.internal_action.read().await, 1);
	}

	// event queueing, implicit and explicit execution
	fsm1.process_event(FsmOneEvents::Event3(Event3)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(*state1.internal_action.read().await, 1);
	}

	fsm1.process_event(FsmOneEvents::Event3(Event3)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(*state1.internal_action.read().await, 2);
	}

	fsm1.execute_queued_events().await;

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(*state1.internal_action.read().await, 3);
	}

	// event guards
	assert_eq!(Err(FsmError::NoTransition), fsm1.process_event(FsmOneEvents::MagicEvent(MagicEvent(1))).await);
	assert_eq!(FsmOneStates::State1, fsm1.get_current_state().await);

	fsm1.process_event(FsmOneEvents::MagicEvent(MagicEvent(42))).await.unwrap();
	assert_eq!(FsmOneStates::State2, fsm1.get_current_state().await);


}