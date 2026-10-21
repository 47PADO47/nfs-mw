//! The four speech flows the machine runs, kept together.

use super::observer::Observer;
use super::pursuit_flow::PursuitFlow;
use super::roadblock_flow::RoadblockFlow;
use super::strategy_flow::StrategyFlow;

#[derive(Debug, Clone, Default)]
pub struct Flows {
    pub pursuit: PursuitFlow,
    pub strategy: StrategyFlow,
    pub roadblock: RoadblockFlow,
    pub observer: Observer,
}

impl Flows {
    /// Every flow back to its start (a pursuit reset).
    pub fn reset(&mut self) {
        self.pursuit.reset();
        self.strategy.reset();
        self.roadblock.reset();
        self.observer.reset();
    }
}
