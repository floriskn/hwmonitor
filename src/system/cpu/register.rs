use crate::system::{
    cpu::{
        backends::backend_context::BackendContext, discover::discover_topology, vendor::vendor_for,
    },
    system::System,
};

pub fn register_all(system: &mut System) {
    let (cpus, threads) = discover_topology();

    let mut backends = BackendContext::new(threads);

    for cpu in &cpus {
        let vendor = vendor_for(system, cpu);

        vendor.register_cpu_sensors(system, cpu, &mut backends);

        for core in &cpu.cores {
            vendor.register_core_sensors(system, cpu, core, &mut backends);

            for thread in &core.threads {
                vendor.register_thread_sensors(system, cpu, Some(core), thread, &backends);
            }
        }
    }

    backends.register_all(system);
}
