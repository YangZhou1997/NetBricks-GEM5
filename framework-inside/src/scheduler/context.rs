use allocators::CacheAligned;
use common::*;
// use config::NetBricksConfiguration;
use failure::Fail;
// use interface::dpdk::{init_system, init_thread};
// use interface::{PmdPort, PortQueue, VirtualPort, VirtualQueue};
use interface::{SimulatePort, SimulateQueue};
use scheduler::*;
use std::collections::HashSet;
use std::sync::Arc;

// type AlignedPortQueue = CacheAligned<PortQueue>;
// type AlignedVirtualQueue = CacheAligned<VirtualQueue>;
type AlignedSimulateQueue = CacheAligned<SimulateQueue>;

#[derive(Debug, Fail)]
#[fail(display = "Port configuration error: {}", _0)]
pub struct PortError(String);

/// `NetBricksContext` contains handles to all schedulers, and provides mechanisms for coordination.
#[derive(Default)]
pub struct NetBricksContext {
    pub ports: Vec<Arc<SimulatePort>>,
    pub rx_queues: Vec<CacheAligned<SimulateQueue>>,
    pub active_cores: Vec<i32>,
}

impl NetBricksContext {

    /// Run a function (which installs a pipeline) on the first core in the system, blocking. 
    pub fn run<T>(&mut self, run: Arc<T>, npkts: u64)
    where
        T: Fn(Vec<AlignedSimulateQueue>, &mut StandaloneScheduler) + Send + Sync + 'static,
    {
        let mut sched = StandaloneScheduler::new(npkts);
        let boxed_run = run.clone();
        let ports = self.rx_queues.clone();
        sched.run(Arc::new(move |s| {
            boxed_run(ports.clone(), s)
        }));
        sched.execute_loop();
    }
}

/// Initialize NetBricks, incl. handling of dpdk configuration, logging, general
/// setup.
///
/// Return a Context to Execute.
pub fn initialize_system() -> Result<NetBricksContext> {
    // init_system(configuration);
    let mut ctx: NetBricksContext = Default::default();
    match SimulatePort::new() {
        Ok(p) => {
            ctx.ports.push(p);
        }
        Err(e) => {
            return Err(PortError(format!(
                "Port {} could not be initialized {:?}",
                "SimulateQueue", e
            ))
            .into());
        }
    }

    let port_instance = &ctx.ports[0];

    let rx_q = 0;
    let rx_q = rx_q as i32;
    match port_instance.new_simulate_queue(rx_q) {
        Ok(q) => {
            ctx.rx_queues.push(q);
        }
        Err(e) => {
            return Err(PortError(format!(
                "Queue {} on port {} could not be \
                    initialized {:?}",
                rx_q, "SimulateQueue", e
            ))
            .into());
        }
    }
    ctx.active_cores.push(0);
    Ok(ctx)
}
