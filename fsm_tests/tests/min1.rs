extern crate fsm;
#[macro_use]
extern crate fsm_codegen;

use async_trait::async_trait;
use assert_matches::assert_matches;

use fsm::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StaticA;

#[async_trait]
impl FsmState<FsmMinOne> for StaticA {

}

#[derive(Fsm)]
#[allow(dead_code)]
struct FsmMinOneDefinition(
	InitialState<FsmMinOne, StaticA>
);


#[cfg(test)]
#[tokio::test]
async fn test_fsm_min1() {
    let fsm = FsmMinOne::new(&Default::default());
    fsm.start().await;
    assert_matches!(fsm.get_current_state().await, FsmMinOneStates::StaticA(_));
}