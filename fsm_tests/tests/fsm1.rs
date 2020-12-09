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

#[derive(Clone, PartialEq, Default)]
pub struct Initial {
	entry: usize,
	exit: usize
}
#[async_trait]
impl FsmState<FsmOne> for Initial {
	async fn on_entry(&mut self, event_context: &mut EventContext<'_, FsmOne>) {
		self.entry += 1;
	}

	async fn on_exit(&mut self, event_context: &mut EventContext<'_, FsmOne>) {
		self.exit += 1;
	}
}

#[derive(Clone, PartialEq, Default)]
pub struct State1 {
	entry: usize,
	exit: usize,
	internal_action: usize
}
#[async_trait]
impl FsmState<FsmOne> for State1  {
	async fn on_entry(&mut self, event_context: &mut EventContext<'_, FsmOne>) {
		println!("State1 Entry!");
		self.entry += 1;
	}

	async fn on_exit(&mut self, event_context: &mut EventContext<'_, FsmOne>) {
		println!("State1 Exit!");
		self.exit += 1;
	}
}

#[derive(Clone, PartialEq, Default)]
pub struct State2;
#[async_trait]
impl FsmState<FsmOne> for State2 {

}


// actions

pub struct InitAction;
impl FsmAction<FsmOne, Initial, State1> for InitAction {
	fn action(event_context: &mut EventContext<FsmOne>, source_state: &mut Initial, target_state: &mut State1) {
		println!("Init action!");
	}
}

pub struct State1InternalAction;
impl FsmActionSelf<FsmOne, State1> for State1InternalAction {
	fn action(event_context: &mut EventContext<FsmOne>, state: &mut State1) {
		state.internal_action += 1;
	}
}

pub struct InternalTrigger;
impl FsmActionSelf<FsmOne, State1> for InternalTrigger {
	fn action(event_context: &mut EventContext<FsmOne>, state: &mut State1) {
		event_context.queue.enqueue_event(FsmOneEvents::Event2(Event2));
	}
}

#[derive(Default)]
pub struct FsmOneContext {
	guard1_exec: usize
}


#[derive(Fsm)]
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

	let mut fsm1 = FsmOne::new(Default::default());

	assert_eq!(fsm1.get_current_state(), FsmOneStates::Initial);
	{
		let initial: &Initial = fsm1.get_state();
		assert_eq!(initial.entry, 0);
		assert_eq!(initial.exit, 0);
	}

	fsm1.start().await;

	assert_eq!(fsm1.get_current_state(), FsmOneStates::State1);

	{
		let initial: &Initial = fsm1.get_state();
		assert_eq!(initial.entry, 1);
		assert_eq!(initial.exit, 1);

		let state1: &State1 = fsm1.get_state();
		assert_eq!(state1.entry, 1);
	}

	fsm1.process_event(FsmOneEvents::Event1(Event1)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(state1.exit, 1);
		assert_eq!(state1.entry, 2);
	}

	fsm1.process_event(FsmOneEvents::Event2(Event2)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(state1.exit, 1);
		assert_eq!(state1.entry, 2);

		assert_eq!(state1.internal_action, 1);
	}

	// event queueing, implicit and explicit execution
	fsm1.process_event(FsmOneEvents::Event3(Event3)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(state1.internal_action, 1);
	}

	fsm1.process_event(FsmOneEvents::Event3(Event3)).await.unwrap();

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(state1.internal_action, 2);
	}

	fsm1.execute_queued_events().await;

	{
		let state1: &State1 = fsm1.get_state();
		assert_eq!(state1.internal_action, 3);
	}

	// event guards
	assert_eq!(Err(FsmError::NoTransition), fsm1.process_event(FsmOneEvents::MagicEvent(MagicEvent(1))).await);
	assert_eq!(FsmOneStates::State1, fsm1.get_current_state());

	fsm1.process_event(FsmOneEvents::MagicEvent(MagicEvent(42))).await.unwrap();
	assert_eq!(FsmOneStates::State2, fsm1.get_current_state());


}